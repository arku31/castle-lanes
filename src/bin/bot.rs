use castle_lanes::net::{
    ClientPacket, DEFAULT_SERVER_ADDR, PROTOCOL_VERSION, ServerPacket, apply_snapshot_delta,
    decode_server, encode,
};
use castle_lanes::sim::{
    BuildZone, BuildingKind, GridCell, Lane, MatchPhase, PlayerId, RaceKind, Team,
};
use std::env;
use std::io::ErrorKind;
use std::net::{SocketAddr, UdpSocket};
use std::thread;
use std::time::{Duration, Instant};

fn main() -> std::io::Result<()> {
    let mut argv = env::args().skip(1);
    if argv.next().as_deref() == Some("--version") {
        println!("castle_lanes_bot {}", castle_lanes::VERSION);
        return Ok(());
    }
    let options = parse_args();
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.set_nonblocking(true)?;

    println!(
        "Castle Lanes bot '{}' connecting to {}",
        options.name, options.server_addr
    );

    let mut player_id = None;
    let mut connected = false;
    let mut printed_welcome = false;
    let mut sent_game_request = false;
    let mut last_join = Instant::now() - Duration::from_secs(3);
    let mut last_keepalive = Instant::now();
    let mut sent_race = false;
    let mut sent_ready = false;
    let mut builds_placed: usize = 0;
    let mut last_build = Instant::now();
    let mut sent_rematch = false;
    let mut last_phase = None;
    let mut snapshot = None;
    let mut buf = [0_u8; 65_535];

    loop {
        if !connected && last_join.elapsed() > Duration::from_secs(1) {
            send(
                &socket,
                options.server_addr,
                &ClientPacket::Join {
                    version: PROTOCOL_VERSION,
                    name: options.name.clone(),
                },
            );
            last_join = Instant::now();
        } else if connected && last_keepalive.elapsed() > Duration::from_secs(2) {
            send(
                &socket,
                options.server_addr,
                &ClientPacket::Join {
                    version: PROTOCOL_VERSION,
                    name: options.name.clone(),
                },
            );
            last_keepalive = Instant::now();
        }

        loop {
            match socket.recv_from(&mut buf) {
                Ok((len, _)) => match decode_server(&buf[..len]) {
                    Ok(ServerPacket::Connected { .. }) => {
                        connected = true;
                        send(&socket, options.server_addr, &ClientPacket::ListGames);
                    }
                    Ok(ServerPacket::GameList { games }) => {
                        if player_id.is_none() && !sent_game_request {
                            if let Some(game) = games.first() {
                                send(
                                    &socket,
                                    options.server_addr,
                                    &ClientPacket::JoinGame { game_id: game.id },
                                );
                            } else {
                                send(
                                    &socket,
                                    options.server_addr,
                                    &ClientPacket::CreateGame {
                                        name: format!("{}'s Game", options.name),
                                        team_size: None,
                                    },
                                );
                            }
                            sent_game_request = true;
                        }
                    }
                    Ok(ServerPacket::Welcome { player, .. }) => {
                        if !printed_welcome {
                            println!("Joined as {} ({:?})", player.name, player.team);
                            printed_welcome = true;
                        }
                        player_id = Some(player.id);
                    }
                    Ok(
                        ServerPacket::BalanceRaces { .. }
                        | ServerPacket::BalanceBuildings { .. }
                        | ServerPacket::BalanceUnits { .. },
                    ) => {}
                    Ok(ServerPacket::Snapshot(next_snapshot)) => {
                        snapshot = Some(next_snapshot);
                    }
                    Ok(ServerPacket::SnapshotDelta(delta)) => {
                        let Some(snapshot) = snapshot.as_mut() else {
                            continue;
                        };
                        apply_snapshot_delta(snapshot, delta);
                    }
                    Ok(ServerPacket::Ack { .. }) => {}
                    Ok(ServerPacket::Error { message }) => {
                        eprintln!("Server error: {message}");
                    }
                    Err(err) => eprintln!("Bad server packet: {err}"),
                },
                Err(err) if err.kind() == ErrorKind::WouldBlock => break,
                Err(err) => return Err(err),
            }

            let Some(snapshot) = snapshot.as_ref() else {
                continue;
            };
            let previous_phase = last_phase;
            if last_phase != Some(snapshot.phase) {
                println!("Phase: {:?} - {}", snapshot.phase, snapshot.message);
                last_phase = Some(snapshot.phase);
            }
            let Some(id) = player_id else {
                continue;
            };
            if previous_phase == Some(MatchPhase::GameOver) && snapshot.phase == MatchPhase::Lobby {
                sent_race = false;
                sent_ready = false;
                sent_rematch = false;
            }
            if !sent_race {
                send(
                    &socket,
                    options.server_addr,
                    &ClientPacket::SetRace {
                        player_id: id,
                        race: options.race,
                    },
                );
                sent_race = true;
            }
            if !sent_ready {
                let Some(player) = snapshot.players.iter().find(|player| player.id == id) else {
                    continue;
                };
                if player.race.is_none() {
                    continue;
                }
                send_ready(&socket, options.server_addr, id);
                sent_ready = true;
            }
            if snapshot.phase == MatchPhase::Playing
                && last_build.elapsed() >= Duration::from_secs(4)
            {
                // Keep building like a player would: rotate lanes and cells
                // whenever this side can afford the configured producer.
                let Some(player) = snapshot.players.iter().find(|player| player.id == id) else {
                    continue;
                };
                let slot = match player.team {
                    castle_lanes::sim::Team::Left => 0,
                    castle_lanes::sim::Team::Right => 1,
                };
                let gold = snapshot.economies[slot].gold;
                let cost = castle_lanes::sim::BalanceConfig::default()
                    .building(options.building)
                    .cost;
                if gold >= cost {
                    let build_count = builds_placed;
                    let lane = if build_count % 2 == 0 {
                        default_lane_for_team(player.team)
                    } else {
                        match default_lane_for_team(player.team) {
                            Lane::Top => Lane::Bottom,
                            Lane::Bottom => Lane::Top,
                        }
                    };
                    let cell = GridCell {
                        x: (build_count / 2) as i32 % 10,
                        y: ((build_count / 2) as i32 / 10) % 5,
                    };
                    send(
                        &socket,
                        options.server_addr,
                        &ClientPacket::PlaceBuilding {
                            player_id: id,
                            kind: options.building,
                            lane,
                            zone: BuildZone::Front,
                            cell,
                            seq: None,
                        },
                    );
                    builds_placed += 1;
                    last_build = Instant::now();
                    println!("Placed {}", options.building.fallback_name());
                }
            }
            if snapshot.phase == MatchPhase::GameOver && !sent_rematch {
                send(
                    &socket,
                    options.server_addr,
                    &ClientPacket::VoteRematch { player_id: id },
                );
                sent_rematch = true;
                println!("Voted for rematch");
            }
        }

        thread::sleep(Duration::from_millis(16));
    }
}

