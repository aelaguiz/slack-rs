use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;

use anyhow::Context as _;
use slack_morphism::prelude::*;

use crate::model::{
    ConversationId, ConversationKind, ConversationSummary, Message, MessageFile, MessageTs,
    Reaction, UserId,
};
use crate::slack::files;
use crate::slack::tokens::SlackTokens;

#[derive(Debug, Clone)]
pub struct AuthTestSummary {
    pub team: String,
    pub team_id: String,
    pub user_id: String,
    pub user: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SidebarConversations {
    pub channels: Vec<ConversationSummary>,
    pub dms: Vec<ConversationSummary>,
}

#[derive(Debug, Clone)]
pub struct HistoryPage {
    pub messages: Vec<Message>,
    pub next_cursor: Option<String>,
}

#[derive(Clone)]
pub struct SlackService {
    client: Arc<SlackHyperClient>,
    http: reqwest::Client,
    bot_token: SlackApiToken,
    app_token: SlackApiToken,
    cache: Arc<SlackServiceCache>,
}

#[derive(Default)]
struct SlackServiceCache {
    bot_user_id: Mutex<Option<SlackUserId>>,
    dm_peer_by_channel: Mutex<HashMap<SlackChannelId, SlackUserId>>,
    user_display_name_by_id: Mutex<HashMap<SlackUserId, String>>,
}

impl SlackService {
    pub fn new(tokens: SlackTokens) -> anyhow::Result<Self> {
        let connector =
            SlackClientHyperConnector::new().context("create Slack hyper HTTPS connector")?;
        let client = Arc::new(SlackClient::new(connector));

        let http = reqwest::Client::builder()
            .user_agent("slack-rs")
            .build()
            .context("init reqwest HTTP client")?;

        let bot_token = SlackApiToken {
            token_value: SlackApiTokenValue(tokens.bot_token().to_string()),
            cookie: None,
            team_id: None,
            scope: None,
            token_type: Some(SlackApiTokenType::Bot),
        };

        let app_token = SlackApiToken {
            token_value: SlackApiTokenValue(tokens.app_token().to_string()),
            cookie: None,
            team_id: None,
            scope: None,
            token_type: Some(SlackApiTokenType::App),
        };

        Ok(Self {
            client,
            http,
            bot_token,
            app_token,
            cache: Arc::new(SlackServiceCache::default()),
        })
    }

    pub fn client(&self) -> Arc<SlackHyperClient> {
        self.client.clone()
    }

    pub fn bot_token(&self) -> &SlackApiToken {
        &self.bot_token
    }

    pub fn app_token(&self) -> &SlackApiToken {
        &self.app_token
    }

    pub async fn socket_mode_url(&self) -> anyhow::Result<String> {
        let session = self.client.open_session(&self.app_token);
        let resp = session
            .apps_connections_open(&SlackApiAppsConnectionOpenRequest {})
            .await
            .context("Slack apps.connections.open")?;

        Ok(resp.url.0.as_str().to_string())
    }

    pub async fn auth_test_bot(&self) -> anyhow::Result<AuthTestSummary> {
        let session = self.client.open_session(&self.bot_token);
        let resp = session.auth_test().await.context("Slack auth.test")?;

        Ok(AuthTestSummary {
            team: resp.team,
            team_id: resp.team_id.to_string(),
            user_id: resp.user_id.to_string(),
            user: resp.user,
        })
    }

    pub async fn list_sidebar_conversations(&self) -> anyhow::Result<SidebarConversations> {
        let session = self.client.open_session(&self.bot_token);

        let req = SlackApiConversationsListRequest {
            cursor: None,
            limit: Some(200),
            exclude_archived: Some(true),
            types: Some(vec![
                SlackConversationType::Public,
                SlackConversationType::Private,
                SlackConversationType::Im,
                SlackConversationType::Mpim,
            ]),
        };

        let mut scroller = req.scroller();
        let mut all = Vec::<SlackChannelInfo>::new();
        while scroller.has_next() {
            let resp = scroller
                .next_mut(&session)
                .await
                .context("Slack conversations.list (scroll)")?;
            all.extend(resp.channels.into_iter());
        }

        let bot_user_id = self.bot_user_id().await.context("resolve bot user id")?;

        let mut channels = Vec::new();
        let mut dms = Vec::new();

        for ch in all {
            let slack_id = ch.id.clone();
            let id = ConversationId::from(slack_id.0.clone());
            let unread = ch.last_state.unread_count_display;

            let kind = classify_conversation_kind(&ch);

            match kind {
                ConversationKind::Channel | ConversationKind::PrivateChannel => {
                    let title = ch.name.clone().unwrap_or_else(|| slack_id.0.clone());
                    channels.push(ConversationSummary {
                        id,
                        kind,
                        title,
                        is_member: ch.flags.is_member.unwrap_or(false),
                        unread_count_display: unread,
                    });
                }
                ConversationKind::Mpim => {
                    let title = ch.name.clone().unwrap_or_else(|| slack_id.0.clone());
                    dms.push(ConversationSummary {
                        id,
                        kind,
                        title,
                        is_member: true,
                        unread_count_display: unread,
                    });
                }
                ConversationKind::Im => {
                    let peer = self
                        .dm_peer_user_id(&slack_id, &bot_user_id)
                        .await
                        .with_context(|| format!("resolve DM peer for {}", slack_id.0))?;
                    let title = self
                        .user_display_name(&peer)
                        .await
                        .with_context(|| format!("resolve user display name for {}", peer.0))?;

                    dms.push(ConversationSummary {
                        id,
                        kind,
                        title,
                        is_member: true,
                        unread_count_display: unread,
                    });
                }
            }
        }

        channels.sort_by(|a, b| {
            a.title
                .to_ascii_lowercase()
                .cmp(&b.title.to_ascii_lowercase())
        });
        dms.sort_by(|a, b| {
            a.title
                .to_ascii_lowercase()
                .cmp(&b.title.to_ascii_lowercase())
        });

        Ok(SidebarConversations { channels, dms })
    }

