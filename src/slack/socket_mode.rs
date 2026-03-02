use std::time::Duration;

use anyhow::Context as _;
use futures_util::{SinkExt as _, StreamExt as _};
use serde_json::json;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

use crate::app::action::Action;
use crate::app::runtime::AppEvent;
use crate::slack::events;
use crate::slack::service::SlackService;

pub async fn run_socket_mode(
    service: SlackService,
    tx: mpsc::Sender<AppEvent>,
) -> anyhow::Result<()> {
    let url = service
        .socket_mode_url()
        .await
        .context("fetch Socket Mode websocket URL via apps.connections.open")?;

    let (ws_stream, _resp) = tokio_tungstenite::connect_async(&url)
        .await
        .context("connect Slack Socket Mode websocket")?;

    // "Connected" means the websocket is live. Slack will also send a `hello` message shortly.
    tx.try_send(AppEvent::SlackConnected).map_err(|err| {
        anyhow::anyhow!("failed to enqueue SlackConnected event to UI loop: {err:?}")
    })?;

    let (mut write, mut read) = ws_stream.split();

    while let Some(msg) = read.next().await {
        let msg = msg.context("read Slack Socket Mode websocket message")?;
        match msg {
            Message::Text(text) => {
                handle_socket_text(&mut write, &tx, &text).await?;
            }
            Message::Binary(bin) => {
                let text = String::from_utf8(bin.to_vec())
                    .context("Slack websocket sent non-UTF8 binary data")?;
                handle_socket_text(&mut write, &tx, &text).await?;
            }
            Message::Ping(payload) => {
                write
                    .send(Message::Pong(payload))
                    .await
                    .context("reply with websocket pong")?;
            }
            Message::Pong(_) => {}
            Message::Close(frame) => {
                anyhow::bail!("Slack Socket Mode websocket closed: {frame:?}");
            }
            Message::Frame(_) => {}
        }
    }

    anyhow::bail!("Slack Socket Mode websocket ended unexpectedly");
}

#[derive(Debug)]
pub struct ListenResult {
    pub saw_hello: bool,
    pub envelopes_seen: u64,
    pub actions: Vec<Action>,
}

/// Listen on Socket Mode for a bounded duration and return any internal actions produced.
///
/// This is used for headless automation to validate near real-time behavior without requiring a
/// long-running background task. Unknown event types are ignored, but disconnects / closes are
/// treated as fatal (fail-fast posture).
pub async fn listen_for_actions(
    service: SlackService,
    duration: Duration,
) -> anyhow::Result<ListenResult> {
    let url = service
        .socket_mode_url()
        .await
        .context("fetch Socket Mode websocket URL via apps.connections.open")?;

    let (ws_stream, _resp) = tokio_tungstenite::connect_async(&url)
        .await
        .context("connect Slack Socket Mode websocket")?;

    let (mut write, mut read) = ws_stream.split();

    let deadline = tokio::time::Instant::now() + duration;
    let mut saw_hello = false;
    let mut envelopes_seen = 0_u64;
    let mut actions = Vec::<Action>::new();

    loop {
        let now = tokio::time::Instant::now();
        if now >= deadline {
            break;
        }
        let remaining = deadline - now;

        let next = match tokio::time::timeout(remaining, read.next()).await {
            Ok(v) => v,
            Err(_) => break,
        };

        let msg = match next {
            Some(msg) => msg.context("read Slack Socket Mode websocket message")?,
            None => anyhow::bail!("Slack Socket Mode websocket ended unexpectedly while listening"),
        };

        match msg {
            Message::Text(text) => {
                handle_listen_text(
                    &mut write,
                    &text,
                    &mut saw_hello,
                    &mut envelopes_seen,
                    &mut actions,
                )
                .await?;
            }
            Message::Binary(bin) => {
                let text = String::from_utf8(bin.to_vec())
                    .context("Slack websocket sent non-UTF8 binary data")?;
                handle_listen_text(
                    &mut write,
                    &text,
                    &mut saw_hello,
                    &mut envelopes_seen,
                    &mut actions,
                )
                .await?;
            }
            Message::Ping(payload) => {
                write
                    .send(Message::Pong(payload))
                    .await
                    .context("reply with websocket pong")?;
            }
            Message::Pong(_) => {}
            Message::Close(frame) => {
                anyhow::bail!("Slack Socket Mode websocket closed while listening: {frame:?}");
            }
            Message::Frame(_) => {}
        }
    }

    Ok(ListenResult {
        saw_hello,
        envelopes_seen,
        actions,
    })
}

/// Minimal Socket Mode connectivity check for headless automation.
///
/// Behavior:
/// - Connects via `apps.connections.open`
/// - Waits for a `hello` message (bounded by `timeout`)
/// - ACKs any envelopes it sees (best effort)
///
/// Fail-fast: any disconnect/close/timeouts are surfaced as errors.
pub async fn smoke_connect(service: SlackService, timeout: Duration) -> anyhow::Result<()> {
    let url = service
        .socket_mode_url()
        .await
        .context("fetch Socket Mode websocket URL via apps.connections.open")?;

    let (ws_stream, _resp) = tokio_tungstenite::connect_async(&url)
        .await
        .context("connect Slack Socket Mode websocket")?;

    let (mut write, mut read) = ws_stream.split();

    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        let now = tokio::time::Instant::now();
        if now >= deadline {
            anyhow::bail!("timed out waiting for Slack Socket Mode `hello` after {timeout:?}");
        }
        let remaining = deadline - now;

        let next = match tokio::time::timeout(remaining, read.next()).await {
            Ok(v) => v,
            Err(_) => {
                anyhow::bail!("timed out waiting for Slack Socket Mode `hello` after {timeout:?}")
            }
        };

        let msg = match next {
            Some(msg) => msg.context("read Slack Socket Mode websocket message")?,
            None => anyhow::bail!("Slack Socket Mode websocket ended before `hello`"),
        };

        match msg {
            Message::Text(text) => {
                if handle_smoke_text(&mut write, &text).await? {
                    return Ok(());
                }
            }
            Message::Binary(bin) => {
                let text = String::from_utf8(bin.to_vec())
                    .context("Slack websocket sent non-UTF8 binary data")?;
                if handle_smoke_text(&mut write, &text).await? {
                    return Ok(());
                }
            }
            Message::Ping(payload) => {
                write
                    .send(Message::Pong(payload))
                    .await
                    .context("reply with websocket pong")?;
            }
            Message::Pong(_) => {}
            Message::Close(frame) => {
                anyhow::bail!("Slack Socket Mode websocket closed before `hello`: {frame:?}");
            }
            Message::Frame(_) => {}
        }
    }
}

