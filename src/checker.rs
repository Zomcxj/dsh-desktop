use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

/// 通过 HTTP GET 确认 host:port 的 Web 服务已能响应。
pub fn http_ready(host: &str, port: u16, timeout: Duration) -> bool {
    let addr: SocketAddr = format!("{host}:{port}").parse().unwrap();
    let Ok(mut stream) = TcpStream::connect_timeout(&addr, timeout) else {
        return false;
    };
    if stream.set_read_timeout(Some(timeout)).is_err()
        || stream.set_write_timeout(Some(timeout)).is_err()
    {
        return false;
    }
    if stream
        .write_all(
            format!("GET / HTTP/1.1\r\nHost: {host}:{port}\r\nConnection: close\r\n\r\n")
                .as_bytes(),
        )
        .is_err()
    {
        return false;
    }

    let mut response = [0; 512];
    let Ok(count) = stream.read(&mut response) else {
        return false;
    };
    std::str::from_utf8(&response[..count])
        .ok()
        .and_then(|response| response.lines().next())
        .is_some_and(|status| status.starts_with("HTTP/"))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionCookie {
    pub name: String,
    pub value: String,
}

/// GET token URL in the background and capture the HttpOnly session cookie.
/// Returns the cookie only when dsh replies with 303 + Set-Cookie.
pub fn exchange_auth_cookie(url: &str, timeout: Duration) -> Option<SessionCookie> {
    let (host, port, path) = parse_local_http_url(url)?;
    let addr: SocketAddr = format!("{host}:{port}").parse().ok()?;
    let mut stream = TcpStream::connect_timeout(&addr, timeout).ok()?;
    stream.set_read_timeout(Some(timeout)).ok()?;
    stream.set_write_timeout(Some(timeout)).ok()?;
    stream
        .write_all(
            format!("GET {path} HTTP/1.1\r\nHost: {host}:{port}\r\nConnection: close\r\n\r\n")
                .as_bytes(),
        )
        .ok()?;

    let mut response = Vec::new();
    let mut buffer = [0; 1024];
    loop {
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(count) => {
                response.extend_from_slice(&buffer[..count]);
                if response.len() > 16 * 1024 {
                    break;
                }
            }
            Err(_) => return None,
        }
    }

    parse_session_cookie(std::str::from_utf8(&response).ok()?)
}

fn parse_local_http_url(url: &str) -> Option<(String, u16, String)> {
    let rest = url.strip_prefix("http://")?;
    let (authority, path) = rest.split_once('/').unwrap_or((rest, ""));
    let (host, port) = match authority.split_once(':') {
        Some((host, port)) => (host.to_string(), port.parse().ok()?),
        None => (authority.to_string(), 80),
    };
    if host.is_empty() {
        return None;
    }
    Some((host, port, format!("/{path}")))
}

fn parse_session_cookie(response: &str) -> Option<SessionCookie> {
    let mut lines = response.split("
");
    let status = lines.next()?;
    if !status.starts_with("HTTP/1.1 303") && !status.starts_with("HTTP/1.0 303") {
        return None;
    }
    for line in lines {
        if line.is_empty() {
            break;
        }
        let (name, value) = line.split_once(':')?;
        if !name.eq_ignore_ascii_case("set-cookie") {
            continue;
        }
        let cookie = value.trim();
        let pair = cookie.split(';').next()?.trim();
        let (cookie_name, cookie_value) = pair.split_once('=')?;
        if cookie_name.is_empty() || cookie_value.is_empty() {
            continue;
        }
        return Some(SessionCookie {
            name: cookie_name.to_string(),
            value: cookie_value.to_string(),
        });
    }
    None
}

#[cfg(windows)]
pub fn close_tcp_listeners(port: u16) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let output = Command::new("netstat")
        .args(["-ano", "-p", "tcp"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|error| format!("读取 TCP 监听端口失败: {error}"))?;
    if !output.status.success() {
        return Err("读取 TCP 监听端口失败".into());
    }

    for pid in listening_pids_from_netstat(
        &String::from_utf8_lossy(&output.stdout),
        port,
        std::process::id(),
    ) {
        let status = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW)
            .status()
            .map_err(|error| format!("关闭占用端口 {port} 的进程 {pid} 失败: {error}"))?;
        if !status.success() {
            return Err(format!("关闭占用端口 {port} 的进程 {pid} 失败"));
        }
    }
    Ok(())
}

