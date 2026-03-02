use std::fmt;

use crate::model::user::UserId;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MessageTs(String);

impl MessageTs {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for MessageTs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for MessageTs {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for MessageTs {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    pub ts: MessageTs,
    pub user: Option<UserId>,
    pub text: String,
    pub thread_ts: Option<MessageTs>,
    pub reactions: Vec<Reaction>,
    pub files: Vec<MessageFile>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reaction {
    pub name: String,
    pub count: u64,
    pub me: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MessageFile {
    pub id: String,
    pub name: Option<String>,
    pub title: Option<String>,
    pub mimetype: Option<String>,
    pub filetype: Option<String>,
    pub url_private_download: Option<String>,
    pub size: Option<u64>,
    pub mode: Option<String>,
    pub snippet_preview: Option<String>,
}

impl MessageFile {
    pub fn display_name(&self) -> String {
        self.title
            .as_ref()
            .filter(|s| !s.trim().is_empty())
            .cloned()
            .or_else(|| self.name.as_ref().filter(|s| !s.trim().is_empty()).cloned())
            .unwrap_or_else(|| self.id.clone())
    }
}
