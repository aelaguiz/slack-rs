use slack_rs::app::action::Action;
use slack_rs::app::pane_view::PaneViewState;
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
fn timeline_selection_moves_in_display_order_oldest_to_newest() {
    let conversation = ConversationId::from("C123");
    let mut state = AppState::default();
    state.workspace.set_focused_kind(PaneKind::Timeline {
        conversation: conversation.clone(),
    });

    // Internal invariant: timeline cache is stored newest-first.
    state.timelines.insert(
        conversation,
        TimelineState {
            messages: vec![msg("3"), msg("2"), msg("1")],
            loading: false,
            next_cursor: None,
        },
    );

    let focused = state.workspace.focused();
    state.pane_views.insert(focused, PaneViewState::default());

    // With no selection, we default to the newest message (bottom).
    reducer::apply_action(&mut state, Action::TimelineSelectPrev).unwrap();
    assert_eq!(
        state.pane_views[&focused]
            .selected_ts
            .as_ref()
            .unwrap()
            .as_str(),
        "2"
    );

    reducer::apply_action(&mut state, Action::TimelineSelectPrev).unwrap();
    assert_eq!(
        state.pane_views[&focused]
            .selected_ts
            .as_ref()
            .unwrap()
            .as_str(),
        "1"
    );

    // Clamp at the oldest message.
    reducer::apply_action(&mut state, Action::TimelineSelectPrev).unwrap();
    assert_eq!(
        state.pane_views[&focused]
            .selected_ts
            .as_ref()
            .unwrap()
            .as_str(),
        "1"
    );

    reducer::apply_action(&mut state, Action::TimelineSelectNext).unwrap();
    assert_eq!(
        state.pane_views[&focused]
            .selected_ts
            .as_ref()
            .unwrap()
            .as_str(),
        "2"
    );

    reducer::apply_action(&mut state, Action::TimelineSelectLast).unwrap();
    assert_eq!(
        state.pane_views[&focused]
            .selected_ts
            .as_ref()
            .unwrap()
            .as_str(),
        "3"
    );

    reducer::apply_action(&mut state, Action::TimelineSelectFirst).unwrap();
    assert_eq!(
        state.pane_views[&focused]
            .selected_ts
            .as_ref()
            .unwrap()
            .as_str(),
        "1"
    );
}
