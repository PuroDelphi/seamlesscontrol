//! Logical desktop geometry. The transport and platform adapters can map
//! physical pixels to these coordinates before requesting an edge crossing.

use std::collections::{BTreeMap, VecDeque};
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

pub struct EdgeReturnDetector {
    edge: Edge,
    segments: Vec<BarrierSegment>,
    away_seen: bool,
    fired: bool,
}

impl EdgeReturnDetector {
    pub fn new(regions: &[Rect], edge: Edge) -> Self {
        Self {
            edge,
            segments: external_barriers(regions, edge),
            away_seen: false,
            fired: false,
        }
    }

    /// Arm only after the remote cursor has moved into the destination.
    /// A new epoch starting at an edge must not bounce straight back.
    pub fn sample(&mut self, x: i32, y: i32) -> bool {
        if self.fired {
            return false;
        }
        let distance = self
            .segments
            .iter()
            .filter_map(|segment| match self.edge {
                Edge::Left | Edge::Right if y >= segment.y1 && y <= segment.y2 => {
                    Some((i64::from(x) - i64::from(segment.x1)).abs())
                }
                Edge::Top | Edge::Bottom if x >= segment.x1 && x <= segment.x2 => {
                    Some((i64::from(y) - i64::from(segment.y1)).abs())
                }
                _ => None,
            })
            .min();
        match distance {
            Some(value) if value > 16 => self.away_seen = true,
            Some(value) if self.away_seen && value <= 2 => {
                self.fired = true;
                return true;
            }
            _ => {}
        }
        false
    }
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

    pub fn opposite(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
            Self::Top => Self::Bottom,
            Self::Bottom => Self::Top,
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "left" => Some(Self::Left),
            "right" => Some(Self::Right),
            "top" => Some(Self::Top),
            "bottom" => Some(Self::Bottom),
            _ => None,
        }
    }
}

fn ordered_exterior(regions: &[Rect], edge: Edge) -> Vec<BarrierSegment> {
    let mut segments = external_barriers(regions, edge);
    segments.sort_unstable_by_key(|segment| match edge {
        Edge::Left | Edge::Right => (segment.y1, segment.x1),
        Edge::Top | Edge::Bottom => (segment.x1, segment.y1),
    });
    segments
}

fn edge_span(segment: BarrierSegment, edge: Edge) -> (i32, i32) {
    match edge {
        Edge::Left | Edge::Right => (segment.y1, segment.y2),
        Edge::Top | Edge::Bottom => (segment.x1, segment.x2),
    }
}

/// Position along the exposed side of all local monitors, preserving the
/// crossing height/width when the neighbor has a different resolution.
pub fn edge_fraction(regions: &[Rect], edge: Edge, x: i32, y: i32) -> Option<u16> {
    let segments = ordered_exterior(regions, edge);
    let total: i64 = segments
        .iter()
        .map(|segment| {
            let (start, end) = edge_span(*segment, edge);
            i64::from(end) - i64::from(start) + 1
        })
        .sum();
    if total == 0 {
        return None;
    }
    let along = match edge {
        Edge::Left | Edge::Right => y,
        Edge::Top | Edge::Bottom => x,
    };
    let across = match edge {
        Edge::Left | Edge::Right => x,
        Edge::Top | Edge::Bottom => y,
    };
    let chosen = segments.iter().enumerate().min_by_key(|(_, segment)| {
        let (start, end) = edge_span(**segment, edge);
        let along_distance = i64::from(along).saturating_sub(i64::from(end)).max(0)
            + i64::from(start).saturating_sub(i64::from(along)).max(0);
        let boundary = match edge {
            Edge::Left | Edge::Right => segment.x1,
            Edge::Top | Edge::Bottom => segment.y1,
        };
        (
            along_distance,
            (i64::from(across) - i64::from(boundary)).abs(),
        )
    })?;
    let before: i64 = segments[..chosen.0]
        .iter()
        .map(|segment| {
            let (start, end) = edge_span(*segment, edge);
            i64::from(end) - i64::from(start) + 1
        })
        .sum();
    let (start, end) = edge_span(*chosen.1, edge);
    let offset = i64::from(along.clamp(start, end)) - i64::from(start);
    let numerator = (before + offset) * i64::from(u16::MAX);
    Some((numerator / (total - 1).max(1)) as u16)
}