struct BotOptions {
    server_addr: SocketAddr,
    name: String,
    race: RaceKind,
    building: BuildingKind,
}

fn parse_args() -> BotOptions {
    let mut server = DEFAULT_SERVER_ADDR.to_string();
    let mut name = "Bot".to_string();
    let mut race = RaceKind::Vanguard;
    let mut building = None;
    let mut legacy_json = false;
    let args: Vec<String> = env::args().collect();
    let mut idx = 1;
    while idx < args.len() {
        match args[idx].as_str() {
            "--legacy-json" => {
                castle_lanes::net::set_outgoing_format(castle_lanes::net::PacketFormat::Json);
                legacy_json = true;
            }
            "--server" if idx + 1 < args.len() => {
                server = args[idx + 1].clone();
                idx += 1;
            }
            "--name" if idx + 1 < args.len() => {
                name = args[idx + 1].clone();
                idx += 1;
            }
            "--building" if idx + 1 < args.len() => {
                building = Some(parse_building(&args[idx + 1]));
                idx += 1;
            }
            "--race" if idx + 1 < args.len() => {
                race = parse_race(&args[idx + 1]);
                idx += 1;
            }
            _ => {}
        }
        idx += 1;
    }

    BotOptions {
        server_addr: server.parse().expect("--server must be host:port"),
        name,
        race,
        building: building.unwrap_or_else(|| default_building_for_race(race)),
    }
}

fn parse_building(value: &str) -> BuildingKind {
    BuildingKind::from_cli(value)
        .unwrap_or_else(|| panic!("--building must name a configured building"))
}

fn parse_race(value: &str) -> RaceKind {
    match value.to_ascii_lowercase().as_str() {
        "vanguard" | "human" | "humans" | "v" | "1" => RaceKind::Vanguard,
        "grove" | "forest" | "g" | "2" => RaceKind::Grove,
        "ember" | "fire" | "e" | "3" => RaceKind::Ember,
        _ => panic!("--race must be vanguard, grove, or ember"),
    }
}

fn default_building_for_race(race: RaceKind) -> BuildingKind {
    match race {
        RaceKind::Vanguard => BuildingKind::VanguardRangeTower,
        RaceKind::Grove => BuildingKind::GroveThornSpire,
        RaceKind::Ember => BuildingKind::EmberFlameSpire,
    }
}

fn default_lane_for_team(team: Team) -> Lane {
    match team {
        Team::Left => Lane::Top,
        Team::Right => Lane::Bottom,
    }
}

fn send_ready(socket: &UdpSocket, server_addr: SocketAddr, player_id: PlayerId) {
    send(
        socket,
        server_addr,
        &ClientPacket::SetReady {
            player_id,
            ready: true,
        },
    );
}

fn send(socket: &UdpSocket, server_addr: SocketAddr, packet: &ClientPacket) {
    let bytes = encode(packet).expect("encode packet");
    let _ = socket.send_to(&bytes, server_addr);
}
