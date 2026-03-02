use tui_textarea::TextArea;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptKind {
    ReactionEmoji,
    FileUploadPath,
    FileDownloadPath,
}

#[derive(Debug)]
pub struct PromptState {
    pub kind: PromptKind,
    pub input: TextArea<'static>,
}

impl PromptState {
    pub fn reaction_emoji(prefill: &str) -> Self {
        let mut input = TextArea::default();
        if !prefill.trim().is_empty() {
            input.insert_str(prefill.trim());
        }

        Self {
            kind: PromptKind::ReactionEmoji,
            input,
        }
    }

    pub fn file_upload_path(prefill: &str) -> Self {
        let mut input = TextArea::default();
        if !prefill.trim().is_empty() {
            input.insert_str(prefill.trim());
        }

        Self {
            kind: PromptKind::FileUploadPath,
            input,
        }
    }

    pub fn file_download_path(prefill: &str) -> Self {
        let mut input = TextArea::default();
        if !prefill.trim().is_empty() {
            input.insert_str(prefill.trim());
        }

        Self {
            kind: PromptKind::FileDownloadPath,
            input,
        }
    }

    pub fn value(&self) -> String {
        // Prompts are intended to be single-line; join defensively.
        self.input.lines().join("")
    }
}
