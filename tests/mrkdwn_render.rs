use slack_rs::render::mrkdwn::render_mrkdwn_to_plaintext;

#[test]
fn renders_user_and_channel_mentions() {
    assert_eq!(render_mrkdwn_to_plaintext("<@U123>"), "@U123");
    assert_eq!(render_mrkdwn_to_plaintext("<@U123|alice>"), "@alice");
    assert_eq!(render_mrkdwn_to_plaintext("<#C123|general>"), "#general");
}

#[test]
fn renders_links_conservatively() {
    assert_eq!(
        render_mrkdwn_to_plaintext("<https://example.com|Example>"),
        "Example (https://example.com)"
    );
    assert_eq!(
        render_mrkdwn_to_plaintext("see <https://example.com>"),
        "see https://example.com"
    );
}

#[test]
fn unescapes_basic_entities() {
    assert_eq!(
        render_mrkdwn_to_plaintext("a &lt; b &amp;&amp; b &gt; c"),
        "a < b && b > c"
    );
}