async fn handle_smoke_text<W>(write: &mut W, text: &str) -> anyhow::Result<bool>
where
    W: futures_util::Sink<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
{
    // Parse as generic JSON so we can ACK quickly without committing to a full event model yet.
    let v: serde_json::Value = serde_json::from_str(text).context("parse Slack socket JSON")?;

    if let Some(ty) = v.get("type").and_then(|t| t.as_str()) {
        match ty {
            // Slack's "hello" indicates the Socket Mode session is established.
            "hello" => return Ok(true),
            // Slack may request clients disconnect (e.g. maintenance).
            "disconnect" => {
                let reason = v
                    .get("reason")
                    .and_then(|r| r.as_str())
                    .unwrap_or("<no reason provided>");
                anyhow::bail!("Slack socket_mode disconnect: {reason}");
            }
            _ => {}
        }
    }

    if let Some(envelope_id) = v.get("envelope_id").and_then(|e| e.as_str()) {
        // ACK must be fast.
        let ack = json!({ "envelope_id": envelope_id }).to_string();
        write
            .send(Message::Text(ack.into()))
            .await
            .context("send Socket Mode envelope ACK")?;
    }

    Ok(false)
}

async fn handle_socket_text<W>(
    write: &mut W,
    tx: &mpsc::Sender<AppEvent>,
    text: &str,
) -> anyhow::Result<()>
where
    W: futures_util::Sink<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
{
    // Parse as generic JSON so we can ACK quickly without committing to a full event model yet.
    let v: serde_json::Value = serde_json::from_str(text).context("parse Slack socket JSON")?;

    if let Some(ty) = v.get("type").and_then(|t| t.as_str()) {
        match ty {
            // Slack's "hello" indicates the Socket Mode session is established.
            "hello" => {
                tx.try_send(AppEvent::SlackConnected).map_err(|err| {
                    anyhow::anyhow!("failed to enqueue SlackConnected event to UI loop: {err:?}")
                })?;
            }
            // Slack may request clients disconnect (e.g. maintenance).
            "disconnect" => {
                let reason = v
                    .get("reason")
                    .and_then(|r| r.as_str())
                    .unwrap_or("<no reason provided>");
                anyhow::bail!("Slack socket_mode disconnect: {reason}");
            }
            _ => {}
        }
    }

    if let Some(envelope_id) = v.get("envelope_id").and_then(|e| e.as_str()) {
        // ACK must be fast; never await UI notification before ACKing.
        let ack = json!({ "envelope_id": envelope_id }).to_string();
        write
            .send(Message::Text(ack.into()))
            .await
            .context("send Socket Mode envelope ACK")?;

        tx.try_send(AppEvent::SlackEventReceived).map_err(|err| {
            anyhow::anyhow!("failed to enqueue SlackEventReceived event to UI loop: {err:?}")
        })?;

        let actions = events::actions_from_socket_envelope(&v)
            .context("translate Slack Socket Mode envelope into actions")?;
        for action in actions {
            // Fail-fast: do not silently drop live events.
            tx.try_send(AppEvent::Action(action)).map_err(|err| {
                anyhow::anyhow!("failed to enqueue Slack action to UI loop: {err:?}")
            })?;
        }
    }

    Ok(())
}

async fn handle_listen_text<W>(
    write: &mut W,
    text: &str,
    saw_hello: &mut bool,
    envelopes_seen: &mut u64,
    actions: &mut Vec<Action>,
) -> anyhow::Result<()>
where
    W: futures_util::Sink<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
{
    // Parse as generic JSON so we can ACK quickly without committing to a full event model yet.
    let v: serde_json::Value = serde_json::from_str(text).context("parse Slack socket JSON")?;

    if let Some(ty) = v.get("type").and_then(|t| t.as_str()) {
        match ty {
            "hello" => *saw_hello = true,
            "disconnect" => {
                let reason = v
                    .get("reason")
                    .and_then(|r| r.as_str())
                    .unwrap_or("<no reason provided>");
                anyhow::bail!("Slack socket_mode disconnect: {reason}");
            }
            _ => {}
        }
    }

    if let Some(envelope_id) = v.get("envelope_id").and_then(|e| e.as_str()) {
        // ACK must be fast.
        let ack = json!({ "envelope_id": envelope_id }).to_string();
        write
            .send(Message::Text(ack.into()))
            .await
            .context("send Socket Mode envelope ACK")?;

        *envelopes_seen = envelopes_seen.saturating_add(1);

        actions.extend(
            events::actions_from_socket_envelope(&v)
                .context("translate Slack Socket Mode envelope into actions")?,
        );
    }

    Ok(())
}
