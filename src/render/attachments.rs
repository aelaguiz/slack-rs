use crate::model::MessageFile;

pub fn render_files_to_plaintext(files: &[MessageFile]) -> Vec<String> {
    if files.is_empty() {
        return Vec::new();
    }

    let mut out = Vec::new();

    for file in files {
        let mut header = String::new();

        let is_snippet = file.mode.as_deref() == Some("snippet");
        if is_snippet {
            header.push_str("[snippet] ");
        } else {
            header.push_str("[file] ");
        }

        header.push_str(&file.display_name());

        if let Some(size) = file.size {
            header.push_str(&format!("  ({size} B)"));
        }
        if let Some(mime) = file.mimetype.as_ref().filter(|s| !s.trim().is_empty()) {
            header.push_str(&format!("  {mime}"));
        } else if let Some(filetype) = file.filetype.as_ref().filter(|s| !s.trim().is_empty()) {
            header.push_str(&format!("  {filetype}"));
        }

        out.push(header);

        if is_snippet {
            match file.snippet_preview.as_deref() {
                None => {
                    out.push("  (snippet preview: loading…)".to_string());
                }
                Some(preview) if preview.trim().is_empty() => {
                    out.push("  (snippet preview unavailable)".to_string());
                }
                Some(preview) => {
                    for line in preview.lines().take(12) {
                        out.push(format!("  {line}"));
                    }
                    if preview.lines().count() > 12 {
                        out.push("  …".to_string());
                    }
                }
            }
        }
    }

    out
}
