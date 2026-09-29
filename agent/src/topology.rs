//! Logical desktop geometry. The transport and platform adapters can map
//! physical pixels to these coordinates before requesting an edge crossing.

use std::collections::BTreeMap;
use std::fmt;
use std::net::IpAddr;

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

/// A portal pointer barrier, represented by its inclusive end points.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct BarrierSegment {
    pub x1: i32,
    pub y1: i32,
    pub x2: i32,
    pub y2: i32,
}

/// Expose only the parts of a chosen edge that face outside the local monitor
/// union. A barrier at a seam between two local monitors would steal the
/// pointer while it moves normally between them.
pub fn external_barriers(regions: &[Rect], edge: Edge) -> Vec<BarrierSegment> {
    let mut barriers = Vec::new();
    for (index, region) in regions.iter().enumerate() {
        let (start, end) = match edge {
            Edge::Left | Edge::Right => (region.y, region.bottom()),
            Edge::Top | Edge::Bottom => (region.x, region.right()),
        };
        let mut exposed = vec![(start, end)];
        for (other_index, other) in regions.iter().enumerate() {
            if index == other_index {
                continue;
            }
            let covered = match edge {
                Edge::Left => other.x < region.x && other.right() >= region.x,
                Edge::Right => other.x <= region.right() && other.right() > region.right(),
                Edge::Top => other.y < region.y && other.bottom() >= region.y,
                Edge::Bottom => other.y <= region.bottom() && other.bottom() > region.bottom(),
            };
            if !covered {
                continue;
            }
            let (cover_start, cover_end) = match edge {
                Edge::Left | Edge::Right => (other.y, other.bottom()),
                Edge::Top | Edge::Bottom => (other.x, other.right()),
            };
            let mut remainder = Vec::new();
            for (part_start, part_end) in exposed {
                if cover_end <= part_start || cover_start >= part_end {
                    remainder.push((part_start, part_end));
                    continue;
                }
                if cover_start > part_start {
                    remainder.push((part_start, cover_start.min(part_end)));
                }
                if cover_end < part_end {
                    remainder.push((cover_end.max(part_start), part_end));
                }
            }
            exposed = remainder;
        }
        for (part_start, part_end) in exposed {
            if part_start >= part_end {
                continue;
            }
            barriers.push(match edge {
                Edge::Left => BarrierSegment {
                    x1: region.x,
                    y1: part_start,
                    x2: region.x,
                    y2: part_end - 1,
                },
                Edge::Right => BarrierSegment {
                    x1: region.right(),
                    y1: part_start,
                    x2: region.right(),
                    y2: part_end - 1,
                },
                Edge::Top => BarrierSegment {
                    x1: part_start,
                    y1: region.y,
                    x2: part_end - 1,
                    y2: region.y,
                },
                Edge::Bottom => BarrierSegment {
                    x1: part_start,
                    y1: region.bottom(),
                    x2: part_end - 1,
                    y2: region.bottom(),
                },
            });
        }
    }
    barriers.sort_unstable();
    barriers.dedup();
    barriers
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Edge {
    Left,
    Right,
    Top,
    Bottom,
}

impl Edge {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
            Self::Top => "top",
            Self::Bottom => "bottom",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Machine {
    Local,
    Peer(IpAddr),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Slot {
    pub column: u8,
    pub row: u8,
}

impl Slot {
    pub fn new(column: u8, row: u8) -> Option<Self> {
        (column < 2 && row < 2).then_some(Self { column, row })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TopologyError {
    InvalidFormat,
    Occupied,
    TooManyMachines,
    MissingMachine,
    NotAdjacent,
}

impl fmt::Display for TopologyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFormat => write!(f, "invalid topology format"),
            Self::Occupied => write!(f, "slot is occupied by a different machine"),
            Self::TooManyMachines => write!(f, "topology supports at most four machines"),
            Self::MissingMachine => write!(f, "machine is not placed in the topology"),
            Self::NotAdjacent => write!(f, "machines are not adjacent"),
        }
    }
}

impl std::error::Error for TopologyError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Topology {
    positions: BTreeMap<Machine, Slot>,
}

impl Default for Topology {
    fn default() -> Self {
        let mut positions = BTreeMap::new();
        positions.insert(Machine::Local, Slot { column: 0, row: 0 });
        Self { positions }
    }
}

impl Topology {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn positions(&self) -> impl Iterator<Item = (Machine, Slot)> + '_ {
        self.positions
            .iter()
            .map(|(machine, slot)| (*machine, *slot))
    }

    pub fn place(&mut self, machine: Machine, slot: Slot) -> Result<(), TopologyError> {
        let old = self.positions.get(&machine).copied();
        if old == Some(slot) {
            return Ok(());
        }
        if old.is_none() && self.positions.len() >= 4 {
            return Err(TopologyError::TooManyMachines);
        }
        if let Some((&occupant, _)) = self
            .positions
            .iter()
            .find(|(other, value)| **other != machine && **value == slot)
        {
            let previous = old.ok_or(TopologyError::Occupied)?;
            self.positions.insert(occupant, previous);
        }
        self.positions.insert(machine, slot);
        Ok(())
    }

    pub fn remove_peer(&mut self, address: IpAddr) {
        self.positions.remove(&Machine::Peer(address));
    }

    pub fn edge_to(&self, address: IpAddr) -> Result<Edge, TopologyError> {
        let local = self
            .positions
            .get(&Machine::Local)
            .ok_or(TopologyError::MissingMachine)?;
        let peer = self
            .positions
            .get(&Machine::Peer(address))
            .ok_or(TopologyError::MissingMachine)?;
        match (
            peer.column as i8 - local.column as i8,
            peer.row as i8 - local.row as i8,
        ) {
            (-1, 0) => Ok(Edge::Left),
            (1, 0) => Ok(Edge::Right),
            (0, -1) => Ok(Edge::Top),
            (0, 1) => Ok(Edge::Bottom),
            _ => Err(TopologyError::NotAdjacent),
        }
    }

    pub fn encode(&self) -> String {
        let mut out = String::from("SCTO0001\n");
        for (machine, slot) in self.positions() {
            match machine {
                Machine::Local => out.push_str(&format!("local\t{}\t{}\n", slot.column, slot.row)),
                Machine::Peer(address) => {
                    out.push_str(&format!("peer\t{address}\t{}\t{}\n", slot.column, slot.row))
                }
            }
        }
        out
    }

    pub fn decode(raw: &str) -> Result<Self, TopologyError> {
        if raw.len() > 512 {
            return Err(TopologyError::InvalidFormat);
        }
        let mut lines = raw.lines();
        if lines.next() != Some("SCTO0001") {
            return Err(TopologyError::InvalidFormat);
        }
        let mut positions = BTreeMap::new();
        for line in lines {
            let parts: Vec<&str> = line.split('\t').collect();
            let (machine, column, row) = match parts.as_slice() {
                ["local", column, row] => (Machine::Local, *column, *row),
                ["peer", address, column, row] => (
                    Machine::Peer(address.parse().map_err(|_| TopologyError::InvalidFormat)?),
                    *column,
                    *row,
                ),
                _ => return Err(TopologyError::InvalidFormat),
            };
            let column = column.parse().map_err(|_| TopologyError::InvalidFormat)?;
            let row = row.parse().map_err(|_| TopologyError::InvalidFormat)?;
            let slot = Slot::new(column, row).ok_or(TopologyError::InvalidFormat)?;
            if positions.insert(machine, slot).is_some()
                || positions.values().filter(|value| **value == slot).count() > 1
            {
                return Err(TopologyError::InvalidFormat);
            }
            if positions.len() > 4 {
                return Err(TopologyError::TooManyMachines);
            }
        }
        if !positions.contains_key(&Machine::Local) {
            return Err(TopologyError::InvalidFormat);
        }
        Ok(Self { positions })
    }
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

    #[test]
    fn barriers_cover_exterior_not_seams_of_staggered_monitors() {
        let regions = [
            Rect::new(-100, 0, 100, 100).unwrap(),
            Rect::new(0, 50, 100, 100).unwrap(),
        ];
        assert_eq!(
            external_barriers(&regions, Edge::Right),
            vec![
                BarrierSegment {
                    x1: 0,
                    y1: 0,
                    x2: 0,
                    y2: 49
                },
                BarrierSegment {
                    x1: 100,
                    y1: 50,
                    x2: 100,
                    y2: 149
                },
            ]
        );
        assert_eq!(
            external_barriers(&regions, Edge::Left),
            vec![
                BarrierSegment {
                    x1: -100,
                    y1: 0,
                    x2: -100,
                    y2: 99
                },
                BarrierSegment {
                    x1: 0,
                    y1: 100,
                    x2: 0,
                    y2: 149
                },
            ]
        );
    }

    #[test]
    fn barrier_splits_around_a_narrow_adjacent_monitor() {
        let regions = [
            Rect::new(0, 0, 100, 200).unwrap(),
            Rect::new(100, 50, 100, 50).unwrap(),
        ];
        assert_eq!(
            external_barriers(&regions, Edge::Right),
            vec![
                BarrierSegment {
                    x1: 100,
                    y1: 0,
                    x2: 100,
                    y2: 49
                },
                BarrierSegment {
                    x1: 100,
                    y1: 100,
                    x2: 100,
                    y2: 199
                },
                BarrierSegment {
                    x1: 200,
                    y1: 50,
                    x2: 200,
                    y2: 99
                },
            ]
        );
        assert!(external_barriers(&[], Edge::Right).is_empty());
    }

    #[test]
    fn four_local_monitors_leave_only_the_outer_border() {
        let regions: Vec<Rect> = [(0, 0), (100, 0), (0, 100), (100, 100)]
            .into_iter()
            .map(|(x, y)| Rect::new(x, y, 100, 100).unwrap())
            .collect();
        assert_eq!(
            external_barriers(&regions, Edge::Right),
            vec![
                BarrierSegment {
                    x1: 200,
                    y1: 0,
                    x2: 200,
                    y2: 99
                },
                BarrierSegment {
                    x1: 200,
                    y1: 100,
                    x2: 200,
                    y2: 199
                },
            ]
        );
        assert_eq!(
            external_barriers(&regions, Edge::Top),
            vec![
                BarrierSegment {
                    x1: 0,
                    y1: 0,
                    x2: 99,
                    y2: 0
                },
                BarrierSegment {
                    x1: 100,
                    y1: 0,
                    x2: 199,
                    y2: 0
                },
            ]
        );
    }

    #[test]
    fn machine_slots_swap_and_define_an_edge() {
        let mut topology = Topology::new();
        let a: IpAddr = "192.168.1.2".parse().unwrap();
        let b: IpAddr = "192.168.1.3".parse().unwrap();
        topology
            .place(Machine::Peer(a), Slot::new(1, 0).unwrap())
            .unwrap();
        topology
            .place(Machine::Peer(b), Slot::new(0, 1).unwrap())
            .unwrap();
        assert_eq!(topology.edge_to(a).unwrap(), Edge::Right);
        topology
            .place(Machine::Local, Slot::new(1, 0).unwrap())
            .unwrap();
        assert_eq!(topology.edge_to(a).unwrap(), Edge::Left);
        let encoded = topology.encode();
        assert_eq!(Topology::decode(&encoded).unwrap(), topology);
    }

    #[test]
    fn topology_rejects_occupied_new_peer_and_bad_files() {
        let mut topology = Topology::new();
        let a: IpAddr = "192.168.1.2".parse().unwrap();
        assert_eq!(
            topology.place(Machine::Peer(a), Slot::new(0, 0).unwrap()),
            Err(TopologyError::Occupied)
        );
        assert!(Topology::decode("SCTO0001\nlocal\t0\t0\npeer\t192.168.1.2\t0\t0\n").is_err());
        assert!(Topology::decode("SCTO0001\npeer\t192.168.1.2\t1\t0\n").is_err());
    }
}
