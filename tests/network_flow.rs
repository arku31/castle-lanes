use castle_lanes::net::{
    ClientPacket, PROTOCOL_VERSION, ServerPacket, apply_snapshot_delta, decode_server, encode,
};
use castle_lanes::sim::{
    BalanceConfig, BuildZone, BuildingKind, GridCell, Lane, MatchPhase, RaceKind, Team,
};
use std::fs;
use std::net::{SocketAddr, UdpSocket};
use std::path::Path;
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

    connect(&left, server_addr, "Alice");
    let left_player = create_game(&left, server_addr, "Alice's Game");
    connect(&right, server_addr, "Bryn");
    let game_id = wait_for_game_list(&right, Duration::from_secs(3))
        .first()
        .expect("created game should be listed")
        .id;
    let right_player = join_game(&right, server_addr, game_id);
    let duplicate = UdpSocket::bind("127.0.0.1:0").unwrap();
    duplicate
        .set_read_timeout(Some(Duration::from_millis(50)))
        .unwrap();
    assert_duplicate_name_rejected(&duplicate, server_addr, "Alice");
    let mut left_snapshot = None;

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

    wait_for_snapshot(
        &left,
        &mut left_snapshot,
        Duration::from_secs(3),
        "playing",
        |snapshot| snapshot.phase == MatchPhase::Playing,
    );

    send(
        &left,
        server_addr,
        &ClientPacket::PlaceBuilding {
            player_id: left_player.id,
            kind: BuildingKind::VanguardBarracks,
            lane: Lane::Top,
            zone: BuildZone::Front,
            cell: GridCell { x: 0, y: 0 },
            seq: None,
        },
    );
    send(
        &right,
        server_addr,
        &ClientPacket::PlaceBuilding {
            player_id: right_player.id,
            kind: BuildingKind::GroveThornSpire,
            lane: Lane::Bottom,
            zone: BuildZone::Front,
            cell: GridCell { x: 0, y: 0 },
            seq: None,
        },
    );
    let mut right_snapshot = None;

    // Server-side fog (plan.md Phase 1): each side only receives its own
    // building; the enemy building stays hidden until scouted.
    wait_for_snapshot(
        &left,
        &mut left_snapshot,
        Duration::from_secs(3),
        "own building visible and enemy building fogged",
        |snapshot| snapshot.buildings.len() == 1 && snapshot.buildings[0].owner == left_player.id,
    );
    wait_for_snapshot(
        &right,
        &mut right_snapshot,
        Duration::from_secs(3),
        "right sees only its own building",
        |snapshot| snapshot.buildings.len() == 1 && snapshot.buildings[0].owner == right_player.id,
    );
    let spawn_timeout = Duration::from_secs_f32(
        BalanceConfig::default()
            .building(BuildingKind::VanguardBarracks)
            .spawn_interval
            .unwrap()
            + 18.0,
    );
    wait_for_snapshot_with_keepalive(
        &left,
        &mut left_snapshot,
        server_addr,
        spawn_timeout,
        &[(&left, "Alice"), (&right, "Bryn")],
        |snapshot| !snapshot.units.is_empty(),
    );
}

