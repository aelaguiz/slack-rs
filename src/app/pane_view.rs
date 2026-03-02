use crate::model::MessageTs;

#[derive(Clone, Debug, Default)]
pub struct PaneViewState {
    /// Selected message within a timeline pane (by message ts).
    ///
    /// Stored as a stable identifier rather than an index so pagination / new message inserts
    /// don't invalidate the cursor.
    pub selected_ts: Option<MessageTs>,
}
