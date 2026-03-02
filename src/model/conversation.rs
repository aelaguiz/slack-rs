use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConversationId(String);

impl ConversationId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ConversationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for ConversationId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for ConversationId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConversationKind {
    Channel,
    PrivateChannel,
    Im,
    Mpim,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConversationSummary {
    pub id: ConversationId,
    pub kind: ConversationKind,
    /// UI-friendly title, without the leading `#`/`@` prefix.
    pub title: String,
    /// Whether the current token is a member of the conversation (relevant for channels).
    pub is_member: bool,
    pub unread_count_display: Option<u64>,
}

impl ConversationSummary {
    pub fn display_title(&self) -> String {
        match self.kind {
            ConversationKind::Channel | ConversationKind::PrivateChannel => {
                format!("#{}", self.title)
            }
            ConversationKind::Im | ConversationKind::Mpim => format!("@{}", self.title),
        }
    }
}