    pub async fn fetch_conversation_history(
        &self,
        conversation: &ConversationId,
        cursor: Option<&str>,
        limit: u16,
    ) -> anyhow::Result<HistoryPage> {
        let session = self.client.open_session(&self.bot_token);
        let channel = SlackChannelId(conversation.as_str().to_string());

        let req = SlackApiConversationsHistoryRequest {
            channel: Some(channel),
            cursor: cursor.map(|c| SlackCursorId(c.to_string())),
            latest: None,
            limit: Some(limit),
            oldest: None,
            inclusive: None,
            include_all_metadata: None,
        };

        let resp = session
            .conversations_history(&req)
            .await
            .context("Slack conversations.history")?;

        let bot_user_id = self
            .bot_user_id()
            .await
            .context("resolve bot user id for reaction mapping")?;

        let messages = resp
            .messages
            .into_iter()
            .map(|m| Message {
                ts: MessageTs::from(m.origin.ts.0),
                user: m.sender.user.map(|u| UserId::from(u.0)),
                text: {
                    let blocks = m
                        .content
                        .blocks
                        .as_ref()
                        .map(|b| crate::render::blocks::render_blocks_to_plaintext(b))
                        .unwrap_or_default();
                    if blocks.trim().is_empty() {
                        m.content.text.unwrap_or_default()
                    } else {
                        blocks
                    }
                },
                thread_ts: m.origin.thread_ts.map(|t| MessageTs::from(t.0)),
                reactions: m
                    .content
                    .reactions
                    .unwrap_or_default()
                    .into_iter()
                    .map(|r| Reaction {
                        name: r.name.0,
                        count: r.count as u64,
                        me: r.users.iter().any(|u| u == &bot_user_id),
                    })
                    .collect(),
                files: map_slack_files(m.content.files),
            })
            .collect::<Vec<_>>();

        let next_cursor = resp
            .response_metadata
            .and_then(|rm| rm.next_cursor.map(|c| c.0))
            .filter(|c| !c.trim().is_empty());

        Ok(HistoryPage {
            messages,
            next_cursor,
        })
    }

    pub async fn fetch_thread_replies(
        &self,
        conversation: &ConversationId,
        thread_ts: &MessageTs,
        cursor: Option<&str>,
        limit: u16,
    ) -> anyhow::Result<HistoryPage> {
        let session = self.client.open_session(&self.bot_token);
        let channel = SlackChannelId(conversation.as_str().to_string());

        let req = SlackApiConversationsRepliesRequest {
            channel,
            ts: SlackTs(thread_ts.as_str().to_string()),
            cursor: cursor.map(|c| SlackCursorId(c.to_string())),
            latest: None,
            limit: Some(limit),
            oldest: None,
            inclusive: None,
        };

        let resp = session
            .conversations_replies(&req)
            .await
            .context("Slack conversations.replies")?;

        let bot_user_id = self
            .bot_user_id()
            .await
            .context("resolve bot user id for reaction mapping")?;

        let messages = resp
            .messages
            .into_iter()
            .map(|m| Message {
                ts: MessageTs::from(m.origin.ts.0),
                user: m.sender.user.map(|u| UserId::from(u.0)),
                text: {
                    let blocks = m
                        .content
                        .blocks
                        .as_ref()
                        .map(|b| crate::render::blocks::render_blocks_to_plaintext(b))
                        .unwrap_or_default();
                    if blocks.trim().is_empty() {
                        m.content.text.unwrap_or_default()
                    } else {
                        blocks
                    }
                },
                thread_ts: m.origin.thread_ts.map(|t| MessageTs::from(t.0)),
                reactions: m
                    .content
                    .reactions
                    .unwrap_or_default()
                    .into_iter()
                    .map(|r| Reaction {
                        name: r.name.0,
                        count: r.count as u64,
                        me: r.users.iter().any(|u| u == &bot_user_id),
                    })
                    .collect(),
                files: map_slack_files(m.content.files),
            })
            .collect::<Vec<_>>();

        // Keep message ordering consistent with `conversations.history` (newest-first), so
        // incremental updates can always `insert(0, msg)` deterministically.
        let mut messages = messages;
        messages.sort_by(|a, b| slack_ts_cmp_desc(&a.ts, &b.ts));

        let next_cursor = resp
            .response_metadata
            .and_then(|rm| rm.next_cursor.map(|c| c.0))
            .filter(|c| !c.trim().is_empty());

        Ok(HistoryPage {
            messages,
            next_cursor,
        })
    }

