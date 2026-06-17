use castle_lanes::net::{
    ClientPacket, DEFAULT_SERVER_ADDR, PROTOCOL_VERSION, ServerPacket, decode_client, encode,
};
use castle_lanes::sim::{BalanceConfig, DEFAULT_BALANCE_PATH, GameSim, PlayerId};
use std::collections::HashMap;
use std::env;
use std::io::ErrorKind;
use std::net::{SocketAddr, UdpSocket};
use std::thread;
use std::time::{Duration, Instant};

const TICK_RATE: Duration = Duration::from_millis(33);
const SNAPSHOT_RATE: Duration = Duration::from_millis(100);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(8);
const SAFE_UDP_PAYLOAD_BYTES: usize = 1200;
const MAX_UDP_PAYLOAD_BYTES: usize = 60_000;
const SNAPSHOT_SIZE_LOG_INTERVAL_TICKS: u64 = 30;

#[derive(Debug, Clone)]
struct ClientSession {
    player_id: PlayerId,
    last_seen: Instant,
}

fn main() -> std::io::Result<()> {
    let mut args = env::args().skip(1);
    let bind_addr = args
        .next()
        .unwrap_or_else(|| DEFAULT_SERVER_ADDR.to_string());
    let balance_path = args
        .next()
        .unwrap_or_else(|| DEFAULT_BALANCE_PATH.to_string());
    let socket = UdpSocket::bind(&bind_addr)?;
    socket.set_nonblocking(true)?;
    println!("Castle Lanes server listening on {bind_addr}");

    let balance = BalanceConfig::load_or_default(&balance_path);
    println!("Loaded balance config from {balance_path}");
    let mut sim = GameSim::new(balance);
    let mut clients: HashMap<SocketAddr, ClientSession> = HashMap::new();
    let mut last_tick = Instant::now();
    let mut last_snapshot = Instant::now();
    let mut buf = [0_u8; 4096];

    loop {
        loop {
            match socket.recv_from(&mut buf) {
                Ok((len, addr)) => {
                    if let Err(err) =
                        handle_packet(&socket, &mut sim, &mut clients, addr, &buf[..len])
                    {
                        eprintln!("packet from {addr}: {err}");
                        let _ = send_packet(&socket, addr, &ServerPacket::Error { message: err });
                    }
                }
                Err(err) if err.kind() == ErrorKind::WouldBlock => break,
                Err(err) => return Err(err),
            }
        }

        let now = Instant::now();
        while now.duration_since(last_tick) >= TICK_RATE {
            sim.tick(TICK_RATE.as_secs_f32());
            last_tick += TICK_RATE;
        }

        let timed_out: Vec<(SocketAddr, PlayerId)> = clients
            .iter()
            .filter(|(_, session)| now.duration_since(session.last_seen) > CLIENT_TIMEOUT)
            .map(|(addr, session)| (*addr, session.player_id))
            .collect();
        for (addr, player_id) in timed_out {
            clients.remove(&addr);
            sim.disconnect_player(player_id);
        }

        if now.duration_since(last_snapshot) >= SNAPSHOT_RATE {
            broadcast_snapshot(&socket, &clients, &sim);
            last_snapshot = now;
        }

        thread::sleep(Duration::from_millis(2));
    }
}