/// A point just inside the requested destination edge. Returns `None` for
/// incomplete or unsupported monitor geometry rather than guessing a warp.
pub fn edge_entry_point(regions: &[Rect], edge: Edge, fraction: u16) -> Option<(i32, i32)> {
    let segments = ordered_exterior(regions, edge);
    let total: i64 = segments
        .iter()
        .map(|segment| {
            let (start, end) = edge_span(*segment, edge);
            i64::from(end) - i64::from(start) + 1
        })
        .sum();
    if total == 0 {
        return None;
    }
    let mut offset = i64::from(fraction) * (total - 1) / i64::from(u16::MAX);
    for segment in segments {
        let (start, end) = edge_span(segment, edge);
        let length = i64::from(end) - i64::from(start) + 1;
        if offset >= length {
            offset -= length;
            continue;
        }
        let along = i32::try_from(i64::from(start) + offset).ok()?;
        let region = regions.iter().find(|region| match edge {
            Edge::Left => region.x == segment.x1 && along >= region.y && along < region.bottom(),
            Edge::Right => {
                region.right() == segment.x1 && along >= region.y && along < region.bottom()
            }
            Edge::Top => region.y == segment.y1 && along >= region.x && along < region.right(),
            Edge::Bottom => {
                region.bottom() == segment.y1 && along >= region.x && along < region.right()
            }
        })?;
        return Some(match edge {
            Edge::Left => ((region.x.saturating_add(2)).min(region.right() - 1), along),
            Edge::Right => ((region.right().saturating_sub(3)).max(region.x), along),
            Edge::Top => (along, (region.y.saturating_add(2)).min(region.bottom() - 1)),
            Edge::Bottom => (along, (region.bottom().saturating_sub(3)).max(region.y)),
        });
    }
    None
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
    NoRoute,
}

impl fmt::Display for TopologyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFormat => write!(f, "invalid topology format"),
            Self::Occupied => write!(f, "slot is occupied by a different machine"),
            Self::TooManyMachines => write!(f, "topology supports at most four machines"),
            Self::MissingMachine => write!(f, "machine is not placed in the topology"),
            Self::NotAdjacent => write!(f, "machines are not adjacent"),
            Self::NoRoute => write!(f, "no route through placed machines"),
        }
    }
}