    pub async fn send_message(
        &self,
        conversation: &ConversationId,
        text: &str,
        thread_ts: Option<&MessageTs>,
    ) -> anyhow::Result<Message> {
        let session = self.client.open_session(&self.bot_token);
        let channel = SlackChannelId(conversation.as_str().to_string());

        let content = SlackMessageContent::new().with_text(text.to_string());
        let req = SlackApiChatPostMessageRequest {
            channel,
            content,
            as_user: None,
            icon_emoji: None,
            icon_url: None,
            link_names: None,
            parse: None,
            thread_ts: thread_ts.map(|ts| SlackTs(ts.as_str().to_string())),
            username: None,
            reply_broadcast: None,
            unfurl_links: None,
            unfurl_media: None,
        };

        let resp = session
            .chat_post_message(&req)
            .await
            .context("Slack chat.postMessage")?;

        Ok(Message {
            ts: MessageTs::from(resp.ts.0),
            user: resp.message.sender.user.map(|u| UserId::from(u.0)),
            text: resp.message.content.text.unwrap_or_default(),
            thread_ts: resp.message.origin.thread_ts.map(|t| MessageTs::from(t.0)),
            reactions: Vec::new(),
            files: Vec::new(),
        })
    }

    pub async fn add_reaction(
        &self,
        conversation: &ConversationId,
        ts: &MessageTs,
        emoji: &str,
    ) -> anyhow::Result<()> {
        let session = self.client.open_session(&self.bot_token);
        let channel = SlackChannelId(conversation.as_str().to_string());

        let req = SlackApiReactionsAddRequest {
            channel,
            name: SlackReactionName(emoji.to_string()),
            timestamp: SlackTs(ts.as_str().to_string()),
        };

        session
            .reactions_add(&req)
            .await
            .context("Slack reactions.add")?;
        Ok(())
    }

    pub async fn remove_reaction(
        &self,
        conversation: &ConversationId,
        ts: &MessageTs,
        emoji: &str,
    ) -> anyhow::Result<()> {
        let session = self.client.open_session(&self.bot_token);
        let channel = SlackChannelId(conversation.as_str().to_string());

        let req = SlackApiReactionsRemoveRequest {
            name: SlackReactionName(emoji.to_string()),
            channel: Some(channel),
            file: None,
            full: None,
            timestamp: Some(SlackTs(ts.as_str().to_string())),
        };

        session
            .reactions_remove(&req)
            .await
            .context("Slack reactions.remove")?;
        Ok(())
    }

    pub async fn upload_file_external(
        &self,
        conversation: &ConversationId,
        thread_ts: Option<&MessageTs>,
        path: &Path,
    ) -> anyhow::Result<Vec<MessageFile>> {
        let session = self.client.open_session(&self.bot_token);
        let uploaded = files::upload_file_external(&session, conversation, thread_ts, path)
            .await
            .with_context(|| format!("upload file {} to {}", path.display(), conversation))?;

        Ok(map_slack_files(Some(uploaded)))
    }

    pub async fn download_url_private_to_path(
        &self,
        url: &str,
        dest_path: &Path,
    ) -> anyhow::Result<u64> {
        files::download_url_private_to_path(
            &self.http,
            self.bot_token.token_value.0.as_str(),
            url,
            dest_path,
        )
        .await
        .with_context(|| format!("download Slack file to {}", dest_path.display()))
    }