pub fn listening_pids_from_netstat(output: &str, port: u16, self_pid: u32) -> Vec<u32> {
    let mut pids = Vec::new();
    for line in output.lines() {
        let fields: Vec<_> = line.split_whitespace().collect();
        if fields.len() < 5 || !fields[0].eq_ignore_ascii_case("TCP") || fields[3] != "LISTENING" {
            continue;
        }
        let Some(local_port) = fields[1]
            .rsplit_once(':')
            .and_then(|(_, port)| port.parse::<u16>().ok())
        else {
            continue;
        };
        let Ok(pid) = fields[4].parse::<u32>() else {
            continue;
        };
        if local_port == port && pid != 0 && pid != self_pid && !pids.contains(&pid) {
            pids.push(pid);
        }
    }
    pids
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn http_ready_requires_an_http_response() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0; 128];
            let count = stream.read(&mut request).unwrap();
            assert!(std::str::from_utf8(&request[..count])
                .unwrap()
                .starts_with("GET / HTTP/1.1\r\n"));
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
                .unwrap();
        });

        assert!(http_ready("127.0.0.1", port, Duration::from_millis(500)));
        server.join().unwrap();
    }

    #[test]
    fn http_ready_rejects_tcp_only_listener() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = thread::spawn(move || {
            let (_stream, _) = listener.accept().unwrap();
        });

        assert!(!http_ready("127.0.0.1", port, Duration::from_millis(100)));
        server.join().unwrap();
    }

    #[test]
    fn http_ready_returns_false_when_port_is_closed() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        assert!(!http_ready("127.0.0.1", port, Duration::from_millis(300)));
    }

    #[test]
    fn listening_pids_selects_only_valid_non_self_3080_listeners() {
        let output = "\
  TCP    127.0.0.1:3080       0.0.0.0:0              LISTENING       4123\n\
  TCP    [::1]:3080           [::]:0                 LISTENING       5678\n\
  TCP    0.0.0.0:3080         0.0.0.0:0              LISTENING       9000\n\
  TCP    127.0.0.1:3081       0.0.0.0:0              LISTENING       6789\n\
  TCP    127.0.0.1:3080       127.0.0.1:60000        ESTABLISHED     7000\n\
  TCP    127.0.0.1:3080       0.0.0.0:0              LISTENING       9000\n\
  TCP    127.0.0.1:3080       0.0.0.0:0              LISTENING       0\n\
  TCP    127.0.0.1:3080       0.0.0.0:0              LISTENING       invalid\n";

        assert_eq!(
            listening_pids_from_netstat(output, 3080, 5678),
            vec![4123, 9000]
        );
    }

    #[test]
    fn parse_local_http_url_keeps_token_query() {
        assert_eq!(
            parse_local_http_url("http://127.0.0.1:3080/?token=abc"),
            Some(("127.0.0.1".into(), 3080, "/?token=abc".into()))
        );
    }

    #[test]
    fn parse_session_cookie_requires_303_and_set_cookie() {
        let response = "HTTP/1.1 303 See Other
set-cookie: dsh.abc=v1.payload.sig; Max-Age=86400; Path=/; HttpOnly; SameSite=Strict
location: /

";
        assert_eq!(
            parse_session_cookie(response),
            Some(SessionCookie {
                name: "dsh.abc".into(),
                value: "v1.payload.sig".into(),
            })
        );
        assert!(parse_session_cookie("HTTP/1.1 401 Unauthorized

").is_none());
    }

    #[test]
    fn exchange_auth_cookie_captures_set_cookie_from_303() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0; 256];
            let count = stream.read(&mut request).unwrap();
            let text = std::str::from_utf8(&request[..count]).unwrap();
            assert!(text.starts_with("GET /?token=abc HTTP/1.1
"));
            stream
                .write_all(
                    b"HTTP/1.1 303 See Other
set-cookie: dsh.abc=v1.payload.sig; Path=/; HttpOnly
location: /

",
                )
                .unwrap();
        });

        assert_eq!(
            exchange_auth_cookie(
                &format!("http://127.0.0.1:{port}/?token=abc"),
                Duration::from_millis(500)
            ),
            Some(SessionCookie {
                name: "dsh.abc".into(),
                value: "v1.payload.sig".into(),
            })
        );
        server.join().unwrap();
    }
}
