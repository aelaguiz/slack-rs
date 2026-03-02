/// Minimal Slack mrkdwn-ish rendering to "usable plaintext".
///
/// This is intentionally conservative and incomplete:
/// - We avoid attempting full Markdown parsing.
/// - We focus on Slack-specific angle-bracket tokens (mentions/links).
/// - Unknown tokens are preserved as-is, so we never silently drop information.
pub fn render_mrkdwn_to_plaintext(input: &str) -> String {
    let input = unescape_basic_entities(input);

    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch != '<' {
            out.push(ch);
            continue;
        }

        let mut token = String::new();
        let mut closed = false;
        for c in chars.by_ref() {
            if c == '>' {
                closed = true;
                break;
            }
            token.push(c);
        }

        if !closed {
            // Unmatched `<` — preserve literally.
            out.push('<');
            out.push_str(&token);
            break;
        }

        match render_angle_token(&token) {
            Some(rendered) => out.push_str(&rendered),
            None => {
                out.push('<');
                out.push_str(&token);
                out.push('>');
            }
        }
    }

    out
}

fn render_angle_token(token: &str) -> Option<String> {
    if token.is_empty() {
        return None;
    }

    if let Some(rest) = token.strip_prefix("@") {
        let (_id, label) = split_label(rest);
        return Some(format!("@{label}"));
    }

    if let Some(rest) = token.strip_prefix("#") {
        let (_id, label) = split_label(rest);
        return Some(format!("#{label}"));
    }

    if let Some(rest) = token.strip_prefix("!") {
        // Special mentions:
        // - <!here>
        // - <!channel>
        // - <!everyone>
        // - <!subteam^S123|@team-name>
        if let Some((_id, label)) = rest.split_once('|') {
            return Some(label.to_string());
        }

        return match rest {
            "here" => Some("@here".to_string()),
            "channel" => Some("@channel".to_string()),
            "everyone" => Some("@everyone".to_string()),
            _ => None,
        };
    }

    if token.starts_with("http://") || token.starts_with("https://") {
        if let Some((url, label)) = token.split_once('|') {
            return Some(format!("{label} ({url})"));
        }
        return Some(token.to_string());
    }

    if let Some(rest) = token.strip_prefix("mailto:") {
        if let Some((_url, label)) = rest.split_once('|') {
            return Some(label.to_string());
        }
        return Some(rest.to_string());
    }

    // Fall back to "label only" for any unknown token that includes a pipe, which matches Slack's
    // general `<url|label>` pattern.
    if let Some((_id, label)) = token.split_once('|') {
        return Some(label.to_string());
    }

    None
}

fn split_label(rest: &str) -> (&str, &str) {
    rest.split_once('|').unwrap_or((rest, rest))
}

fn unescape_basic_entities(input: &str) -> String {
    // Slack text uses HTML-like escaping for a few characters.
    // Keep it minimal; we can expand later if needed.
    input
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
}
