use slack_rs::workspace::ops::{self, FocusDir};
use slack_rs::workspace::tree::{PaneTree, SplitAxis};

#[test]
fn focus_is_always_a_leaf() {
    let mut tree = PaneTree::default();
    assert!(tree.leaf_ids().contains(&tree.focused()));

    ops::split_focused(&mut tree, SplitAxis::Vertical);
    assert!(tree.leaf_ids().contains(&tree.focused()));

    ops::move_focus(&mut tree, FocusDir::Left);
    assert!(tree.leaf_ids().contains(&tree.focused()));

    ops::close_focused(&mut tree);
    assert!(tree.leaf_ids().contains(&tree.focused()));
}

#[test]
fn split_adds_a_new_leaf_and_focuses_it() {
    let mut tree = PaneTree::default();
    let old_focus = tree.focused();

    let new_id = ops::split_focused(&mut tree, SplitAxis::Vertical);
    assert_eq!(tree.focused(), new_id);

    let leaf_ids = tree.leaf_ids();
    assert_eq!(leaf_ids.len(), 2);
    assert!(leaf_ids.contains(&old_focus));
    assert!(leaf_ids.contains(&new_id));
}

#[test]
fn close_collapses_splits_and_keeps_valid_focus() {
    let mut tree = PaneTree::default();
    let _ = ops::split_focused(&mut tree, SplitAxis::Vertical);
    let _ = ops::split_focused(&mut tree, SplitAxis::Horizontal);

    let before = tree.leaf_ids().len();
    let closed_to = ops::close_focused(&mut tree).expect("can close when there are multiple panes");
    let after = tree.leaf_ids().len();

    assert!(after < before);
    assert_eq!(tree.focused(), closed_to);
    assert!(tree.leaf_ids().contains(&tree.focused()));
}

#[test]
fn focus_moves_across_vertical_splits() {
    let mut tree = PaneTree::default();
    let left = tree.focused();
    let right = ops::split_focused(&mut tree, SplitAxis::Vertical);

    // Split focuses the newly created pane by default.
    assert_eq!(tree.focused(), right);

    ops::move_focus(&mut tree, FocusDir::Left);
    assert_eq!(tree.focused(), left);

    ops::move_focus(&mut tree, FocusDir::Right);
    assert_eq!(tree.focused(), right);
}
