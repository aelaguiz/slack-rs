use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
#[ignore]
fn headless_live_slack_can_send_message_via_composer() {
    if std::env::var("SLACK_LIVE_TEST").ok().as_deref() != Some("1") {
        panic!("Set SLACK_LIVE_TEST=1 to run live Slack integration tests");
    }
    if std::env::var("SLACK_LIVE_TEST_ALLOW_WRITES")
        .ok()
        .as_deref()
        != Some("1")
    {
        panic!(
            "This test writes to a real Slack workspace.\n\
             Set SLACK_LIVE_TEST_ALLOW_WRITES=1 to explicitly allow test messages to be posted."
        );
    }

    let conv_id = std::env::var("SLACK_TEST_CONVERSATION_ID").unwrap_or_else(|_| {
        panic!(
            "Set SLACK_TEST_CONVERSATION_ID to a channel/DM id the bot token can write (e.g. C123... or D123...)."
        )
    });

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_millis();
    let msg = format!("[slack-rs test] headless send nonce={nonce}");

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
        writeln!(stdin, "open {conv_id}").unwrap();
        writeln!(stdin, "compose").unwrap();
        writeln!(stdin, "type {msg}").unwrap();
        writeln!(stdin, "send").unwrap();
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

    let state_after_open = parsed
        .iter()
        .find(|v| {
            v.get("event").and_then(|e| e.as_str()) == Some("state")
                && v.get("line").and_then(|n| n.as_u64()) == Some(1)
        })
        .expect("missing state JSON after `open`");

    let state_after_send = parsed
        .iter()
        .find(|v| {
            v.get("event").and_then(|e| e.as_str()) == Some("state")
                && v.get("line").and_then(|n| n.as_u64()) == Some(4)
        })
        .expect("missing state JSON after `send`");

    let open_count = state_after_open
        .get("focused_timeline_messages")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let after_send_count = state_after_send
        .get("focused_timeline_messages")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    assert_eq!(
        after_send_count,
        open_count.saturating_add(1),
        "expected send to insert exactly one new message; before={open_count} after={after_send_count} state_after_open={state_after_open} state_after_send={state_after_send}"
    );
}
