use std::fmt;

use anyhow::Context as _;

/// Slack tokens required for Socket Mode + Web API calls.
///
/// Source of truth:
/// - `APP_TOKEN` (Socket Mode, typically starts with `xapp-`)
/// - `BOT_TOKEN` (Web API bot token, typically starts with `xoxb-`)
///
/// These are intentionally not part of `config.toml` to avoid accidental commits.
pub struct SlackTokens {
    app_token: String,
    bot_token: String,
}

impl SlackTokens {
    pub fn from_env() -> anyhow::Result<Self> {
        let app_token = std::env::var("APP_TOKEN").context("missing APP_TOKEN env var")?;
        let bot_token = std::env::var("BOT_TOKEN").context("missing BOT_TOKEN env var")?;

        validate_prefix("APP_TOKEN", &app_token, "xapp-")?;
        validate_prefix("BOT_TOKEN", &bot_token, "xoxb-")?;

        Ok(Self {
            app_token,
            bot_token,
        })
    }

    pub fn app_token(&self) -> &str {
        &self.app_token
    }

    pub fn bot_token(&self) -> &str {
        &self.bot_token
    }
}

impl fmt::Debug for SlackTokens {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SlackTokens")
            .field("app_token", &"<redacted>")
            .field("bot_token", &"<redacted>")
            .finish()
    }
}

fn validate_prefix(
    name: &'static str,
    value: &str,
    expected_prefix: &'static str,
) -> anyhow::Result<()> {
    if value.starts_with(expected_prefix) {
        return Ok(());
    }

    anyhow::bail!("{name} must start with '{expected_prefix}' (got an unexpected token prefix)");
}
