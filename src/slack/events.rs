use anyhow::Context as _;

use crate::app::action::Action;
use crate::model::{ConversationId, Message, MessageFile, MessageTs, UserId};

/// Translate Slack Socket Mode envelopes into internal `Action`s.
///
/// This is intentionally minimal and "ignore-by-default":
/// - Unknown envelope types return `Ok(vec![])`
/// - Known types we handle must be parsed strictly (missing required fields is an error)
///
/// Rationale:
/// - We don't want to crash just because Slack sends an event type we don't care about yet.
/// - But if we *claim* to handle an event (e.g. a `message`), missing fields indicates a
///   parsing bug or unexpected API change, and we want to fail loudly.
pub fn actions_from_socket_envelope(v: &serde_json::Value) -> anyhow::Result<Vec<Action>> {
    let Some(envelope_type) = v.get("type").and_then(|t| t.as_str()) else {
        return Ok(vec![]);
    };

    if envelope_type != "events_api" {
        return Ok(vec![]);
    }

    let payload = match v.get("payload") {
        Some(p) => p,
        None => return Ok(vec![]),
    };

    let event = match payload.get("event") {
        Some(e) => e,
        None => return Ok(vec![]),
    };

    let Some(event_type) = event.get("type").and_then(|t| t.as_str()) else {
        return Ok(vec![]);
    };

    match event_type {
        "message" => actions_from_message_event(event),
        _ => Ok(vec![]),
    }
}

fn actions_from_message_event(event: &serde_json::Value) -> anyhow::Result<Vec<Action>> {
    // Ignore message subtypes for now (edits/deletes/bot_message/etc), except `file_share`
    // which is required for usable file parity.
    if let Some(subtype) = event.get("subtype").and_then(|s| s.as_str()) {
        if subtype != "file_share" {
            return Ok(vec![]);
        }
    }

    let channel = event
        .get("channel")
        .and_then(|c| c.as_str())
        .context("Slack message event missing `channel`")?;
    let ts = event
        .get("ts")
        .and_then(|t| t.as_str())
        .context("Slack message event missing `ts`")?;

    let user = event.get("user").and_then(|u| u.as_str()).map(UserId::from);
    let text = event
        .get("text")
        .and_then(|t| t.as_str())
        .unwrap_or_default()
        .to_string();
    let thread_ts = event
        .get("thread_ts")
        .and_then(|t| t.as_str())
        .map(MessageTs::from);

    let files = parse_message_files(event).context("parse Slack message files")?;

    let message = Message {
        ts: MessageTs::from(ts),
        user,
        text,
        thread_ts,
        reactions: Vec::new(),
        files,
    };

    Ok(vec![Action::SlackMessageReceived {
        conversation: ConversationId::from(channel),
        message,
    }])
}

fn parse_message_files(event: &serde_json::Value) -> anyhow::Result<Vec<MessageFile>> {
    let Some(files) = event.get("files") else {
        return Ok(Vec::new());
    };
    let Some(list) = files.as_array() else {
        return Ok(Vec::new());
    };

    let mut out = Vec::new();
    for f in list {
        let id = f
            .get("id")
            .and_then(|v| v.as_str())
            .context("Slack file missing `id`")?
            .to_string();

        out.push(MessageFile {
            id,
            name: f
                .get("name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            title: f
                .get("title")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            mimetype: f
                .get("mimetype")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            filetype: f
                .get("filetype")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            url_private_download: f
                .get("url_private_download")
                .and_then(|v| v.as_str())
                .or_else(|| f.get("url_private").and_then(|v| v.as_str()))
                .map(|s| s.to_string()),
            size: f.get("size").and_then(|v| v.as_u64()),
            mode: f
                .get("mode")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            snippet_preview: f
                .get("preview_plain_text")
                .and_then(|v| v.as_str())
                .or_else(|| f.get("preview").and_then(|v| v.as_str()))
                .map(|s| s.to_string()),
        });
    }

    Ok(out)
}
