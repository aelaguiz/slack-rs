use std::process::{Command, Stdio};

#[test]
#[ignore]
fn headless_live_slack_socket_mode_listen_sees_hello() {
    if std::env::var("SLACK_LIVE_TEST").ok().as_deref() != Some("1") {
        panic!("Set SLACK_LIVE_TEST=1 to run live Slack integration tests");
    }

    let exe = env!("CARGO_BIN_EXE_slack-rs");

    let mut child = Command::new(exe)
        .args(["--headless", "--script", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn slack-rs headless");

    {
        use std::io::Write as _;
        let mut stdin = child.stdin.take().expect("child stdin");
        writeln!(stdin, "slack_socket_mode_listen 5000").unwrap();
        writeln!(stdin, "quit").unwrap();
    }

    let out = child
        .wait_with_output()
        .expect("wait for slack-rs headless");

    assert!(
        out.status.success(),
        "live Slack headless run failed.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed = stdout
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .collect::<Vec<_>>();

    let listen_event = parsed
        .iter()
        .find(|v| v.get("event").and_then(|e| e.as_str()) == Some("slack_socket_mode_listen"))
        .expect("missing slack_socket_mode_listen event");

    let saw_hello = listen_event
        .get("saw_hello")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    assert!(
        saw_hello,
        "expected saw_hello=true; listen_event={listen_event}"
    );
}
