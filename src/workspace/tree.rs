use std::fmt;

use crate::model::{ConversationId, MessageTs};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PaneId(u64);

impl PaneId {
    pub fn get(self) -> u64 {
        self.0
    }

    pub(crate) const fn new(raw: u64) -> Self {
        Self(raw)
    }
}

impl fmt::Display for PaneId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SplitAxis {
    /// Left/right panes (Vim `C-w v`).
    Vertical,
    /// Top/bottom panes (Vim `C-w s`).
    Horizontal,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PaneKind {
    Placeholder,
    Timeline {
        conversation: ConversationId,
    },
    Thread {
        conversation: ConversationId,
        thread_ts: MessageTs,
    },
}

#[derive(Clone, Debug)]
pub(crate) enum Node {
    Leaf {
        id: PaneId,
        kind: PaneKind,
    },
    Split {
        axis: SplitAxis,
        /// Fraction of space given to `first` (left/top). Always clamped.
        ratio: f32,
        first: Box<Node>,
        second: Box<Node>,
    },
}

impl Node {
    pub(crate) fn contains_leaf(&self, id: PaneId) -> bool {
        match self {
            Node::Leaf { id: leaf_id, .. } => *leaf_id == id,
            Node::Split { first, second, .. } => {
                first.contains_leaf(id) || second.contains_leaf(id)
            }
        }
    }

    pub(crate) fn leftmost_leaf(&self) -> PaneId {
        match self {
            Node::Leaf { id, .. } => *id,
            Node::Split {
                axis,
                first,
                second: _,
                ..
            } => match axis {
                SplitAxis::Vertical => first.leftmost_leaf(),
                // Horizontal splits don't change x-range; pick deterministically.
                SplitAxis::Horizontal => first.leftmost_leaf(),
            },
        }
    }

    pub(crate) fn rightmost_leaf(&self) -> PaneId {
        match self {
            Node::Leaf { id, .. } => *id,
            Node::Split {
                axis,
                first: _,
                second,
                ..
            } => match axis {
                SplitAxis::Vertical => second.rightmost_leaf(),
                // Horizontal splits don't change x-range; pick deterministically.
                SplitAxis::Horizontal => second.rightmost_leaf(),
            },
        }
    }

    pub(crate) fn topmost_leaf(&self) -> PaneId {
        match self {
            Node::Leaf { id, .. } => *id,
            Node::Split {
                axis,
                first,
                second: _,
                ..
            } => match axis {
                SplitAxis::Horizontal => first.topmost_leaf(),
                // Vertical splits don't change y-range; pick deterministically.
                SplitAxis::Vertical => first.topmost_leaf(),
            },
        }
    }

    pub(crate) fn bottommost_leaf(&self) -> PaneId {
        match self {
            Node::Leaf { id, .. } => *id,
            Node::Split {
                axis,
                first: _,
                second,
                ..
            } => match axis {
                SplitAxis::Horizontal => second.bottommost_leaf(),
                // Vertical splits don't change y-range; pick deterministically.
                SplitAxis::Vertical => second.bottommost_leaf(),
            },
        }
    }

    pub(crate) fn leaf_ids(&self, out: &mut Vec<PaneId>) {
        match self {
            Node::Leaf { id, .. } => out.push(*id),
            Node::Split { first, second, .. } => {
                first.leaf_ids(out);
                second.leaf_ids(out);
            }
        }
    }

    pub(crate) fn leaf_kinds(&self, out: &mut Vec<(PaneId, PaneKind)>) {
        match self {
            Node::Leaf { id, kind } => out.push((*id, kind.clone())),
            Node::Split { first, second, .. } => {
                first.leaf_kinds(out);
                second.leaf_kinds(out);
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct PaneTree {
    pub(crate) root: Node,
    focused: PaneId,
    next_id: u64,
}

impl Default for PaneTree {
    fn default() -> Self {
        let first = PaneId::new(1);
        Self {
            root: Node::Leaf {
                id: first,
                kind: PaneKind::Placeholder,
            },
            focused: first,
            next_id: 2,
        }
    }
}

impl PaneTree {
    pub fn focused(&self) -> PaneId {
        self.focused
    }

    pub fn focused_kind(&self) -> PaneKind {
        let focused = self.focused;
        find_leaf_kind(&self.root, focused)
            .expect("focused leaf must exist")
            .clone()
    }

    pub fn leaf_ids(&self) -> Vec<PaneId> {
        let mut out = Vec::new();
        self.root.leaf_ids(&mut out);
        out
    }

    pub fn leaf_kinds(&self) -> Vec<(PaneId, PaneKind)> {
        let mut out = Vec::new();
        self.root.leaf_kinds(&mut out);
        out
    }

    pub(crate) fn alloc_pane_id(&mut self) -> PaneId {
        let id = PaneId::new(self.next_id);
        self.next_id += 1;
        id
    }

    pub(crate) fn set_focused(&mut self, id: PaneId) {
        debug_assert!(self.root.contains_leaf(id), "focus must point to a leaf");
        self.focused = id;
    }

    pub fn set_focused_kind(&mut self, kind: PaneKind) {
        let focused = self.focused;
        let updated = set_leaf_kind(&mut self.root, focused, kind);
        debug_assert!(updated, "focused leaf must exist");
    }
}

fn set_leaf_kind(node: &mut Node, target: PaneId, kind: PaneKind) -> bool {
    match node {
        Node::Leaf { id, kind: existing } if *id == target => {
            *existing = kind;
            true
        }
        Node::Leaf { .. } => false,
        Node::Split { first, second, .. } => {
            set_leaf_kind(first, target, kind.clone()) || set_leaf_kind(second, target, kind)
        }
    }
}

fn find_leaf_kind(node: &Node, target: PaneId) -> Option<&PaneKind> {
    match node {
        Node::Leaf { id, kind } if *id == target => Some(kind),
        Node::Leaf { .. } => None,
        Node::Split { first, second, .. } => {
            find_leaf_kind(first, target).or_else(|| find_leaf_kind(second, target))
        }
    }
}
