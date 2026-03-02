use crate::model::Message;

#[derive(Clone, Debug, Default)]
pub struct TimelineState {
    pub messages: Vec<Message>,
    pub loading: bool,
    pub next_cursor: Option<String>,
}

impl TimelineState {
    pub fn has_more(&self) -> bool {
        self.next_cursor.is_some()
    }
}