#[test]
fn dedicated_server_accepts_rematch_votes_and_resets_lobby() {
    let server_addr = unused_local_addr();
    let balance_path = write_fast_gameover_balance();
    let _server = spawn_server_with_balance(server_addr, &balance_path);
    let left = UdpSocket::bind("127.0.0.1:0").unwrap();
    let right = UdpSocket::bind("127.0.0.1:0").unwrap();
    left.set_read_timeout(Some(Duration::from_millis(50)))
        .unwrap();
    right
        .set_read_timeout(Some(Duration::from_millis(50)))
        .unwrap();

    connect(&left, server_addr, "RematchAlice");
    let left_player = create_game(&left, server_addr, "Fast Rematch");
    connect(&right, server_addr, "RematchBryn");
    let game_id = wait_for_game_list(&right, Duration::from_secs(3))
        .first()
        .expect("created game should be listed")
        .id;
    let right_player = join_game(&right, server_addr, game_id);
    let mut left_snapshot = None;
    ready_player(&left, server_addr, left_player.id, RaceKind::Vanguard);
    ready_player(&right, server_addr, right_player.id, RaceKind::Ember);

    wait_for_snapshot(
        &left,
        &mut left_snapshot,
        Duration::from_secs(3),
        "game over",
        |snapshot| snapshot.phase == MatchPhase::GameOver,
    );
    send(
        &left,
        server_addr,
        &ClientPacket::VoteRematch {
            player_id: left_player.id,
        },
    );
    send(
        &right,
        server_addr,
        &ClientPacket::VoteRematch {
            player_id: right_player.id,
        },
    );

    let reset = wait_for_snapshot(
        &left,
        &mut left_snapshot,
        Duration::from_secs(3),
        "lobby reset",
        |snapshot| {
            snapshot.phase == MatchPhase::Lobby
                && snapshot
                    .players
                    .iter()
                    .all(|player| !player.ready && player.race.is_none() && !player.rematch_vote)
                && snapshot.buildings.is_empty()
                && snapshot.units.is_empty()
                && snapshot.winner.is_none()
        },
    );

    assert_eq!(reset.message, "Rematch ready. Set ready again.");
    let _ = fs::remove_file(balance_path);
}

#[test]
fn dedicated_server_lists_multiple_games() {
    let server_addr = unused_local_addr();
    let _server = spawn_server(server_addr);
    let first = UdpSocket::bind("127.0.0.1:0").unwrap();
    let second = UdpSocket::bind("127.0.0.1:0").unwrap();
    let browser = UdpSocket::bind("127.0.0.1:0").unwrap();
    for socket in [&first, &second, &browser] {
        socket
            .set_read_timeout(Some(Duration::from_millis(50)))
            .unwrap();
    }

    connect(&first, server_addr, "HostOne");
    create_game(&first, server_addr, "First Game");
    connect(&second, server_addr, "HostTwo");
    create_game(&second, server_addr, "Second Game");
    connect(&browser, server_addr, "Browser");

    let games = wait_for_game_list(&browser, Duration::from_secs(3));

    assert!(games.iter().any(|game| game.name == "First Game"));
    assert!(games.iter().any(|game| game.name == "Second Game"));
    assert_eq!(games.len(), 2);
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
    spawn_server_args(addr, None)
}

fn spawn_server_with_balance(addr: SocketAddr, balance_path: &Path) -> ServerProcess {
    spawn_server_args(addr, Some(balance_path))
}

fn spawn_server_args(addr: SocketAddr, balance_path: Option<&Path>) -> ServerProcess {
    let child = Command::new(env!("CARGO_BIN_EXE_castle_lanes_server"))
        .arg(addr.to_string())
        .args(balance_path.iter().map(|path| path.as_os_str()))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    thread::sleep(Duration::from_millis(250));
    ServerProcess(child)
}

