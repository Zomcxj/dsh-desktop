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

#[cfg(windows)]
pub fn close_tcp_listeners(port: u16) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let output = Command::new("netstat")
        .args(["-ano", "-p", "tcp"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
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
}
