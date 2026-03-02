use crate::model::{ConversationId, ConversationSummary, Message, MessageFile, MessageTs};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    Quit,

    ToggleFocusArea,

    EnterCompose,
    LeaveCompose,
    ComposerSend,
    ComposerNewline,

    SplitVertical,
    SplitHorizontal,
    FocusLeft,
    FocusDown,
    FocusUp,
    FocusRight,
    ClosePane,

    ResizeVerticalPlus,
    ResizeVerticalMinus,
    ResizeHorizontalPlus,
    ResizeHorizontalMinus,

    SidebarRefresh,
    SidebarSelectNext,
    SidebarSelectPrev,
    SidebarOpenSelected,
    OpenConversation {
        conversation: ConversationId,
    },
    OpenThread {
        conversation: ConversationId,
        thread_ts: MessageTs,
    },
    OpenThreadFromSelection,

    SidebarLoaded {
        channels: Vec<ConversationSummary>,
        dms: Vec<ConversationSummary>,
    },

    TimelineLoadOlder,
    TimelineSelectPrev,
    TimelineSelectNext,
    TimelineSelectFirst,
    TimelineSelectLast,

    ReactionPromptOpen,
    ReactionToggle {
        emoji: String,
    },

    FileUploadPromptOpen,
    FileUploadStart {
        path: String,
    },
    FileUploadCompleted {
        conversation: ConversationId,
        thread_ts: Option<MessageTs>,
        files: Vec<MessageFile>,
    },

    FileDownloadPromptOpen,
    FileDownloadStart {
        dest_path: String,
    },
    FileDownloaded {
        dest_path: String,
        bytes: u64,
    },

    FileInfoLoaded {
        file_id: String,
        size: Option<u64>,
        mode: Option<String>,
        preview_plain_text: Option<String>,
    },

    TimelineLoaded {
        conversation: ConversationId,
        messages: Vec<Message>,
        next_cursor: Option<String>,
        append: bool,
    },
    ThreadLoaded {
        conversation: ConversationId,
        thread_ts: MessageTs,
        messages: Vec<Message>,
        next_cursor: Option<String>,
        append: bool,
    },

    SlackMessageReceived {
        conversation: ConversationId,
        message: Message,
    },

    ReactionChanged {
        conversation: ConversationId,
        ts: MessageTs,
        emoji: String,
        delta: i64,
        me: Option<bool>,
    },
}
