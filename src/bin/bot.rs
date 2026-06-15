use castle_lanes::net::{
    ClientPacket, DEFAULT_SERVER_ADDR, PROTOCOL_VERSION, ServerPacket, decode_server, encode,
};
use castle_lanes::sim::{BuildingKind, GridCell, MatchPhase, PlayerId, RaceKind};
use std::env;
use std::io::ErrorKind;
use std::net::{SocketAddr, UdpSocket};
use std::thread;
use std::time::{Duration, Instant};

fn main() -> std::io::Result<()> {
    let options = parse_args();
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.set_nonblocking(true)?;

    println!(
        "Castle Lanes bot '{}' connecting to {}",
        options.name, options.server_addr
    );

    let mut player_id = None;
    let mut printed_welcome = false;
    let mut last_join = Instant::now() - Duration::from_secs(3);
    let mut last_keepalive = Instant::now();
    let mut sent_race = false;
    let mut sent_ready = false;
    let mut sent_building = false;
    let mut last_phase = None;
    let mut buf = [0_u8; 16_384];

    loop {
        if player_id.is_none() && last_join.elapsed() > Duration::from_secs(1) {
            send(
                &socket,
                options.server_addr,
                &ClientPacket::Join {
                    version: PROTOCOL_VERSION,
                    name: options.name.clone(),
                },
            );
            last_join = Instant::now();
        } else if player_id.is_some() && last_keepalive.elapsed() > Duration::from_secs(2) {
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
                    Ok(ServerPacket::Welcome { player }) => {
                        if !printed_welcome {
                            println!("Joined as {} ({:?})", player.name, player.team);
                            printed_welcome = true;
                        }
                        player_id = Some(player.id);
                    }
                    Ok(ServerPacket::Snapshot(snapshot)) => {
                        if last_phase != Some(snapshot.phase) {
                            println!("Phase: {:?} - {}", snapshot.phase, snapshot.message);
                            last_phase = Some(snapshot.phase);
                        }
                        let Some(id) = player_id else {
                            continue;
                        };
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
                            let Some(player) =
                                snapshot.players.iter().find(|player| player.id == id)
                            else {
                                continue;
                            };
                            if player.race.is_none() {
                                continue;
                            }
                            send_ready(&socket, options.server_addr, id);
                            sent_ready = true;
                        }
                        if snapshot.phase == MatchPhase::Playing && !sent_building {
                            send(
                                &socket,
                                options.server_addr,
                                &ClientPacket::PlaceBuilding {
                                    player_id: id,
                                    kind: options.building,
                                    cell: GridCell { x: 0, y: 0 },
                                },
                            );
                            sent_building = true;
                            println!("Placed {}", options.building.fallback_name());
                        }
                    }
                    Ok(ServerPacket::Error { message }) => {
                        eprintln!("Server error: {message}");
                    }
                    Err(err) => eprintln!("Bad server packet: {err}"),
                },
                Err(err) if err.kind() == ErrorKind::WouldBlock => break,
                Err(err) => return Err(err),
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
    let args: Vec<String> = env::args().collect();
    let mut idx = 1;
    while idx < args.len() {
        match args[idx].as_str() {
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
