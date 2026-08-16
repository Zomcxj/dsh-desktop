use std::process::{Child, Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

pub fn output(command: &mut Command, timeout: Duration) -> Option<Output> {
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    wait(&mut child, timeout)
        .then(|| child.wait_with_output().ok())
        .flatten()
}

pub fn status(command: &mut Command, timeout: Duration) -> bool {
    let Ok(mut child) = command
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    else {
        return false;
    };
    wait(&mut child, timeout)
        && child
            .wait()
            .is_ok_and(|status| status.success())
}

fn wait(child: &mut Child, timeout: Duration) -> bool {
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status.success(),
            Ok(None) if started.elapsed() < timeout => thread::sleep(Duration::from_millis(50)),
            _ => {
                kill_tree(child);
                let _ = child.wait();
                return false;
            }
        }
    }
}

fn kill_tree(child: &mut Child) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let _ = Command::new("taskkill")
            .args(["/PID", &child.id().to_string(), "/T", "/F"])
            .creation_flags(0x0800_0000)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    #[cfg(not(windows))]
    let _ = child.kill();
}
