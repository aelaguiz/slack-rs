use slack_morphism::prelude::*;

pub fn render_blocks_to_plaintext(blocks: &[SlackBlock]) -> String {
    let mut lines = Vec::<String>::new();

    for block in blocks {
        match block {
            SlackBlock::Section(section) => {
                if let Some(text) = section.text.as_ref() {
                    let rendered = render_block_text(text);
                    if !rendered.trim().is_empty() {
                        lines.push(rendered);
                    }
                }

                if let Some(fields) = section.fields.as_ref() {
                    let rendered_fields = fields
                        .iter()
                        .map(render_block_text)
                        .filter(|s| !s.trim().is_empty())
                        .collect::<Vec<_>>();
                    if !rendered_fields.is_empty() {
                        lines.push(rendered_fields.join(" | "));
                    }
                }
            }
            SlackBlock::Context(context) => {
                let rendered = context
                    .elements
                    .iter()
                    .map(render_context_element)
                    .filter(|s| !s.trim().is_empty())
                    .collect::<Vec<_>>();
                if !rendered.is_empty() {
                    lines.push(rendered.join(" · "));
                }
            }
            SlackBlock::Divider(_) => {
                lines.push("────────────────────────".to_string());
            }
            // Ignore-by-default: blocks we don't support yet should fall back to `text`.
            _ => {}
        }
    }

    lines.join("\n")
}

fn render_context_element(el: &SlackContextBlockElement) -> String {
    match el {
        SlackContextBlockElement::Plain(pt) => pt.text.clone(),
        SlackContextBlockElement::MarkDown(md) => {
            crate::render::mrkdwn::render_mrkdwn_to_plaintext(&md.text)
        }
        SlackContextBlockElement::Image(_) => String::new(),
    }
}

fn render_block_text(text: &SlackBlockText) -> String {
    match text {
        SlackBlockText::Plain(pt) => pt.text.clone(),
        SlackBlockText::MarkDown(md) => crate::render::mrkdwn::render_mrkdwn_to_plaintext(&md.text),
    }
}
