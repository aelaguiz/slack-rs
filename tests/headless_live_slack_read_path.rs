use std::process::{Command, Stdio};

#[test]
#[ignore]
fn headless_live_slack_can_load_history_for_known_conversation() {
    if std::env::var("SLACK_LIVE_TEST").ok().as_deref() != Some("1") {
        panic!("Set SLACK_LIVE_TEST=1 to run live Slack integration tests");
    }

    let conv_id = std::env::var("SLACK_TEST_CONVERSATION_ID").unwrap_or_else(|_| {
        panic!(
            "Set SLACK_TEST_CONVERSATION_ID to a channel/DM id the bot token can read (e.g. C123... or D123...)."
        )
    });

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
        writeln!(stdin, "sidebar_refresh").unwrap();
        writeln!(stdin, "open {conv_id}").unwrap();
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

    // After line 2 ("open ...") we expect the timeline cache to have at least one message.
    let open_state = parsed
        .iter()
        .find(|v| {
            v.get("event").and_then(|e| e.as_str()) == Some("state")
                && v.get("line").and_then(|n| n.as_u64()) == Some(2)
        })
        .expect("missing state JSON after `open`");

    let msg_count = open_state
        .get("focused_timeline_messages")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    assert!(
        msg_count > 0,
        "expected >0 messages after open; got {msg_count}. open_state={open_state}"
    );
}