fn handle_packet(
    socket: &UdpSocket,
    sim: &mut GameSim,
    clients: &mut HashMap<SocketAddr, ClientSession>,
    addr: SocketAddr,
    bytes: &[u8],
) -> Result<(), String> {
    let packet = decode_client(bytes).map_err(|err| format!("bad packet: {err}"))?;
    match packet {
        ClientPacket::Join { version, name } => {
            if version != PROTOCOL_VERSION {
                return Err(format!(
                    "protocol mismatch: client {version}, server {PROTOCOL_VERSION}"
                ));
            }
            let player = if let Some(session) = clients.get_mut(&addr) {
                session.last_seen = Instant::now();
                sim.player(session.player_id)
                    .ok_or_else(|| "Existing session has no player.".to_string())?
            } else {
                sim.join_or_update_player(name)?
            };
            clients.insert(
                addr,
                ClientSession {
                    player_id: player.id,
                    last_seen: Instant::now(),
                },
            );
            send_packet(socket, addr, &ServerPacket::Welcome { player })?;
            send_balance(socket, addr, &sim.balance)?;
        }
        ClientPacket::SetReady { player_id, ready } => {
            touch(clients, addr, player_id)?;
            sim.set_ready(player_id, ready)?;
        }
        ClientPacket::SetRace { player_id, race } => {
            touch(clients, addr, player_id)?;
            sim.set_race(player_id, race)?;
        }
        ClientPacket::PlaceBuilding {
            player_id,
            kind,
            lane,
            zone,
            cell,
        } => {
            touch(clients, addr, player_id)?;
            sim.place_building(player_id, kind, lane, zone, cell)?;
        }
        ClientPacket::VoteRematch { player_id } => {
            touch(clients, addr, player_id)?;
            sim.vote_rematch(player_id);
        }
        ClientPacket::Disconnect { player_id } => {
            touch(clients, addr, player_id)?;
            clients.remove(&addr);
            sim.disconnect_player(player_id);
        }
    }
    Ok(())
}

fn touch(
    clients: &mut HashMap<SocketAddr, ClientSession>,
    addr: SocketAddr,
    player_id: PlayerId,
) -> Result<(), String> {
    let Some(session) = clients.get_mut(&addr) else {
        return Err("Join before sending commands.".to_string());
    };
    if session.player_id != player_id {
        return Err("Player id does not match this connection.".to_string());
    }
    session.last_seen = Instant::now();
    Ok(())
}

fn broadcast_snapshot(
    socket: &UdpSocket,
    clients: &HashMap<SocketAddr, ClientSession>,
    sim: &GameSim,
) {
    let packet = ServerPacket::Snapshot(sim.snapshot());
    let bytes = match encode(&packet) {
        Ok(bytes) => bytes,
        Err(err) => {
            eprintln!("snapshot encode failed: {err}");
            return;
        }
    };

    if bytes.len() > SAFE_UDP_PAYLOAD_BYTES && sim.tick % SNAPSHOT_SIZE_LOG_INTERVAL_TICKS == 0 {
        eprintln!(
            "snapshot is {} bytes for {} units / {} buildings; UDP fragmentation is likely",
            bytes.len(),
            sim.units.len(),
            sim.buildings.len()
        );
    }
    if bytes.len() > MAX_UDP_PAYLOAD_BYTES {
        eprintln!(
            "snapshot is {} bytes and too large for UDP; skipped broadcast",
            bytes.len()
        );
        return;
    }

    for addr in clients.keys() {
        let _ = socket.send_to(&bytes, *addr);
    }
}

fn send_balance(
    socket: &UdpSocket,
    addr: SocketAddr,
    balance: &BalanceConfig,
) -> Result<(), String> {
    send_packet(
        socket,
        addr,
        &ServerPacket::BalanceRaces {
            starting_gold: balance.starting_gold,
            base_income: balance.base_income,
            income_interval: balance.income_interval,
            interest_rate: balance.interest_rate,
            castle_regen_per_second: balance.castle_regen_per_second,
            sudden_death_start: balance.sudden_death_start,
            races: balance.races.clone(),
        },
    )?;
    send_packet(
        socket,
        addr,
        &ServerPacket::BalanceBuildings {
            buildings: balance.buildings.clone(),
        },
    )?;
    send_packet(
        socket,
        addr,
        &ServerPacket::BalanceUnits {
            units: balance.units.clone(),
        },
    )?;
    Ok(())
}

fn send_packet(socket: &UdpSocket, addr: SocketAddr, packet: &ServerPacket) -> Result<(), String> {
    let bytes = encode(packet).map_err(|err| format!("encode failed: {err}"))?;
    socket
        .send_to(&bytes, addr)
        .map(|_| ())
        .map_err(|err| format!("send failed: {err}"))
}
