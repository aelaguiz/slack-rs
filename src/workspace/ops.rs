use crate::workspace::tree::{Node, PaneId, PaneKind, PaneTree, SplitAxis};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusDir {
    Left,
    Down,
    Up,
    Right,
}

pub fn split_focused(tree: &mut PaneTree, axis: SplitAxis) -> PaneId {
    let focused = tree.focused();
    let new_id = tree.alloc_pane_id();

    let replaced = split_leaf(&mut tree.root, focused, axis, new_id);
    debug_assert!(replaced, "focused leaf must exist");

    tree.set_focused(new_id);
    new_id
}

pub fn close_focused(tree: &mut PaneTree) -> Option<PaneId> {
    let focused = tree.focused();

    // Can't close the only pane.
    if matches!(tree.root, Node::Leaf { .. }) {
        return None;
    }

    let old_root = std::mem::replace(
        &mut tree.root,
        Node::Leaf {
            id: PaneId::new(u64::MAX),
            kind: PaneKind::Placeholder,
        },
    );

    let (new_root, found) = close_leaf(old_root, focused);
    debug_assert!(found, "focused leaf must exist");

    let Some(new_root) = new_root else {
        // This should be unreachable because we already handled the single-pane case.
        tree.root = Node::Leaf {
            id: PaneId::new(1),
            kind: PaneKind::Placeholder,
        };
        tree.set_focused(PaneId::new(1));
        return Some(PaneId::new(1));
    };

    tree.root = new_root;
    let new_focus = tree.root.leftmost_leaf();
    tree.set_focused(new_focus);
    Some(new_focus)
}

pub fn move_focus(tree: &mut PaneTree, dir: FocusDir) -> PaneId {
    let focused = tree.focused();
    let next = match dir {
        FocusDir::Left => neighbor_left(&tree.root, focused),
        FocusDir::Right => neighbor_right(&tree.root, focused),
        FocusDir::Up => neighbor_up(&tree.root, focused),
        FocusDir::Down => neighbor_down(&tree.root, focused),
    };

    if let Some(next) = next {
        tree.set_focused(next);
    }

    tree.focused()
}

pub fn resize_focused(tree: &mut PaneTree, axis: SplitAxis, delta: f32) -> bool {
    let focused = tree.focused();
    resize_at_nearest_split(&mut tree.root, focused, axis, delta)
}

fn split_leaf(node: &mut Node, target: PaneId, axis: SplitAxis, new_id: PaneId) -> bool {
    match node {
        Node::Leaf { id, kind } if *id == target => {
            let old_leaf = Node::Leaf {
                id: *id,
                kind: kind.clone(),
            };
            let new_leaf = Node::Leaf {
                id: new_id,
                kind: PaneKind::Placeholder,
            };

            *node = Node::Split {
                axis,
                ratio: 0.5,
                first: Box::new(old_leaf),
                second: Box::new(new_leaf),
            };

            true
        }
        Node::Leaf { .. } => false,
        Node::Split { first, second, .. } => {
            split_leaf(first, target, axis, new_id) || split_leaf(second, target, axis, new_id)
        }
    }
}

fn close_leaf(node: Node, target: PaneId) -> (Option<Node>, bool) {
    match node {
        Node::Leaf { id, .. } if id == target => (None, true),
        Node::Leaf { .. } => (Some(node), false),
        Node::Split {
            axis,
            ratio,
            first,
            second,
        } => {
            let (new_first, found) = close_leaf(*first, target);
            if found {
                return match new_first {
                    None => (Some(*second), true),
                    Some(new_first) => (
                        Some(Node::Split {
                            axis,
                            ratio,
                            first: Box::new(new_first),
                            second,
                        }),
                        true,
                    ),
                };
            }

            let first = new_first.expect("close_leaf must return Some(node) when not found");

            let (new_second, found) = close_leaf(*second, target);
            if found {
                return match new_second {
                    None => (Some(first), true),
                    Some(new_second) => (
                        Some(Node::Split {
                            axis,
                            ratio,
                            first: Box::new(first),
                            second: Box::new(new_second),
                        }),
                        true,
                    ),
                };
            }

            let second = new_second.expect("close_leaf must return Some(node) when not found");

            (
                Some(Node::Split {
                    axis,
                    ratio,
                    first: Box::new(first),
                    second: Box::new(second),
                }),
                false,
            )
        }
    }
}

fn neighbor_left(node: &Node, target: PaneId) -> Option<PaneId> {
    match node {
        Node::Leaf { .. } => None,
        Node::Split {
            axis,
            first,
            second,
            ..
        } => {
            if first.contains_leaf(target) {
                neighbor_left(first, target)
            } else if second.contains_leaf(target) {
                neighbor_left(second, target).or_else(|| match axis {
                    SplitAxis::Vertical => Some(first.rightmost_leaf()),
                    SplitAxis::Horizontal => None,
                })
            } else {
                None
            }
        }
    }
}

fn neighbor_right(node: &Node, target: PaneId) -> Option<PaneId> {
    match node {
        Node::Leaf { .. } => None,
        Node::Split {
            axis,
            first,
            second,
            ..
        } => {
            if first.contains_leaf(target) {
                neighbor_right(first, target).or_else(|| match axis {
                    SplitAxis::Vertical => Some(second.leftmost_leaf()),
                    SplitAxis::Horizontal => None,
                })
            } else if second.contains_leaf(target) {
                neighbor_right(second, target)
            } else {
                None
            }
        }
    }
}

fn neighbor_up(node: &Node, target: PaneId) -> Option<PaneId> {
    match node {
        Node::Leaf { .. } => None,
        Node::Split {
            axis,
            first,
            second,
            ..
        } => {
            if first.contains_leaf(target) {
                neighbor_up(first, target)
            } else if second.contains_leaf(target) {
                neighbor_up(second, target).or_else(|| match axis {
                    SplitAxis::Horizontal => Some(first.bottommost_leaf()),
                    SplitAxis::Vertical => None,
                })
            } else {
                None
            }
        }
    }
}

fn neighbor_down(node: &Node, target: PaneId) -> Option<PaneId> {
    match node {
        Node::Leaf { .. } => None,
        Node::Split {
            axis,
            first,
            second,
            ..
        } => {
            if first.contains_leaf(target) {
                neighbor_down(first, target).or_else(|| match axis {
                    SplitAxis::Horizontal => Some(second.topmost_leaf()),
                    SplitAxis::Vertical => None,
                })
            } else if second.contains_leaf(target) {
                neighbor_down(second, target)
            } else {
                None
            }
        }
    }
}

fn resize_at_nearest_split(node: &mut Node, target: PaneId, axis: SplitAxis, delta: f32) -> bool {
    match node {
        Node::Leaf { .. } => false,
        Node::Split {
            axis: split_axis,
            ratio,
            first,
            second,
        } => {
            if first.contains_leaf(target) {
                if resize_at_nearest_split(first, target, axis, delta) {
                    return true;
                }

                if *split_axis == axis {
                    *ratio = clamp_ratio(*ratio + delta);
                    return true;
                }
            } else if second.contains_leaf(target) {
                if resize_at_nearest_split(second, target, axis, delta) {
                    return true;
                }

                if *split_axis == axis {
                    *ratio = clamp_ratio(*ratio - delta);
                    return true;
                }
            }

            false
        }
    }
}

fn clamp_ratio(ratio: f32) -> f32 {
    ratio.clamp(0.1, 0.9)
}
