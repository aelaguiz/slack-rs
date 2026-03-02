use slack_rs::app::action::Action;
use slack_rs::app::effects::Effect;
use slack_rs::app::reducer;
use slack_rs::app::state::AppState;
use slack_rs::app::timeline::TimelineState;
use slack_rs::model::{ConversationId, Message, MessageTs};
use slack_rs::workspace::tree::PaneKind;

fn msg(ts: &str) -> Message {
    Message {
        ts: MessageTs::from(ts),
        user: None,
        text: format!("msg {ts}"),
        thread_ts: None,
        reactions: Vec::new(),
        files: Vec::new(),
    }
}

#[test]
fn timeline_load_older_emits_append_effect_for_focused_timeline() {
    let conversation = ConversationId::from("C123");
    let mut state = AppState::default();
    state.workspace.set_focused_kind(PaneKind::Timeline {
        conversation: conversation.clone(),
    });
    state.timelines.insert(
        conversation.clone(),
        TimelineState {
            messages: vec![msg("3")],
            loading: false,
            next_cursor: Some("cursor-1".to_string()),
        },
    );

    let out = reducer::apply_action(&mut state, Action::TimelineLoadOlder).unwrap();
    assert_eq!(out.effects.len(), 1);
    assert_eq!(
        out.effects[0],
        Effect::SlackLoadHistory {
            conversation,
            cursor: Some("cursor-1".to_string()),
            limit: 50,
            append: true,
        }
    );
}

#[test]
fn timeline_loaded_merges_and_dedups_on_append() {
    let conversation = ConversationId::from("C123");
    let mut state = AppState::default();

    // Simulate a Socket Mode message arriving while the first history fetch is in flight.
    state.timelines.insert(
        conversation.clone(),
        TimelineState {
            messages: vec![msg("3")],
            loading: true,
            next_cursor: None,
        },
    );

    reducer::apply_action(
        &mut state,
        Action::TimelineLoaded {
            conversation: conversation.clone(),
            messages: vec![msg("3"), msg("2"), msg("1")],
            next_cursor: Some("cursor-1".to_string()),
            append: false,
        },
    )
    .unwrap();

    let timeline = state.timelines.get(&conversation).unwrap();
    assert_eq!(timeline.loading, false);
    assert_eq!(
        timeline
            .messages
            .iter()
            .map(|m| m.ts.as_str())
            .collect::<Vec<_>>(),
        vec!["3", "2", "1"]
    );
    assert_eq!(timeline.next_cursor.as_deref(), Some("cursor-1"));
    assert_eq!(timeline.has_more(), true);

    // Now append an older page.
    reducer::apply_action(
        &mut state,
        Action::TimelineLoaded {
            conversation: conversation.clone(),
            messages: vec![msg("1"), msg("0")],
            next_cursor: None,
            append: true,
        },
    )
    .unwrap();

    let timeline = state.timelines.get(&conversation).unwrap();
    assert_eq!(
        timeline
            .messages
            .iter()
            .map(|m| m.ts.as_str())
            .collect::<Vec<_>>(),
        vec!["3", "2", "1", "0"]
    );
    assert_eq!(timeline.next_cursor, None);
    assert_eq!(timeline.has_more(), false);
}
