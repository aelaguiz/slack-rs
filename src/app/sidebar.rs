use crate::model::ConversationId;
use crate::model::ConversationSummary;
use crate::model::{ThreadKey, ThreadSummary};

#[derive(Debug, Default)]
pub struct SidebarState {
    pub threads: Vec<ThreadSummary>,
    pub channels: Vec<ConversationSummary>,
    pub dms: Vec<ConversationSummary>,

    /// Selected index across the concatenated selectable list:
    /// `threads[..]` then `channels[..]` then `dms[..]`.
    pub selected_idx: Option<usize>,

    pub loading: bool,
}

impl SidebarState {
    pub fn selectable_len(&self) -> usize {
        self.threads.len() + self.channels.len() + self.dms.len()
    }

    pub fn clamp_selection(&mut self) {
        let len = self.selectable_len();
        if len == 0 {
            self.selected_idx = None;
            return;
        }

        let idx = self.selected_idx.unwrap_or(0);
        self.selected_idx = Some(idx.min(len.saturating_sub(1)));
    }

    pub fn select_next(&mut self) {
        let len = self.selectable_len();
        if len == 0 {
            self.selected_idx = None;
            return;
        }

        let idx = self.selected_idx.unwrap_or(0);
        self.selected_idx = Some((idx + 1).min(len - 1));
    }

    pub fn select_prev(&mut self) {
        let len = self.selectable_len();
        if len == 0 {
            self.selected_idx = None;
            return;
        }

        let idx = self.selected_idx.unwrap_or(0);
        self.selected_idx = Some(idx.saturating_sub(1));
    }

    pub fn selected_conversation_id(&self) -> Option<ConversationId> {
        let idx = self.selected_idx?;
        if idx < self.threads.len() {
            return None;
        }

        let channel_idx = idx - self.threads.len();
        if channel_idx < self.channels.len() {
            return Some(self.channels[channel_idx].id.clone());
        }

        let dm_idx = channel_idx - self.channels.len();
        self.dms.get(dm_idx).map(|dm| dm.id.clone())
    }

    pub fn selected_conversation(&self) -> Option<&ConversationSummary> {
        let idx = self.selected_idx?;
        if idx < self.threads.len() {
            return None;
        }

        let channel_idx = idx - self.threads.len();
        if channel_idx < self.channels.len() {
            return self.channels.get(channel_idx);
        }

        let dm_idx = channel_idx - self.channels.len();
        self.dms.get(dm_idx)
    }

    pub fn selected_thread_key(&self) -> Option<ThreadKey> {
        let idx = self.selected_idx?;
        self.threads.get(idx).map(|t| t.key.clone())
    }

    pub fn selected_thread(&self) -> Option<&ThreadSummary> {
        let idx = self.selected_idx?;
        self.threads.get(idx)
    }
}
