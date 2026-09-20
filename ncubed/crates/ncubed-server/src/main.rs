//! ncubed-server — headless, no window, no Neptune. Binds a UDP socket, hands
//! each new caller a player id (`Welcome`), and broadcasts the full player
//! snapshot every ~30 Hz (cubed00.md §3.2). Players silent for ~3 s are
//! dropped, which is what makes a cube disappear when its client goes away.

use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

use ncubed_common::{Packet, PlayerId, PlayerState};
use tokio::net::UdpSocket;
use tokio::time::interval;

const TICK: Duration = Duration::from_millis(33); // ~30 Hz
const STALE: Duration = Duration::from_secs(3);   // drop silent players after ~3 s
const BUF: usize = 2048;
const DEFAULT_BIND: &str = "0.0.0.0:8192";

struct Peer {
    addr: SocketAddr,
    state: PlayerState,
    last_seen: Instant,
}

/// The entire server, held in `main`'s loop. Split into methods so the state
/// machine is testable without any socket.
struct Server {
    next_id: PlayerId,
    peers: HashMap<PlayerId, Peer>,
}

impl Server {
    fn new() -> Self {
        Server {
            next_id: 1,
            peers: HashMap::new(),
        }
    }

    /// Records one client's position update. The first datagram from an
    /// unknown address assigns a fresh id and returns `Some(id)` so the caller
    /// can welcome the new player; later datagrams from the same address just
    /// update its state and return `None`.
    fn record(&mut self, from: SocketAddr, state: PlayerState) -> Option<PlayerId> {
        let known = self.peers.iter().find(|(_, p)| p.addr == from).map(|(id, _)| *id);
        match known {
            Some(id) => {
                let peer = self.peers.get_mut(&id).expect("found peer must exist");
                peer.state = state;
                peer.last_seen = Instant::now();
                None
            }
            None => {
                let id = self.next_id;
                self.next_id += 1;
                self.peers.insert(
                    id,
                    Peer {
                        addr: from,
                        state,
                        last_seen: Instant::now(),
                    },
                );
                Some(id)
            }
        }
    }

    /// The full snapshot broadcast every tick.
    fn snapshot(&self) -> Packet {
        Packet::Players {
            players: self.peers.iter().map(|(id, p)| (*id, p.state)).collect(),
        }
    }

    /// Drops every player whose last datagram is older than `STALE`.
    fn prune(&mut self, now: Instant) {
        self.peers.retain(|_, p| now.duration_since(p.last_seen) < STALE);
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let bind = std::env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_BIND.to_string());
    let socket = UdpSocket::bind(&bind).await?;
    println!("ncubed-server listening on {bind}");
    run(socket, None).await;
    Ok(())
}

