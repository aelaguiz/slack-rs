use std::path::Path;
use std::time::Duration;

use anyhow::Context as _;

use crate::app::action::Action;
use crate::model::{ConversationId, MessageTs};
use crate::slack::service::SlackService;

/// Default timeout for a single Slack Web API effect.
///
/// Fail-fast posture: a hung HTTP call should surface as a fatal error rather than stalling the UI
/// or a headless automation run indefinitely.
pub const DEFAULT_SLACK_EFFECT_TIMEOUT: Duration = Duration::from_secs(15);

pub fn timeout_for_effect(effect: &Effect) -> Duration {
    match effect {
        Effect::SlackUploadFile { .. } | Effect::SlackDownloadFile { .. } => {
            Duration::from_secs(60)
        }
        _ => DEFAULT_SLACK_EFFECT_TIMEOUT,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    SlackLoadSidebar,
    SlackLoadHistory {
        conversation: ConversationId,
        cursor: Option<String>,
        limit: u16,
        append: bool,
    },
    SlackLoadThreadReplies {
        conversation: ConversationId,
        thread_ts: MessageTs,
        cursor: Option<String>,
        limit: u16,
        append: bool,
    },
    SlackAddReaction {
        conversation: ConversationId,
        ts: MessageTs,
        emoji: String,
    },
    SlackRemoveReaction {
        conversation: ConversationId,
        ts: MessageTs,
        emoji: String,
    },
    SlackSendMessage {
        conversation: ConversationId,
        text: String,
        thread_ts: Option<MessageTs>,
    },

    SlackUploadFile {
        conversation: ConversationId,
        thread_ts: Option<MessageTs>,
        path: String,
    },
    SlackDownloadFile {
        url: String,
        dest_path: String,
    },
    SlackLoadFileInfo {
        file_id: String,
    },
}

pub async fn execute(effect: Effect, slack: SlackService) -> anyhow::Result<Vec<Action>> {
    match effect {
        Effect::SlackLoadSidebar => {
            let sidebar = slack
                .list_sidebar_conversations()
                .await
                .context("load sidebar conversations from Slack")?;
            Ok(vec![Action::SidebarLoaded {
                channels: sidebar.channels,
                dms: sidebar.dms,
            }])
        }
        Effect::SlackLoadHistory {
            conversation,
            cursor,
            limit,
            append,
        } => {
            let page = slack
                .fetch_conversation_history(&conversation, cursor.as_deref(), limit)
                .await
                .with_context(|| {
                    format!("fetch Slack history for conversation {}", conversation)
                })?;

            Ok(vec![Action::TimelineLoaded {
                conversation,
                messages: page.messages,
                next_cursor: page.next_cursor,
                append,
            }])
        }
        Effect::SlackLoadThreadReplies {
            conversation,
            thread_ts,
            cursor,
            limit,
            append,
        } => {
            let page = slack
                .fetch_thread_replies(&conversation, &thread_ts, cursor.as_deref(), limit)
                .await
                .with_context(|| {
                    format!(
                        "fetch Slack thread replies for conversation {} thread_ts {}",
                        conversation, thread_ts
                    )
                })?;

            Ok(vec![Action::ThreadLoaded {
                conversation,
                thread_ts,
                messages: page.messages,
                next_cursor: page.next_cursor,
                append,
            }])
        }
        Effect::SlackSendMessage {
            conversation,
            text,
            thread_ts,
        } => {
            let message = slack
                .send_message(&conversation, &text, thread_ts.as_ref())
                .await
                .with_context(|| format!("Slack send message to {}", conversation))?;

            Ok(vec![Action::SlackMessageReceived {
                conversation,
                message,
            }])
        }
        Effect::SlackAddReaction {
            conversation,
            ts,
            emoji,
        } => {
            slack
                .add_reaction(&conversation, &ts, &emoji)
                .await
                .with_context(|| format!("Slack add reaction :{}: on {}", emoji, conversation))?;

            Ok(vec![Action::ReactionChanged {
                conversation,
                ts,
                emoji,
                delta: 1,
                me: Some(true),
            }])
        }
        Effect::SlackRemoveReaction {
            conversation,
            ts,
            emoji,
        } => {
            slack
                .remove_reaction(&conversation, &ts, &emoji)
                .await
                .with_context(|| {
                    format!("Slack remove reaction :{}: on {}", emoji, conversation)
                })?;

            Ok(vec![Action::ReactionChanged {
                conversation,
                ts,
                emoji,
                delta: -1,
                me: Some(false),
            }])
        }
        Effect::SlackUploadFile {
            conversation,
            thread_ts,
            path,
        } => {
            let uploaded = slack
                .upload_file_external(&conversation, thread_ts.as_ref(), Path::new(&path))
                .await
                .with_context(|| format!("Slack upload file {} to {}", path, conversation))?;

            Ok(vec![Action::FileUploadCompleted {
                conversation,
                thread_ts,
                files: uploaded,
            }])
        }
        Effect::SlackDownloadFile { url, dest_path } => {
            let bytes = slack
                .download_url_private_to_path(&url, Path::new(&dest_path))
                .await
                .with_context(|| format!("Slack download file {} -> {}", url, dest_path))?;

            Ok(vec![Action::FileDownloaded { dest_path, bytes }])
        }
        Effect::SlackLoadFileInfo { file_id } => {
            let info = slack
                .fetch_file_info_summary(&file_id)
                .await
                .with_context(|| format!("Slack files.info {}", file_id))?;

            Ok(vec![Action::FileInfoLoaded {
                file_id: info.file_id,
                size: info.size,
                mode: info.mode,
                preview_plain_text: info.preview_plain_text,
            }])
        }
    }
}
