use std::process::{Command, Stdio};

#[test]
#[ignore]
fn headless_live_slack_smoke_connects() {
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
        writeln!(stdin, "slack_auth_test").unwrap();
        writeln!(stdin, "slack_socket_mode_smoke 5000").unwrap();
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
    assert!(
        stdout.contains("\"event\":\"slack_auth_test\""),
        "missing slack_auth_test marker. stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("\"event\":\"slack_socket_mode_smoke\""),
        "missing slack_socket_mode_smoke marker. stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("\"event\":\"quit\""),
        "missing quit marker. stdout:\n{stdout}"
    );
}
