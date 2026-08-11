//! ncubed-client — a Neptune window showing a flat plane with one cube per
//! player logged in to the server (cubed00.md §2.1). WASD moves your cube;
//! every other cube tracks the other players.
//!
//! Run `ncubed-server` first, then one `ncubed-client` per player:
//!
//! ```text
//! ncubed-server          # or ncubed-server 0.0.0.0:9000
//! ncubed-client          # or ncubed-client 127.0.0.1:9000
//! ```

use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant};

use ncubed_common::{Packet, PlayerId, PlayerState};
use neptune::prelude::*;

const SERVER_ADDR: &str = "127.0.0.1:8192";
const SPEED: f32 = 150.0; // reference feel: speed = 150
const DAMP: f32 = 10.0; // reference feel: velocity damped by 10 * dt
const SEND_HZ: f32 = 30.0;
const ARENA_HALF: f32 = 15.0; // clamp cubes to +/-15 so they stay on the plane
const POOL: usize = 64; // cubed00.md §5.1 G3: no Scene::remove -> pool + visible toggle

type Cube = Mesh<BufferGeometry<SimpleVertex>, MeshBasicMaterial>;

fn main() {
    let server_addr = std::env::args()
        .nth(1)
        .unwrap_or_else(|| SERVER_ADDR.to_string());

    let (to_net, net_in) = channel::<Packet>();
    let (net_out, from_net) = channel::<Packet>();
    std::thread::spawn(move || net_thread(server_addr, net_in, net_out));

    let mut renderer = Renderer::new(RendererOptions {
        width: 1280,
        height: 720,
        title: "ncubed — you are a cube",
    });

    let mut scene = Scene::new();
    scene.background = Color::hex(0x14141c);
    scene.add(Mesh::new(
        PlaneGeometry::new(ARENA_HALF * 2.0, ARENA_HALF * 2.0),
        MeshBasicMaterial::new(Color::hex(0x2a2f3a)),
    ));

    // The cube pool: every slot pre-created and hidden, revealed on join,
    // re-hidden on leave. Cube ids never move, so the client never needs
    // `Scene::remove` — the workaround cubed00.md §5.1 G3 calls for.
    let mut slots: Vec<Option<PlayerId>> = Vec::with_capacity(POOL);
    let mut slot_of: HashMap<PlayerId, usize> = HashMap::new();
    let mut cube_ids: Vec<ObjectId> = Vec::with_capacity(POOL);
    for _ in 0..POOL {
        let mut cube = Mesh::new(BoxGeometry::cube(1.0), MeshBasicMaterial::new(Color::WHITE));
        cube.visible = false;
        cube_ids.push(scene.add(cube));
        slots.push(None);
    }

    let mut my_id: Option<PlayerId> = None;
    let mut pos = Vec2::ZERO;
    let mut vel = Vec2::ZERO;
    let mut last_send = Instant::now() - Duration::from_millis(100); // say hello on frame one

    let mut camera = PerspectiveCamera::new(60.0_f32.to_radians(), 1280.0 / 720.0, 0.1, 200.0);
    camera.transform.position = Vec3::new(0.0, 14.0, 14.0);
    camera.look_at(Vec3::ZERO, Vec3::Y);

    renderer.render_loop(move |frame| {
        if frame.input().just_pressed(KeyCode::Escape) {
            frame.exit();
        }

        // --- Own movement: WASD, velocity damped like the reference game. ---
        let dt = frame.delta_seconds();
        let mut wish = Vec2::ZERO;
        if frame.input().held(KeyCode::KeyW) {
            wish.y += 1.0;
        }
        if frame.input().held(KeyCode::KeyS) {
            wish.y -= 1.0;
        }
        if frame.input().held(KeyCode::KeyA) {
            wish.x -= 1.0;
        }
        if frame.input().held(KeyCode::KeyD) {
            wish.x += 1.0;
        }
        if wish != Vec2::ZERO {
            wish = wish.normalize();
        }
        vel = vel * (1.0 - (DAMP * dt).min(1.0)) + wish * SPEED;
        pos += vel * dt;
        pos.x = pos.x.clamp(-ARENA_HALF, ARENA_HALF);
        pos.y = pos.y.clamp(-ARENA_HALF, ARENA_HALF);

        // --- Reconcile the latest server snapshot (latest-wins). ---
        while let Ok(packet) = from_net.try_recv() {
            match packet {
                Packet::Welcome(id) => {
                    my_id = Some(id);
                    println!("connected - you are player {id}");
                }
                Packet::Players { players } => {
                    // A player who vanished no longer owns a slot.
                    for slot in slots.iter_mut() {
                        *slot = None;
                    }
                    for &cube_id in &cube_ids {
                        scene
                            .get_mut_as::<Cube>(cube_id)
                            .expect("pooled cube exists")
                            .visible = false;
                    }
                    for (id, st) in players {
                        // Reuse the slot this player had last time if it is free.
                        let slot = match slot_of.get(&id) {
                            Some(s) if slots[*s].is_none() => *s,
                            _ => {
                                let free = slots
                                    .iter()
                                    .position(|s| s.is_none())
                                    .expect("more players than pool slots");
                                slot_of.insert(id, free);
                                free
                            }
                        };
                        slots[slot] = Some(id);
                        let cube = scene.get_mut_as::<Cube>(cube_ids[slot]).unwrap();
                        cube.visible = true;
                        cube.transform.position = Vec3::new(st.position.x, 0.5, st.position.y);
                        cube.material.color = color_for(id, my_id);
                    }
                }
            }
        }

        // --- Send our state at ~30 Hz. ---
        if last_send.elapsed().as_secs_f32() >= 1.0 / SEND_HZ {
            let _ = to_net.send(Packet::Players {
                players: vec![(0, PlayerState::new(pos, vel))],
            });
            last_send = Instant::now();
        }

        camera.aspect = frame.aspect_ratio();
        frame.render(&scene, &camera);
    });
}