#[test]
fn team_play_2v2_full_match_flow() {
    let server_addr = unused_local_addr();
    let _server = spawn_server(server_addr);

    let mut sockets = Vec::new();
    let mut players = Vec::new();
    let names = ["Alice", "Bob", "Carol", "Dave"];
    let races = [
        RaceKind::Vanguard,
        RaceKind::Grove,
        RaceKind::Ember,
        RaceKind::Vanguard,
    ];

    for (index, name) in names.iter().enumerate() {
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        socket
            .set_read_timeout(Some(Duration::from_millis(50)))
            .unwrap();
        connect(&socket, server_addr, name);
        if index == 0 {
            let player = create_game_with_team_size(&socket, server_addr, "2v2 Match", Some(2));
            assert_eq!(player.team, Team::Left);
            players.push(player);
        } else {
            let games = wait_for_game_list(&socket, Duration::from_secs(3));
            let game = games.first().expect("game should be listed");
            let player = join_game(&socket, server_addr, game.id);
            players.push(player);
        }
        sockets.push(socket);
    }

    assert_eq!(players[0].team, Team::Left);
    assert_eq!(players[1].team, Team::Left);
    assert_eq!(players[2].team, Team::Right);
    assert_eq!(players[3].team, Team::Right);

    for (index, socket) in sockets.iter().enumerate() {
        send(
            socket,
            server_addr,
            &ClientPacket::SetRace {
                player_id: players[index].id,
                race: races[index],
            },
        );
        send(
            socket,
            server_addr,
            &ClientPacket::SetReady {
                player_id: players[index].id,
                ready: true,
            },
        );
    }

    let mut snapshots: Vec<Option<castle_lanes::sim::MatchSnapshot>> =
        sockets.iter().map(|_| None).collect();
    wait_for_snapshot(
        &sockets[0],
        &mut snapshots[0],
        Duration::from_secs(5),
        "2v2 match started",
        |snapshot| snapshot.phase == MatchPhase::Playing && snapshot.players.len() == 4,
    );

    let lanes = [Lane::Top, Lane::UpperMid, Lane::Bottom, Lane::LowerMid];
    let kinds = [
        BuildingKind::VanguardBarracks,
        BuildingKind::GroveRootDen,
        BuildingKind::EmberCinderPit,
        BuildingKind::VanguardBarracks,
    ];
    for (index, socket) in sockets.iter().enumerate() {
        send(
            socket,
            server_addr,
            &ClientPacket::PlaceBuilding {
                player_id: players[index].id,
                kind: kinds[index],
                lane: lanes[index],
                zone: BuildZone::Front,
                cell: GridCell { x: 0, y: 0 },
                seq: None,
            },
        );
    }

    wait_for_snapshot_with_keepalive(
        &sockets[0],
        &mut snapshots[0],
        server_addr,
        Duration::from_secs(10),
        &[
            (&sockets[0], "Alice"),
            (&sockets[1], "Bob"),
            (&sockets[2], "Carol"),
            (&sockets[3], "Dave"),
        ],
        |snapshot| snapshot.buildings.len() >= 2,
    );

    let snapshot = snapshots[0].as_ref().unwrap();
    // Fog: Alice only sees Left-team buildings
    for index in 0..2 {
        let owned = snapshot
            .buildings
            .iter()
            .filter(|b| b.owner == players[index].id)
            .count();
        assert_eq!(owned, 1, "{} should have 1 building", names[index]);
    }
    assert_eq!(snapshot.buildings.len(), 2);
}

fn connect(socket: &UdpSocket, server_addr: SocketAddr, name: &str) {
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
        if let Some(ServerPacket::Connected { .. }) = recv(socket) {
            return;
        }
    }
    panic!("server did not connect {name}");
}

fn create_game(
    socket: &UdpSocket,
    server_addr: SocketAddr,
    name: &str,
) -> castle_lanes::sim::PlayerInfo {
    create_game_with_team_size(socket, server_addr, name, None)
}

fn create_game_with_team_size(
    socket: &UdpSocket,
    server_addr: SocketAddr,
    name: &str,
    team_size: Option<usize>,
) -> castle_lanes::sim::PlayerInfo {
    send(
        socket,
        server_addr,
        &ClientPacket::CreateGame {
            name: name.to_string(),
            team_size,
            random_factions: false,
        },
    );
    wait_for_welcome(socket, Duration::from_secs(3))
}

fn join_game(
    socket: &UdpSocket,
    server_addr: SocketAddr,
    game_id: castle_lanes::net::GameId,
) -> castle_lanes::sim::PlayerInfo {
    send(
        socket,
        server_addr,
        &ClientPacket::JoinGame {
            game_id,
            spectator: false,
        },
    );
    wait_for_welcome(socket, Duration::from_secs(3))
}

fn wait_for_welcome(socket: &UdpSocket, timeout: Duration) -> castle_lanes::sim::PlayerInfo {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if let Some(ServerPacket::Welcome { player, .. }) = recv(socket) {
            return player;
        }
    }
    panic!("server did not welcome player into game");
}