/// The whole server loop. Split out of `main` so a test can drive it against a
/// real loopback socket. `ticks` bounds the loop for tests; `None` runs
/// forever, which is what production wants.
async fn run(socket: UdpSocket, ticks: Option<u64>) {
    let mut server = Server::new();
    let mut tick = interval(TICK);
    let mut buf = vec![0u8; BUF];
    let mut done = 0u64;

    loop {
        tick.tick().await;
        if ticks.is_some_and(|limit| done >= limit) {
            return;
        }
        done += 1;

        // Drain every inbound datagram since the last tick.
        while let Ok((n, from)) = socket.try_recv_from(&mut buf) {
            if let Ok(Packet::Players { players }) = ncubed_common::decode(&buf[..n])
                && let Some((_, state)) = players.first()
                && let Some(id) = server.record(from, *state)
            {
                let welcome = ncubed_common::encode(&Packet::Welcome(id));
                let _ = socket.send_to(&welcome, from).await;
            }
        }

        server.prune(Instant::now());

        // Broadcast the whole snapshot to every live peer, one datagram each.
        let bytes = ncubed_common::encode(&server.snapshot());
        let addrs: Vec<SocketAddr> = server.peers.values().map(|p| p.addr).collect();
        for addr in addrs {
            let _ = socket.send_to(&bytes, addr).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ncubed_common::Vec2;

    fn addr(port: u16) -> SocketAddr {
        SocketAddr::from(([127, 0, 0, 1], port))
    }

    fn state() -> PlayerState {
        PlayerState::new(Vec2::new(1.0, 2.0), Vec2::ZERO)
    }

    #[test]
    fn first_datagram_from_an_address_gets_a_fresh_id() {
        let mut server = Server::new();
        let id = server.record(addr(4000), state()).expect("new peer welcomed");
        assert_eq!(id, 1);
        assert_eq!(server.peers.len(), 1);
    }

    #[test]
    fn the_same_address_reuses_its_id() {
        let mut server = Server::new();
        server.record(addr(4000), state());
        let again = server.record(
            addr(4000),
            PlayerState::new(Vec2::new(5.0, 5.0), Vec2::ZERO),
        );
        assert!(again.is_none(), "known peer is updated, not re-welcomed");
        assert_eq!(server.peers.len(), 1);
        let peer = server.peers.values().next().unwrap();
        assert_eq!(peer.state.position, Vec2::new(5.0, 5.0));
    }

    #[test]
    fn distinct_addresses_get_distinct_ids() {
        let mut server = Server::new();
        let a = server.record(addr(4000), state()).unwrap();
        let b = server.record(addr(4001), state()).unwrap();
        assert_ne!(a, b);
        assert_eq!(server.peers.len(), 2);
    }

    #[test]
    fn snapshot_contains_every_peer() {
        let mut server = Server::new();
        server.record(addr(4000), state());
        server.record(addr(4001), state());
        let Packet::Players { players } = server.snapshot() else {
            panic!("snapshot is a Players packet");
        };
        assert_eq!(players.len(), 2);
    }

    #[test]
    fn silent_peers_are_pruned() {
        let mut server = Server::new();
        server.record(addr(4000), state());
        let now = Instant::now() + STALE + Duration::from_millis(1);
        server.prune(now);
        assert!(server.peers.is_empty());
    }

    #[test]
    fn a_peer_that_just_spoke_is_not_pruned() {
        let mut server = Server::new();
        server.record(addr(4000), state());
        server.prune(Instant::now());
        assert_eq!(server.peers.len(), 1);
    }

    #[tokio::test]
    async fn a_client_is_welcomed_then_gets_snapshots_over_udp() {
        use ncubed_common::{decode, encode, Packet, PlayerState, Vec2};
        use tokio::time::timeout;

        let server_socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let server_addr = server_socket.local_addr().unwrap();
        let client = UdpSocket::bind("127.0.0.1:0").await.unwrap();

        let server_task = tokio::spawn(async move {
            run(server_socket, Some(2)).await;
        });

        // First datagram from an unknown address: the server should welcome us.
        let hello = encode(&Packet::Players {
            players: vec![(0, PlayerState::new(Vec2::ZERO, Vec2::ZERO))],
        });
        client.send_to(&hello, server_addr).await.unwrap();

        let mut buf = vec![0u8; 2048];
        let (n, _) = timeout(Duration::from_secs(2), client.recv_from(&mut buf))
            .await
            .expect("a welcome arrives")
            .unwrap();
        assert!(matches!(decode(&buf[..n]).unwrap(), Packet::Welcome(1)));

        // And every tick broadcasts a full snapshot that includes us.
        let (n, _) = timeout(Duration::from_secs(2), client.recv_from(&mut buf))
            .await
            .expect("a snapshot arrives")
            .unwrap();
        let Packet::Players { players } = decode(&buf[..n]).unwrap() else {
            panic!("second packet is a snapshot");
        };
        assert_eq!(players.len(), 1);
        assert_eq!(players[0].0, 1);

        server_task.await.unwrap();
    }
}