/// Your cube is magenta; everyone else cycles a small palette so players are
/// distinguishable at a glance.
fn color_for(id: PlayerId, my_id: Option<PlayerId>) -> Color {
    if my_id == Some(id) {
        return Color::hex(0xff00ff);
    }
    const PALETTE: [u32; 6] = [0x9fb4ff, 0x4ade80, 0xfbbf24, 0xf87171, 0x38bdf8, 0xc084fc];
    Color::hex(PALETTE[id as usize % PALETTE.len()])
}

/// One network thread owns the socket; the render loop never touches it.
/// Outbound state flows in over `net_in`, inbound packets come back out on
/// `net_out` — channels, no locks on the render path (cubed00.md §3.4).
fn net_thread(server_addr: String, net_in: Receiver<Packet>, net_out: Sender<Packet>) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("a small single-threaded runtime for the socket");
    runtime.block_on(async move {
        let std_socket = std::net::UdpSocket::bind("0.0.0.0:0").expect("ephemeral local socket");
        std_socket
            .set_nonblocking(true)
            .expect("tokio needs a non-blocking socket");
        let addr: std::net::SocketAddr = server_addr
            .parse()
            .expect("server address must be host:port, e.g. 127.0.0.1:8192");
        let socket = tokio::net::UdpSocket::from_std(std_socket)
            .expect("tokio takes ownership of the socket");
        let mut buf = vec![0u8; 2048];

        loop {
            // Anything the render loop asked us to send goes out first.
            while let Ok(packet) = net_in.try_recv() {
                let bytes = ncubed_common::encode(&packet);
                let _ = socket.send_to(&bytes, addr).await;
            }
            // Block up to a millisecond for a reply, then loop: incoming
            // snapshots are latest-wins, so this never builds up back-pressure.
            let incoming = tokio::time::timeout(Duration::from_millis(1), socket.recv_from(&mut buf));
            if let Ok(Ok((n, _from))) = incoming.await
                && let Ok(packet) = ncubed_common::decode(&buf[..n])
            {
                let _ = net_out.send(packet);
            }
        }
    });
}
