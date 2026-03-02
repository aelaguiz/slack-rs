use ratatui::layout::Rect;

use slack_rs::app::action::Action;
use slack_rs::app::reducer;
use slack_rs::app::state::AppState;
use slack_rs::workspace::render;

fn focused_rect(state: &AppState, area: Rect) -> Rect {
    let focused = state.workspace.focused();
    let layouts = render::layout(&state.workspace, area);
    layouts
        .into_iter()
        .find(|p| p.id == focused)
        .expect("focused pane should exist in layout")
        .area
}

#[test]
fn resize_vertical_grows_and_shrinks_focused_pane_width() {
    let area = Rect {
        x: 0,
        y: 0,
        width: 100,
        height: 40,
    };

    let mut state = AppState::default();
    reducer::apply_action(&mut state, Action::SplitVertical).unwrap();

    let before = focused_rect(&state, area).width;
    reducer::apply_action(&mut state, Action::ResizeVerticalPlus).unwrap();
    let after_plus = focused_rect(&state, area).width;
    reducer::apply_action(&mut state, Action::ResizeVerticalMinus).unwrap();
    let after_minus = focused_rect(&state, area).width;

    assert!(
        after_plus > before,
        "expected vertical plus to grow focused width: before={before} after={after_plus}"
    );
    assert!(
        after_minus < after_plus,
        "expected vertical minus to shrink focused width: after_plus={after_plus} after_minus={after_minus}"
    );
}

#[test]
fn resize_horizontal_grows_and_shrinks_focused_pane_height() {
    let area = Rect {
        x: 0,
        y: 0,
        width: 120,
        height: 50,
    };

    let mut state = AppState::default();
    reducer::apply_action(&mut state, Action::SplitHorizontal).unwrap();

    let before = focused_rect(&state, area).height;
    reducer::apply_action(&mut state, Action::ResizeHorizontalPlus).unwrap();
    let after_plus = focused_rect(&state, area).height;
    reducer::apply_action(&mut state, Action::ResizeHorizontalMinus).unwrap();
    let after_minus = focused_rect(&state, area).height;

    assert!(
        after_plus > before,
        "expected horizontal plus to grow focused height: before={before} after={after_plus}"
    );
    assert!(
        after_minus < after_plus,
        "expected horizontal minus to shrink focused height: after_plus={after_plus} after_minus={after_minus}"
    );
}
