use crate::model::{ConversationId, MessageTs};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ThreadKey {
    pub conversation: ConversationId,
    pub thread_ts: MessageTs,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThreadSummary {
    pub key: ThreadKey,
    /// UI-friendly label for the sidebar (best-effort; can be refined later).
    pub title: String,
}
