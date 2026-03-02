use std::path::{Path, PathBuf};

use anyhow::Context as _;
use slack_morphism::prelude::*;

use crate::model::{ConversationId, MessageTs};

// NOTE: Slack deprecated `files.upload`.
//
// We intentionally use the modern external upload flow:
// - `files.getUploadURLExternal`
// - upload bytes to `upload_url`
// - `files.completeUploadExternal`

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileInfoSummary {
    pub file_id: String,
    pub size: Option<u64>,
    pub mode: Option<String>,
    pub preview_plain_text: Option<String>,
}

pub async fn upload_file_external<SCHC>(
    session: &SlackClientSession<'_, SCHC>,
    conversation: &ConversationId,
    thread_ts: Option<&MessageTs>,
    path: &Path,
) -> anyhow::Result<Vec<SlackFile>>
where
    SCHC: SlackClientHttpConnector + Send,
{
    let filename = path
        .file_name()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("upload path has no filename: {}", path.display()))?;

    let metadata = tokio::fs::metadata(path)
        .await
        .with_context(|| format!("stat upload file {}", path.display()))?;
    let length = metadata.len() as usize;

    let upload = session
        .get_upload_url_external(&SlackApiFilesGetUploadUrlExternalRequest {
            filename: filename.clone(),
            length,
            alt_txt: None,
            snippet_type: None,
        })
        .await
        .context("Slack files.getUploadURLExternal")?;

    let content = tokio::fs::read(path)
        .await
        .with_context(|| format!("read upload file {}", path.display()))?;

    // Content type is not critical for the upload URL flow; keep it simple until we need better.
    let content_type = "application/octet-stream".to_string();
    session
        .files_upload_via_url(&SlackApiFilesUploadViaUrlRequest {
            upload_url: upload.upload_url.clone(),
            content,
            content_type,
        })
        .await
        .context("Slack files_upload_via_url")?;

    let req = SlackApiFilesCompleteUploadExternalRequest {
        files: vec![SlackApiFilesComplete {
            id: upload.file_id,
            title: Some(filename),
        }],
        channel_id: Some(SlackChannelId(conversation.as_str().to_string())),
        initial_comment: None,
        thread_ts: thread_ts.map(|ts| SlackTs(ts.as_str().to_string())),
    };

    let resp = session
        .files_complete_upload_external(&req)
        .await
        .context("Slack files.completeUploadExternal")?;

    Ok(resp.files)
}

pub async fn fetch_file_info_summary<SCHC>(
    session: &SlackClientSession<'_, SCHC>,
    file_id: &str,
) -> anyhow::Result<FileInfoSummary>
where
    SCHC: SlackClientHttpConnector + Send,
{
    let params = vec![("file", Some(file_id))];
    let resp: serde_json::Value = session
        .http_session_api
        .http_get("files.info", &params, None)
        .await
        .context("Slack files.info")?;

    let file = resp
        .get("file")
        .context("Slack files.info response missing `file`")?;

    let file_id = file
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or(file_id)
        .to_string();

    let size = file.get("size").and_then(|v| v.as_u64());
    let mode = file
        .get("mode")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let preview_plain_text = file
        .get("preview_plain_text")
        .and_then(|v| v.as_str())
        .or_else(|| file.get("preview").and_then(|v| v.as_str()))
        .map(truncate_preview_plaintext);

    Ok(FileInfoSummary {
        file_id,
        size,
        mode,
        preview_plain_text,
    })
}

fn truncate_preview_plaintext(raw: &str) -> String {
    const MAX_CHARS: usize = 1_200;
    const MAX_LINES: usize = 24;

    let mut out = String::new();
    let mut chars = 0_usize;
    let mut lines = 0_usize;

    for line in raw.lines() {
        if lines >= MAX_LINES || chars >= MAX_CHARS {
            break;
        }
        let remaining = MAX_CHARS.saturating_sub(chars);
        if remaining == 0 {
            break;
        }
        let mut slice = line;
        if slice.len() > remaining {
            slice = &slice[..remaining];
        }
        out.push_str(slice);
        out.push('\n');
        chars = chars.saturating_add(slice.len() + 1);
        lines = lines.saturating_add(1);
    }

    if raw.len() > out.len() {
        out.push_str("…\n");
    }

    out
}

pub async fn download_url_private_to_path(
    http: &reqwest::Client,
    bot_token: &str,
    url: &str,
    dest_path: &Path,
) -> anyhow::Result<u64> {
    if dest_path.exists() {
        anyhow::bail!(
            "refusing to overwrite existing file: {}",
            dest_path.display()
        );
    }

    let bytes = http
        .get(url)
        .bearer_auth(bot_token)
        .send()
        .await
        .with_context(|| format!("GET {}", redact_url_for_error(url)))?
        .error_for_status()
        .with_context(|| format!("GET {} (status)", redact_url_for_error(url)))?
        .bytes()
        .await
        .with_context(|| format!("read body {}", redact_url_for_error(url)))?;

    write_atomic(dest_path, &bytes).await
}

async fn write_atomic(dest_path: &Path, bytes: &[u8]) -> anyhow::Result<u64> {
    let parent = dest_path.parent().unwrap_or_else(|| Path::new("."));
    tokio::fs::create_dir_all(parent)
        .await
        .with_context(|| format!("create parent dir {}", parent.display()))?;

    let tmp_path = tmp_path_for(dest_path);
    tokio::fs::write(&tmp_path, bytes)
        .await
        .with_context(|| format!("write temp file {}", tmp_path.display()))?;

    tokio::fs::rename(&tmp_path, dest_path)
        .await
        .with_context(|| {
            format!(
                "rename temp file {} -> {}",
                tmp_path.display(),
                dest_path.display()
            )
        })?;

    Ok(bytes.len() as u64)
}

fn tmp_path_for(dest_path: &Path) -> PathBuf {
    let filename = dest_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("download");
    dest_path.with_file_name(format!("{filename}.part"))
}

fn redact_url_for_error(raw: &str) -> String {
    // File URLs can contain signed query strings; keep errors readable without leaking secrets.
    raw.split_once('?')
        .map(|(a, _)| a.to_string())
        .unwrap_or_else(|| raw.to_string())
}