fn wait_for_game_list(socket: &UdpSocket, timeout: Duration) -> Vec<castle_lanes::net::GameInfo> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if let Some(ServerPacket::GameList { games }) = recv(socket) {
            return games;
        }
    }
    panic!("server did not send game list");
}

fn ready_player(
    socket: &UdpSocket,
    server_addr: SocketAddr,
    player_id: castle_lanes::sim::PlayerId,
    race: RaceKind,
) {
    send(
        socket,
        server_addr,
        &ClientPacket::SetRace { player_id, race },
    );
    send(
        socket,
        server_addr,
        &ClientPacket::SetReady {
            player_id,
            ready: true,
        },
    );
}

fn write_fast_gameover_balance() -> std::path::PathBuf {
    let mut balance = BalanceConfig::default();
    balance.sudden_death_start = 0.0;
    for race in &mut balance.races {
        race.castle_health = 1;
    }
    let path = std::env::temp_dir().join(format!(
        "castle_lanes_fast_gameover_{}_{}.json",
        std::process::id(),
        Instant::now().elapsed().as_nanos()
    ));
    fs::write(&path, serde_json::to_string(&balance).unwrap()).unwrap();
    path
}

fn wait_for_snapshot(
    socket: &UdpSocket,
    snapshot: &mut Option<castle_lanes::sim::MatchSnapshot>,
    timeout: Duration,
    label: &str,
    predicate: impl Fn(&castle_lanes::sim::MatchSnapshot) -> bool,
) -> castle_lanes::sim::MatchSnapshot {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if let Some(packet) = recv(socket) {
            update_snapshot(snapshot, packet);
        }
        if let Some(snapshot) = snapshot.as_ref() {
            if predicate(snapshot) {
                return snapshot.clone();
            }
        }
    }
    panic!("timed out waiting for matching snapshot: {label}");
}

fn wait_for_snapshot_with_keepalive(
    socket: &UdpSocket,
    snapshot: &mut Option<castle_lanes::sim::MatchSnapshot>,
    server_addr: SocketAddr,
    timeout: Duration,
    keepalives: &[(&UdpSocket, &str)],
    predicate: impl Fn(&castle_lanes::sim::MatchSnapshot) -> bool,
) -> castle_lanes::sim::MatchSnapshot {
    let deadline = Instant::now() + timeout;
    let mut next_keepalive = Instant::now();
    while Instant::now() < deadline {
        if Instant::now() >= next_keepalive {
            for (keepalive_socket, name) in keepalives {
                send(
                    keepalive_socket,
                    server_addr,
                    &ClientPacket::Join {
                        version: PROTOCOL_VERSION,
                        name: (*name).to_string(),
                    },
                );
            }
            next_keepalive = Instant::now() + Duration::from_secs(1);
        }

        if let Some(packet) = recv(socket) {
            update_snapshot(snapshot, packet);
        }
        if let Some(snapshot) = snapshot.as_ref() {
            if predicate(snapshot) {
                return snapshot.clone();
            }
        }
    }
    panic!("timed out waiting for matching snapshot");
}

fn update_snapshot(snapshot: &mut Option<castle_lanes::sim::MatchSnapshot>, packet: ServerPacket) {
    match packet {
        ServerPacket::Snapshot(next_snapshot) => {
            *snapshot = Some(next_snapshot);
        }
        ServerPacket::SnapshotDelta(delta) => {
            if let Some(snapshot) = snapshot {
                apply_snapshot_delta(snapshot, delta);
            }
        }
        _ => {}
    }
}

fn send(socket: &UdpSocket, server_addr: SocketAddr, packet: &ClientPacket) {
    let bytes = encode(packet).unwrap();
    socket.send_to(&bytes, server_addr).unwrap();
}

fn recv(socket: &UdpSocket) -> Option<ServerPacket> {
    let mut buf = [0_u8; 65_535];
    let (len, _) = socket.recv_from(&mut buf).ok()?;
    decode_server(&buf[..len]).ok()
}
