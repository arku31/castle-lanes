use castle_lanes::net::{ClientPacket, PROTOCOL_VERSION, ServerPacket, decode_server, encode};
use castle_lanes::sim::{BuildingKind, GridCell, MatchPhase, RaceKind};
use std::net::{SocketAddr, UdpSocket};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

struct ServerProcess(Child);

impl Drop for ServerProcess {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn dedicated_server_runs_two_player_building_flow() {
    let server_addr = unused_local_addr();
    let _server = spawn_server(server_addr);
    let left = UdpSocket::bind("127.0.0.1:0").unwrap();
    let right = UdpSocket::bind("127.0.0.1:0").unwrap();
    left.set_read_timeout(Some(Duration::from_millis(50)))
        .unwrap();
    right
        .set_read_timeout(Some(Duration::from_millis(50)))
        .unwrap();

    let left_player = join(&left, server_addr, "Alice");
    let right_player = join(&right, server_addr, "Bryn");
    let duplicate = UdpSocket::bind("127.0.0.1:0").unwrap();
    duplicate
        .set_read_timeout(Some(Duration::from_millis(50)))
        .unwrap();
    assert_duplicate_name_rejected(&duplicate, server_addr, "Alice");

    send(
        &left,
        server_addr,
        &ClientPacket::SetRace {
            player_id: left_player.id,
            race: RaceKind::Vanguard,
        },
    );
    send(
        &right,
        server_addr,
        &ClientPacket::SetRace {
            player_id: right_player.id,
            race: RaceKind::Grove,
        },
    );

    send(
        &left,
        server_addr,
        &ClientPacket::SetReady {
            player_id: left_player.id,
            ready: true,
        },
    );
    send(
        &right,
        server_addr,
        &ClientPacket::SetReady {
            player_id: right_player.id,
            ready: true,
        },
    );

    wait_for_snapshot(&left, Duration::from_secs(3), |snapshot| {
        snapshot.phase == MatchPhase::Playing
    });

    send(
        &left,
        server_addr,
        &ClientPacket::PlaceBuilding {
            player_id: left_player.id,
            kind: BuildingKind::VanguardBarracks,
            cell: GridCell { x: 0, y: 0 },
        },
    );
    send(
        &right,
        server_addr,
        &ClientPacket::PlaceBuilding {
            player_id: right_player.id,
            kind: BuildingKind::GroveThornSpire,
            cell: GridCell { x: 0, y: 0 },
        },
    );

    wait_for_snapshot(&left, Duration::from_secs(3), |snapshot| {
        snapshot.buildings.len() == 2
    });
    wait_for_snapshot(&left, Duration::from_secs(9), |snapshot| {
        !snapshot.units.is_empty()
    });
}

fn assert_duplicate_name_rejected(socket: &UdpSocket, server_addr: SocketAddr, name: &str) {
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        send(
            socket,
            server_addr,
            &ClientPacket::Join {
                version: PROTOCOL_VERSION,
                name: name.to_string(),
            },
        );
        if let Some(ServerPacket::Error { message }) = recv(socket) {
            assert!(message.contains("already connected"));
            return;
        }
    }
    panic!("duplicate name was not rejected");
}

fn unused_local_addr() -> SocketAddr {
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    socket.local_addr().unwrap()
}

fn spawn_server(addr: SocketAddr) -> ServerProcess {
    let child = Command::new(env!("CARGO_BIN_EXE_castle_lanes_server"))
        .arg(addr.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    thread::sleep(Duration::from_millis(250));
    ServerProcess(child)
}

fn join(socket: &UdpSocket, server_addr: SocketAddr, name: &str) -> castle_lanes::sim::PlayerInfo {
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        send(
            socket,
            server_addr,
            &ClientPacket::Join {
                version: PROTOCOL_VERSION,
                name: name.to_string(),
            },
        );
        if let Some(ServerPacket::Welcome { player }) = recv(socket) {
            return player;
        }
    }
    panic!("server did not welcome {name}");
}

fn wait_for_snapshot(
    socket: &UdpSocket,
    timeout: Duration,
    predicate: impl Fn(&castle_lanes::sim::MatchSnapshot) -> bool,
) -> castle_lanes::sim::MatchSnapshot {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if let Some(ServerPacket::Snapshot(snapshot)) = recv(socket) {
            if predicate(&snapshot) {
                return snapshot;
            }
        }
    }
    panic!("timed out waiting for matching snapshot");
}

fn send(socket: &UdpSocket, server_addr: SocketAddr, packet: &ClientPacket) {
    let bytes = encode(packet).unwrap();
    socket.send_to(&bytes, server_addr).unwrap();
}

fn recv(socket: &UdpSocket) -> Option<ServerPacket> {
    let mut buf = [0_u8; 16_384];
    let (len, _) = socket.recv_from(&mut buf).ok()?;
    decode_server(&buf[..len]).ok()
}