    pub async fn fetch_file_info_summary(
        &self,
        file_id: &str,
    ) -> anyhow::Result<files::FileInfoSummary> {
        let session = self.client.open_session(&self.bot_token);
        files::fetch_file_info_summary(&session, file_id)
            .await
            .with_context(|| format!("fetch Slack file info for {}", file_id))
    }
}

fn slack_ts_cmp_desc(a: &MessageTs, b: &MessageTs) -> std::cmp::Ordering {
    // Slack ts format is typically `<seconds>.<fraction>`. We want "newest first".
    // If parsing fails, fall back to lexicographic string order (best-effort).
    match (parse_slack_ts(a.as_str()), parse_slack_ts(b.as_str())) {
        (Some(a_key), Some(b_key)) => b_key.cmp(&a_key),
        _ => b.as_str().cmp(a.as_str()),
    }
}

fn parse_slack_ts(raw: &str) -> Option<(u64, u32)> {
    let (sec, frac) = raw.split_once('.')?;
    let sec = sec.parse::<u64>().ok()?;
    let frac = frac.parse::<u32>().ok()?;
    Some((sec, frac))
}

fn classify_conversation_kind(info: &SlackChannelInfo) -> ConversationKind {
    if info.flags.is_im.unwrap_or(false) {
        return ConversationKind::Im;
    }
    if info.flags.is_mpim.unwrap_or(false) {
        return ConversationKind::Mpim;
    }
    if info.flags.is_private.unwrap_or(false) || info.flags.is_group.unwrap_or(false) {
        return ConversationKind::PrivateChannel;
    }
    ConversationKind::Channel
}

fn map_slack_files(files: Option<Vec<SlackFile>>) -> Vec<MessageFile> {
    files
        .unwrap_or_default()
        .into_iter()
        .map(|f| MessageFile {
            id: f.id.0,
            name: f.name,
            title: f.title,
            mimetype: f.mimetype.map(|m| m.0),
            filetype: f.filetype.map(|t| t.0),
            url_private_download: f.url_private_download.map(|u| u.to_string()),
            size: None,
            mode: None,
            snippet_preview: None,
        })
        .collect()
}

impl SlackService {
    async fn bot_user_id(&self) -> anyhow::Result<SlackUserId> {
        if let Some(cached) = self
            .cache
            .bot_user_id
            .lock()
            .expect("bot_user_id lock")
            .clone()
        {
            return Ok(cached);
        }

        let session = self.client.open_session(&self.bot_token);
        let resp = session.auth_test().await.context("Slack auth.test")?;
        let id = resp.user_id.clone();

        *self.cache.bot_user_id.lock().expect("bot_user_id lock") = Some(id.clone());

        Ok(id)
    }

    async fn dm_peer_user_id(
        &self,
        channel: &SlackChannelId,
        bot_user_id: &SlackUserId,
    ) -> anyhow::Result<SlackUserId> {
        if let Some(peer) = self
            .cache
            .dm_peer_by_channel
            .lock()
            .expect("dm_peer_by_channel lock")
            .get(channel)
            .cloned()
        {
            return Ok(peer);
        }

        let session = self.client.open_session(&self.bot_token);
        let req = SlackApiConversationsMembersRequest {
            channel: Some(channel.clone()),
            cursor: None,
            limit: Some(200),
        };

        let mut scroller = req.scroller();
        let mut members = Vec::<SlackUserId>::new();
        while scroller.has_next() {
            let resp = scroller
                .next_mut(&session)
                .await
                .context("Slack conversations.members (scroll)")?;
            members.extend(resp.members.into_iter());
        }

        let peer = members
            .into_iter()
            .find(|u| u != bot_user_id)
            .unwrap_or_else(|| bot_user_id.clone());

        self.cache
            .dm_peer_by_channel
            .lock()
            .expect("dm_peer_by_channel lock")
            .insert(channel.clone(), peer.clone());

        Ok(peer)
    }

    async fn user_display_name(&self, user: &SlackUserId) -> anyhow::Result<String> {
        if let Some(name) = self
            .cache
            .user_display_name_by_id
            .lock()
            .expect("user_display_name_by_id lock")
            .get(user)
            .cloned()
        {
            return Ok(name);
        }

        let session = self.client.open_session(&self.bot_token);
        let req = SlackApiUsersInfoRequest {
            user: user.clone(),
            include_locale: None,
        };
        let resp = session.users_info(&req).await.context("Slack users.info")?;

        let name = pick_user_display_name(&resp.user).unwrap_or_else(|| format!("user:{}", user.0));

        self.cache
            .user_display_name_by_id
            .lock()
            .expect("user_display_name_by_id lock")
            .insert(user.clone(), name.clone());

        Ok(name)
    }
}

fn pick_user_display_name(user: &SlackUser) -> Option<String> {
    let display = user
        .profile
        .as_ref()
        .and_then(|p| p.display_name.as_ref())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());
    if display.is_some() {
        return display;
    }

    let name = user
        .name
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());
    if name.is_some() {
        return name;
    }

    user.real_name
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}
