//! Ownership policy for switching the physical origin between remote desktops.
//! Network adapters must acquire the next peer before requesting RELEASE, and
//! may send BEGIN to it only after ENDED is authenticated from the old peer.

use crate::topology::{Edge, Machine, Topology};
use std::fmt;
use std::net::IpAddr;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HandoffError {
    NotLocalNeighbor,
    NoActivePeer,
    WrongPeer,
    WrongEpoch,
    NotAdjacent,
    AlreadyPending,
    NoPendingRelease,
}

impl fmt::Display for HandoffError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid input handoff: {self:?}")
    }
}

impl std::error::Error for HandoffError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PendingRelease {
    pub from: IpAddr,
    pub epoch: u64,
    pub target: Machine,
}

pub struct HandoffCoordinator {
    topology: Topology,
    owner: Machine,
    epoch: Option<u64>,
    pending: Option<PendingRelease>,
}

impl HandoffCoordinator {
    pub fn new(topology: Topology) -> Self {
        Self {
            topology,
            owner: Machine::Local,
            epoch: None,
            pending: None,
        }
    }

    pub fn owner(&self) -> Machine {
        self.owner
    }

    pub fn epoch(&self) -> Option<u64> {
        self.epoch
    }

    pub fn pending(&self) -> Option<PendingRelease> {
        self.pending
    }

    pub fn activate_local_edge(
        &mut self,
        edge: Edge,
        peer: IpAddr,
        epoch: u64,
    ) -> Result<(), HandoffError> {
        if self.owner != Machine::Local || self.pending.is_some() {
            return Err(HandoffError::NoActivePeer);
        }
        if epoch == 0 {
            return Err(HandoffError::WrongEpoch);
        }
        if self.topology.neighbor(Machine::Local, edge) != Some(Machine::Peer(peer)) {
            return Err(HandoffError::NotLocalNeighbor);
        }
        self.owner = Machine::Peer(peer);
        self.epoch = Some(epoch);
        Ok(())
    }

    /// A request is valid only from the current destination, for its current
    /// epoch, and to one of its orthogonal neighbors in the shared grid.
    pub fn request(
        &mut self,
        from: IpAddr,
        epoch: u64,
        target: Machine,
    ) -> Result<PendingRelease, HandoffError> {
        if self.pending.is_some() {
            return Err(HandoffError::AlreadyPending);
        }
        if self.owner == Machine::Local {
            return Err(HandoffError::NoActivePeer);
        }
        if self.owner != Machine::Peer(from) {
            return Err(HandoffError::WrongPeer);
        }
        if self.epoch != Some(epoch) {
            return Err(HandoffError::WrongEpoch);
        }
        if ![Edge::Left, Edge::Right, Edge::Top, Edge::Bottom]
            .into_iter()
            .any(|edge| self.topology.neighbor(Machine::Peer(from), edge) == Some(target))
        {
            return Err(HandoffError::NotAdjacent);
        }
        let pending = PendingRelease {
            from,
            epoch,
            target,
        };
        self.pending = Some(pending);
        Ok(pending)
    }

    /// Only the old peer's acknowledgement permits a new owner. The caller
    /// sends BEGIN for `target` with `next_epoch` after this succeeds.
    pub fn acknowledge(
        &mut self,
        from: IpAddr,
        epoch: u64,
        next_epoch: u64,
    ) -> Result<Machine, HandoffError> {
        let pending = self.pending.ok_or(HandoffError::NoPendingRelease)?;
        if pending.from != from {
            return Err(HandoffError::WrongPeer);
        }
        if pending.epoch != epoch || next_epoch == 0 || next_epoch == epoch {
            return Err(HandoffError::WrongEpoch);
        }
        self.owner = pending.target;
        self.epoch = (self.owner != Machine::Local).then_some(next_epoch);
        self.pending = None;
        Ok(self.owner)
    }

    /// A transport failure returns ownership to the physical source. The
    /// network adapter must also release the portal before using this path.
    pub fn reset_local(&mut self) {
        self.owner = Machine::Local;
        self.epoch = None;
        self.pending = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topology::Slot;

    fn ip(last: u8) -> IpAddr {
        IpAddr::from([192, 168, 1, last])
    }

    fn square() -> Topology {
        let mut topology = Topology::new();
        topology
            .place(Machine::Peer(ip(2)), Slot::new(1, 0).unwrap())
            .unwrap();
        topology
            .place(Machine::Peer(ip(3)), Slot::new(1, 1).unwrap())
            .unwrap();
        topology
            .place(Machine::Peer(ip(4)), Slot::new(0, 1).unwrap())
            .unwrap();
        topology
    }

    #[test]
    fn four_machine_handoff_waits_for_the_previous_release() {
        let mut control = HandoffCoordinator::new(square());
        control.activate_local_edge(Edge::Right, ip(2), 10).unwrap();
        control.request(ip(2), 10, Machine::Peer(ip(3))).unwrap();
        assert_eq!(control.owner(), Machine::Peer(ip(2)));
        assert_eq!(control.epoch(), Some(10));
        assert_eq!(
            control.acknowledge(ip(3), 10, 11),
            Err(HandoffError::WrongPeer)
        );
        assert_eq!(
            control.acknowledge(ip(2), 9, 11),
            Err(HandoffError::WrongEpoch)
        );
        assert_eq!(control.owner(), Machine::Peer(ip(2)));
        assert_eq!(control.acknowledge(ip(2), 10, 11), Ok(Machine::Peer(ip(3))));
        control.request(ip(3), 11, Machine::Peer(ip(4))).unwrap();
        assert_eq!(control.acknowledge(ip(3), 11, 12), Ok(Machine::Peer(ip(4))));
        control.request(ip(4), 12, Machine::Local).unwrap();
        assert_eq!(control.acknowledge(ip(4), 12, 13), Ok(Machine::Local));
        assert_eq!(control.epoch(), None);
    }

    #[test]
    fn stale_or_non_neighbor_requests_never_create_another_owner() {
        let mut control = HandoffCoordinator::new(square());
        assert_eq!(
            control.activate_local_edge(Edge::Right, ip(3), 1),
            Err(HandoffError::NotLocalNeighbor)
        );
        control.activate_local_edge(Edge::Right, ip(2), 2).unwrap();
        assert_eq!(
            control.request(ip(3), 2, Machine::Peer(ip(4))),
            Err(HandoffError::WrongPeer)
        );
        assert_eq!(
            control.request(ip(2), 1, Machine::Peer(ip(3))),
            Err(HandoffError::WrongEpoch)
        );
        assert_eq!(
            control.request(ip(2), 2, Machine::Peer(ip(4))),
            Err(HandoffError::NotAdjacent)
        );
        control.request(ip(2), 2, Machine::Peer(ip(3))).unwrap();
        assert_eq!(
            control.request(ip(2), 2, Machine::Peer(ip(3))),
            Err(HandoffError::AlreadyPending)
        );
        control.reset_local();
        assert_eq!(control.owner(), Machine::Local);
        assert_eq!(control.pending(), None);
        assert_eq!(
            control.acknowledge(ip(2), 2, 3),
            Err(HandoffError::NoPendingRelease)
        );
    }
}
