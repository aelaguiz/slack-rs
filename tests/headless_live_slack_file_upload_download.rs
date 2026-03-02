use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
#[ignore]
fn headless_live_slack_can_upload_and_download_a_file() {
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
             Set SLACK_LIVE_TEST_ALLOW_WRITES=1 to explicitly allow test files to be uploaded."
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

    let temp = std::env::temp_dir().join(format!("slack-rs-upload-{nonce}"));
    std::fs::create_dir_all(&temp).expect("create temp dir");

    let upload_path = temp.join("hello.txt");
    std::fs::write(&upload_path, format!("hello from slack-rs nonce={nonce}\n"))
        .expect("write upload file");

    let download_path = temp.join("downloaded.txt");

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
        writeln!(stdin, "upload {}", upload_path.display()).unwrap();
        writeln!(stdin, "download {}", download_path.display()).unwrap();
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

    assert!(
        download_path.exists(),
        "expected downloaded file to exist: {}",
        download_path.display()
    );
    let uploaded = std::fs::read(&upload_path).expect("read uploaded file");
    let downloaded = std::fs::read(&download_path).expect("read downloaded file");
    assert_eq!(
        downloaded, uploaded,
        "downloaded bytes did not match uploaded bytes"
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed = stdout
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .collect::<Vec<_>>();

    let state_after_upload = parsed
        .iter()
        .find(|v| {
            v.get("event").and_then(|e| e.as_str()) == Some("state")
                && v.get("line").and_then(|n| n.as_u64()) == Some(2)
        })
        .expect("missing state JSON after `upload`");

    let selected_files = state_after_upload
        .get("focused_selected_files")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    assert!(
        !selected_files.is_empty(),
        "expected selected message to include at least one file after upload; state_after_upload={state_after_upload}"
    );

    let state_after_download = parsed
        .iter()
        .find(|v| {
            v.get("event").and_then(|e| e.as_str()) == Some("state")
                && v.get("line").and_then(|n| n.as_u64()) == Some(3)
        })
        .expect("missing state JSON after `download`");

    let expected_path = download_path.display().to_string();
    assert_eq!(
        state_after_download
            .get("last_downloaded_path")
            .and_then(|v| v.as_str()),
        Some(expected_path.as_str()),
        "expected last_downloaded_path to match"
    );
    let bytes = state_after_download
        .get("last_downloaded_bytes")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    assert!(
        bytes > 0,
        "expected last_downloaded_bytes > 0; state_after_download={state_after_download}"
    );
}
