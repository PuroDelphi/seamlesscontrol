//! Logical desktop geometry. The transport and platform adapters can map
//! physical pixels to these coordinates before requesting an edge crossing.

use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Option<Self> {
        if width <= 0
            || height <= 0
            || x.checked_add(width).is_none()
            || y.checked_add(height).is_none()
        {
            return None;
        }
        Some(Self {
            x,
            y,
            width,
            height,
        })
    }

    pub fn right(self) -> i32 {
        self.x + self.width
    }
    pub fn bottom(self) -> i32 {
        self.y + self.height
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Edge {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Output {
    pub id: String,
    pub machine_id: String,
    pub rect: Rect,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Crossing {
    pub target_output: String,
    pub target_machine: String,
    pub entry_x: i32,
    pub entry_y: i32,
}

#[derive(Default)]
pub struct Layout {
    outputs: BTreeMap<String, Output>,
}

impl Layout {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, output: Output) -> Option<Output> {
        self.outputs.insert(output.id.clone(), output)
    }

    /// Find the physically adjacent output. A one-pixel inset avoids an
    /// immediate crossing back caused by rounding at the shared edge.
    pub fn crossing(
        &self,
        from_id: &str,
        edge: Edge,
        pointer_x: i32,
        pointer_y: i32,
    ) -> Option<Crossing> {
        let from = self.outputs.get(from_id)?;
        let on_edge = match edge {
            Edge::Left => {
                pointer_x == from.rect.x
                    && pointer_y >= from.rect.y
                    && pointer_y < from.rect.bottom()
            }
            Edge::Right => {
                pointer_x == from.rect.right() - 1
                    && pointer_y >= from.rect.y
                    && pointer_y < from.rect.bottom()
            }
            Edge::Top => {
                pointer_y == from.rect.y
                    && pointer_x >= from.rect.x
                    && pointer_x < from.rect.right()
            }
            Edge::Bottom => {
                pointer_y == from.rect.bottom() - 1
                    && pointer_x >= from.rect.x
                    && pointer_x < from.rect.right()
            }
        };
        if !on_edge {
            return None;
        }
        let candidate = self.outputs.values().find(|to| {
            if to.id == from.id || to.machine_id == from.machine_id {
                return false;
            }
            match edge {
                Edge::Left => {
                    to.rect.right() == from.rect.x
                        && pointer_y >= to.rect.y
                        && pointer_y < to.rect.bottom()
                }
                Edge::Right => {
                    to.rect.x == from.rect.right()
                        && pointer_y >= to.rect.y
                        && pointer_y < to.rect.bottom()
                }
                Edge::Top => {
                    to.rect.bottom() == from.rect.y
                        && pointer_x >= to.rect.x
                        && pointer_x < to.rect.right()
                }
                Edge::Bottom => {
                    to.rect.y == from.rect.bottom()
                        && pointer_x >= to.rect.x
                        && pointer_x < to.rect.right()
                }
            }
        })?;
        let (entry_x, entry_y) = match edge {
            Edge::Left => (
                candidate.rect.right() - 2.min(candidate.rect.width),
                pointer_y,
            ),
            Edge::Right => (
                candidate.rect.x + 1.min(candidate.rect.width - 1),
                pointer_y,
            ),
            Edge::Top => (
                pointer_x,
                candidate.rect.bottom() - 2.min(candidate.rect.height),
            ),
            Edge::Bottom => (
                pointer_x,
                candidate.rect.y + 1.min(candidate.rect.height - 1),
            ),
        };
        Some(Crossing {
            target_output: candidate.id.clone(),
            target_machine: candidate.machine_id.clone(),
            entry_x,
            entry_y,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_by_two_layout_routes_edges_and_rejects_corners_without_neighbor() {
        let mut layout = Layout::new();
        for (id, machine, x, y) in [
            ("a", "one", 0, 0),
            ("b", "two", 100, 0),
            ("c", "three", 0, 100),
            ("d", "four", 100, 100),
        ] {
            layout.insert(Output {
                id: id.into(),
                machine_id: machine.into(),
                rect: Rect::new(x, y, 100, 100).unwrap(),
            });
        }
        let next = layout.crossing("a", Edge::Right, 99, 50).unwrap();
        assert_eq!(
            (next.target_output.as_str(), next.entry_x, next.entry_y),
            ("b", 101, 50)
        );
        assert_eq!(
            layout
                .crossing("a", Edge::Bottom, 50, 99)
                .unwrap()
                .target_output,
            "c"
        );
        assert!(layout.crossing("a", Edge::Right, 99, 100).is_none());
    }

    #[test]
    fn invalid_geometry_is_rejected() {
        assert!(Rect::new(0, 0, 0, 1).is_none());
        assert!(Rect::new(i32::MAX, 0, 10, 10).is_none());
    }
}
