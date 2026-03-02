use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
#[ignore]
fn headless_live_slack_can_toggle_reaction_on_selected_message() {
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
             Set SLACK_LIVE_TEST_ALLOW_WRITES=1 to explicitly allow test messages/reactions."
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
    let msg = format!("[slack-rs test] headless react nonce={nonce}");

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
        writeln!(stdin, "timeline_select_last").unwrap();
        writeln!(stdin, "react eyes").unwrap();
        writeln!(stdin, "react eyes").unwrap();
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

    let state_after_add = parsed
        .iter()
        .find(|v| {
            v.get("event").and_then(|e| e.as_str()) == Some("state")
                && v.get("line").and_then(|n| n.as_u64()) == Some(6)
        })
        .expect("missing state JSON after `react eyes` (add)");

    let state_after_remove = parsed
        .iter()
        .find(|v| {
            v.get("event").and_then(|e| e.as_str()) == Some("state")
                && v.get("line").and_then(|n| n.as_u64()) == Some(7)
        })
        .expect("missing state JSON after `react eyes` (remove)");

    let reactions_after_add = state_after_add
        .get("focused_selected_reactions")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let eyes_after_add = reactions_after_add.iter().find(|r| {
        r.get("name").and_then(|n| n.as_str()) == Some("eyes")
            && r.get("me").and_then(|m| m.as_bool()) == Some(true)
    });

    assert!(
        eyes_after_add.is_some(),
        "expected :eyes: to be present (me=true) after add; state_after_add={state_after_add}"
    );

    let reactions_after_remove = state_after_remove
        .get("focused_selected_reactions")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let eyes_after_remove = reactions_after_remove
        .iter()
        .find(|r| r.get("name").and_then(|n| n.as_str()) == Some("eyes"));

    assert!(
        eyes_after_remove.is_none(),
        "expected :eyes: to be removed after second toggle; state_after_remove={state_after_remove}"
    );
}
