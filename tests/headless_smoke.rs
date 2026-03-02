use std::process::{Command, Stdio};

#[test]
fn headless_smoke_quits_cleanly() {
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
        // Keep it minimal: exercise a couple of pane ops, then quit.
        writeln!(stdin, "split_vertical").unwrap();
        writeln!(stdin, "focus_right").unwrap();
        writeln!(stdin, "quit").unwrap();
    }

    let out = child
        .wait_with_output()
        .expect("wait for slack-rs headless");

    assert!(
        out.status.success(),
        "headless run failed.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("\"event\":\"started\""),
        "missing started marker. stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("\"event\":\"quit\""),
        "missing quit marker. stdout:\n{stdout}"
    );
}
