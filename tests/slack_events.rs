use slack_rs::app::action::Action;
use slack_rs::slack::events;

#[test]
fn translates_basic_message_event() {
    let v = serde_json::json!({
        "envelope_id": "abc",
        "type": "events_api",
        "payload": {
            "event": {
                "type": "message",
                "channel": "C123",
                "user": "U234",
                "text": "hello",
                "ts": "123.456",
            }
        }
    });

    let actions = events::actions_from_socket_envelope(&v).unwrap();
    assert_eq!(actions.len(), 1);
    match &actions[0] {
        Action::SlackMessageReceived {
            conversation,
            message,
        } => {
            assert_eq!(conversation.as_str(), "C123");
            assert_eq!(message.ts.as_str(), "123.456");
            assert_eq!(message.user.as_ref().unwrap().as_str(), "U234");
            assert_eq!(message.text, "hello");
            assert_eq!(message.thread_ts, None);
            assert!(message.files.is_empty());
        }
        other => panic!("expected SlackMessageReceived, got {other:?}"),
    }
}

#[test]
fn ignores_message_subtypes_for_now() {
    let v = serde_json::json!({
        "envelope_id": "abc",
        "type": "events_api",
        "payload": {
            "event": {
                "type": "message",
                "subtype": "message_changed",
                "channel": "C123",
                "ts": "123.456",
            }
        }
    });

    let actions = events::actions_from_socket_envelope(&v).unwrap();
    assert!(actions.is_empty());
}

#[test]
fn translates_file_share_message_subtype() {
    let v = serde_json::json!({
        "envelope_id": "abc",
        "type": "events_api",
        "payload": {
            "event": {
                "type": "message",
                "subtype": "file_share",
                "channel": "C123",
                "user": "U234",
                "text": "uploaded a file",
                "ts": "123.456",
                "files": [
                    {
                        "id": "F111",
                        "name": "hello.txt",
                        "mimetype": "text/plain",
                        "size": 12,
                        "mode": "snippet",
                        "preview_plain_text": "hello\nworld\n"
                    }
                ]
            }
        }
    });

    let actions = events::actions_from_socket_envelope(&v).unwrap();
    assert_eq!(actions.len(), 1);
    match &actions[0] {
        Action::SlackMessageReceived { message, .. } => {
            assert_eq!(message.files.len(), 1);
            assert_eq!(message.files[0].id, "F111");
            assert_eq!(message.files[0].name.as_deref(), Some("hello.txt"));
            assert_eq!(message.files[0].size, Some(12));
            assert_eq!(message.files[0].mode.as_deref(), Some("snippet"));
            assert_eq!(
                message.files[0].snippet_preview.as_deref(),
                Some("hello\nworld\n")
            );
        }
        other => panic!("expected SlackMessageReceived, got {other:?}"),
    }
}
