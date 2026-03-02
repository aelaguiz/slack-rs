use ratatui::layout::Rect;

use crate::workspace::tree::{Node, PaneId, PaneKind, PaneTree, SplitAxis};

#[derive(Clone, Debug)]
pub struct PaneLayout {
    pub id: PaneId,
    pub kind: PaneKind,
    pub area: Rect,
}

pub fn layout(tree: &PaneTree, area: Rect) -> Vec<PaneLayout> {
    let mut out = Vec::new();
    layout_node(&tree.root, area, &mut out);
    out
}

fn layout_node(node: &Node, area: Rect, out: &mut Vec<PaneLayout>) {
    match node {
        Node::Leaf { id, kind } => out.push(PaneLayout {
            id: *id,
            kind: kind.clone(),
            area,
        }),
        Node::Split {
            axis,
            ratio,
            first,
            second,
        } => {
            let (first_area, second_area) = split_rect(area, *axis, *ratio);
            layout_node(first, first_area, out);
            layout_node(second, second_area, out);
        }
    }
}

fn split_rect(area: Rect, axis: SplitAxis, ratio: f32) -> (Rect, Rect) {
    match axis {
        SplitAxis::Vertical => {
            let total = area.width.max(2);
            let mut first_w = ((total as f32) * ratio).round() as u16;
            first_w = first_w.clamp(1, total - 1);

            let second_w = total - first_w;
            let first = Rect {
                x: area.x,
                y: area.y,
                width: first_w,
                height: area.height,
            };
            let second = Rect {
                x: area.x + first_w,
                y: area.y,
                width: second_w,
                height: area.height,
            };
            (first, second)
        }
        SplitAxis::Horizontal => {
            let total = area.height.max(2);
            let mut first_h = ((total as f32) * ratio).round() as u16;
            first_h = first_h.clamp(1, total - 1);

            let second_h = total - first_h;
            let first = Rect {
                x: area.x,
                y: area.y,
                width: area.width,
                height: first_h,
            };
            let second = Rect {
                x: area.x,
                y: area.y + first_h,
                width: area.width,
                height: second_h,
            };
            (first, second)
        }
    }
}