impl std::error::Error for TopologyError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Topology {
    positions: BTreeMap<Machine, Slot>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RouteHop {
    pub machine: Machine,
    pub edge: Edge,
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

    pub fn rename_peer(&mut self, old: IpAddr, new: IpAddr) -> Result<bool, TopologyError> {
        if old == new {
            return Ok(false);
        }
        if self.positions.contains_key(&Machine::Peer(new)) {
            return Err(TopologyError::Occupied);
        }
        let Some(slot) = self.positions.remove(&Machine::Peer(old)) else {
            return Ok(false);
        };
        self.positions.insert(Machine::Peer(new), slot);
        Ok(true)
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

    /// Shortest path through occupied, orthogonally adjacent slots. Every hop
    /// is a directly authenticated link; transport handoff is handled elsewhere.
    pub fn route_to(&self, address: IpAddr) -> Result<Vec<RouteHop>, TopologyError> {
        let target = Machine::Peer(address);
        if !self.positions.contains_key(&target) {
            return Err(TopologyError::MissingMachine);
        }
        let mut queue = VecDeque::from([Machine::Local]);
        let mut previous = BTreeMap::<Machine, (Machine, Edge)>::new();
        while let Some(current) = queue.pop_front() {
            if current == target {
                break;
            }
            for edge in [Edge::Left, Edge::Right, Edge::Top, Edge::Bottom] {
                let Some(next) = self.neighbor(current, edge) else {
                    continue;
                };
                if next == Machine::Local || previous.contains_key(&next) {
                    continue;
                }
                previous.insert(next, (current, edge));
                queue.push_back(next);
            }
        }
        if !previous.contains_key(&target) {
            return Err(TopologyError::NoRoute);
        }
        let mut route = Vec::new();
        let mut current = target;
        while current != Machine::Local {
            let (prior, edge) = previous[&current];
            route.push(RouteHop {
                machine: current,
                edge,
            });
            current = prior;
        }
        route.reverse();
        Ok(route)
    }

    pub fn neighbor(&self, machine: Machine, edge: Edge) -> Option<Machine> {
        let slot = self.positions.get(&machine)?;
        let (column, row) = match edge {
            Edge::Left => (slot.column.checked_sub(1)?, slot.row),
            Edge::Right => (slot.column.checked_add(1)?, slot.row),
            Edge::Top => (slot.column, slot.row.checked_sub(1)?),
            Edge::Bottom => (slot.column, slot.row.checked_add(1)?),
        };
        if column >= 2 || row >= 2 {
            return None;
        }
        self.positions.iter().find_map(|(candidate, position)| {
            (position.column == column && position.row == row).then_some(*candidate)
        })
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
    fn edge_position_preserves_relative_height_across_resolutions() {
        let source = [Rect::new(0, 0, 1920, 1080).unwrap()];
        let destination = [Rect::new(0, 0, 1280, 720).unwrap()];
        let fraction = edge_fraction(&source, Edge::Right, 1920, 270).unwrap();
        let point = edge_entry_point(&destination, Edge::Left, fraction).unwrap();
        assert_eq!(point.0, 2);
        assert!((point.1 - 180).abs() <= 1);
        assert_eq!(Edge::Right.opposite(), Edge::Left);
        assert_eq!(Edge::parse("left"), Some(Edge::Left));
        assert_eq!(Edge::parse("LEFT"), None);
    }

    #[test]
    fn entry_point_stays_on_exposed_monitor_edges() {
        let monitors = [
            Rect::new(-800, 100, 800, 600).unwrap(),
            Rect::new(0, 0, 1366, 768).unwrap(),
        ];
        for edge in [Edge::Left, Edge::Right, Edge::Top, Edge::Bottom] {
            for fraction in [0, 10_000, 32_768, 55_000, u16::MAX] {
                let (x, y) = edge_entry_point(&monitors, edge, fraction).unwrap();
                assert!(monitors.iter().any(|region| {
                    x >= region.x && x < region.right() && y >= region.y && y < region.bottom()
                }));
                let recovered = edge_fraction(&monitors, edge, x, y).unwrap();
                assert!((i32::from(recovered) - i32::from(fraction)).abs() <= 200);
            }
        }
    }

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
    fn remote_return_arms_inside_and_fires_once_at_exterior_edge() {
        let regions = [
            Rect::new(0, 0, 100, 100).unwrap(),
            Rect::new(100, 0, 100, 100).unwrap(),
        ];
        let mut detector = EdgeReturnDetector::new(&regions, Edge::Left);
        assert!(!detector.sample(0, 50));
        assert!(!detector.sample(100, 50));
        assert!(!detector.sample(20, 50));
        assert!(detector.sample(1, 50));
        assert!(!detector.sample(1, 50));
    }

    #[test]
    fn return_ignores_internal_seams_and_coordinates_off_the_edge() {
        let regions = [
            Rect::new(0, 0, 100, 100).unwrap(),
            Rect::new(100, 50, 100, 100).unwrap(),
        ];
        let mut detector = EdgeReturnDetector::new(&regions, Edge::Right);
        assert!(!detector.sample(50, 75));
        assert!(!detector.sample(100, 75));
        assert!(!detector.sample(200, 200));
        assert!(detector.sample(199, 75));
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
    fn routing_uses_occupied_neighbors_and_never_jumps_a_diagonal() {
        let mut topology = Topology::new();
        let right: IpAddr = "192.168.1.2".parse().unwrap();
        let bottom: IpAddr = "192.168.1.3".parse().unwrap();
        let diagonal: IpAddr = "192.168.1.4".parse().unwrap();
        topology
            .place(Machine::Peer(diagonal), Slot::new(1, 1).unwrap())
            .unwrap();
        assert_eq!(topology.route_to(diagonal), Err(TopologyError::NoRoute));
        topology
            .place(Machine::Peer(right), Slot::new(1, 0).unwrap())
            .unwrap();
        assert_eq!(
            topology.route_to(diagonal).unwrap(),
            vec![
                RouteHop {
                    machine: Machine::Peer(right),
                    edge: Edge::Right
                },
                RouteHop {
                    machine: Machine::Peer(diagonal),
                    edge: Edge::Bottom
                },
            ]
        );
        topology
            .place(Machine::Peer(bottom), Slot::new(0, 1).unwrap())
            .unwrap();
        assert_eq!(
            topology.route_to(bottom).unwrap(),
            vec![RouteHop {
                machine: Machine::Peer(bottom),
                edge: Edge::Bottom
            },]
        );
        assert_eq!(
            topology.neighbor(Machine::Peer(right), Edge::Left),
            Some(Machine::Local)
        );
        topology.remove_peer(right);
        assert_eq!(
            topology.route_to(diagonal).unwrap(),
            vec![
                RouteHop {
                    machine: Machine::Peer(bottom),
                    edge: Edge::Bottom
                },
                RouteHop {
                    machine: Machine::Peer(diagonal),
                    edge: Edge::Right
                },
            ]
        );
        assert_eq!(topology.edge_to(diagonal), Err(TopologyError::NotAdjacent));
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
