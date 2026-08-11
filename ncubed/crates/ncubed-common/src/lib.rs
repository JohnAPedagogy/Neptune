//! The wire format and shared types for ncubed — the only crate both the
//! server and the client depend on, and the only place the on-the-wire
//! `Packet` enum lives (cubed00.md §3.1, §8.2, §8.4).

use serde::{Deserialize, Serialize};

/// The 2D vector used for positions and velocities, re-exported so server and
/// client never need a direct `glam` dependency.
pub use glam::Vec2;

/// A player's identity on the wire. Assigned by the server, starting at 1.
pub type PlayerId = u32;

/// Where a player's cube is, and how it is moving. 2D: the cube's world
/// position is `(x, 0.5, y)` — sitting on the plane — and its velocity is
/// `(vx, vy)` in the same plane.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PlayerState {
    pub position: Vec2,
    pub velocity: Vec2,
}

impl PlayerState {
    pub fn new(position: Vec2, velocity: Vec2) -> Self {
        PlayerState { position, velocity }
    }
}

/// The entire ncubed wire protocol — two packets, one UDP datagram each.
///
/// - `Welcome` — server -> client, in reply to a client's first datagram. It
///   tells the client which id this socket is known by.
/// - `Players` — client -> server (this client's own state) and server ->
///   client (a full snapshot of every known player).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Packet {
    Welcome(PlayerId),
    Players { players: Vec<(PlayerId, PlayerState)> },
}

/// Serialises a packet into bytes for one datagram.
pub fn encode(packet: &Packet) -> Vec<u8> {
    postcard::to_allocvec(packet)
        .expect("ncubed packets always fit postcard's allocation budget")
}

/// Deserialises one datagram back into a [`Packet`].
pub fn decode(bytes: &[u8]) -> Result<Packet, postcard::Error> {
    postcard::from_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> PlayerState {
        PlayerState::new(Vec2::new(1.5, -2.0), Vec2::new(10.0, 0.0))
    }

    #[test]
    fn welcome_round_trips() {
        let packet = Packet::Welcome(7);
        assert_eq!(decode(&encode(&packet)).unwrap(), packet);
    }

    #[test]
    fn snapshot_round_trips() {
        let packet = Packet::Players {
            players: vec![
                (1, state()),
                (2, PlayerState::new(Vec2::new(0.0, 0.0), Vec2::ZERO)),
            ],
        };
        assert_eq!(decode(&encode(&packet)).unwrap(), packet);
    }

    #[test]
    fn empty_snapshot_round_trips() {
        let packet = Packet::Players { players: Vec::new() };
        assert_eq!(decode(&encode(&packet)).unwrap(), packet);
    }

    #[test]
    fn a_truncated_datagram_is_a_decode_error() {
        let bytes = encode(&Packet::Players {
            players: vec![(1, state())],
        });
        let truncated = &bytes[..bytes.len() / 2];
        assert!(decode(truncated).is_err());
    }

    #[test]
    fn garbage_is_a_decode_error() {
        assert!(decode(&[0xFF, 0x00, 0x99]).is_err());
    }
}
