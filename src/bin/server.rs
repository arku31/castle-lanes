use castle_lanes::net::{
    ClientPacket, DEFAULT_SERVER_ADDR, GameId, GameInfo, PROTOCOL_VERSION, ServerPacket,
    SnapshotDelta, decode_client, diff_snapshot, encode, entityless_snapshot,
    filter_snapshot_for_viewer,
};
use castle_lanes::sim::{BalanceConfig, DEFAULT_BALANCE_PATH, GameSim, MatchSnapshot, PlayerId};
use std::collections::{HashMap, HashSet};
use std::env;
use std::io::ErrorKind;
use std::net::{SocketAddr, UdpSocket};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const TICK_RATE: Duration = Duration::from_millis(33);
const SNAPSHOT_RATE: Duration = Duration::from_millis(100);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(8);
const RECONNECT_GRACE: Duration = Duration::from_secs(90);

/// Seats that disconnected during a live match, with the instant their
/// reserved seat expires (plan.md Phase 1 reconnect grace).
type ReconnectDeadlines = HashMap<(GameId, PlayerId), Instant>;
const SAFE_UDP_PAYLOAD_BYTES: usize = 1200;
const MAX_UDP_PAYLOAD_BYTES: usize = 60_000;
const SNAPSHOT_SIZE_LOG_INTERVAL_TICKS: u64 = 30;

#[derive(Debug, Clone)]
struct ClientSession {
    name: String,
    game_id: Option<GameId>,
    player_id: Option<PlayerId>,
    last_seen: Instant,
    last_snapshot: Option<MatchSnapshot>,
    /// Enemy buildings this session has scouted; re-sent stale while the
    /// client's fog memory should still show their silhouette.
    seen_enemy_buildings: HashMap<u64, castle_lanes::sim::Building>,
    /// Last successfully applied placement seq, for at-most-once retries.
    last_applied_seq: Option<u32>,
}

struct GameRoom {
    id: GameId,
    name: String,
    sim: GameSim,
    clients: HashSet<SocketAddr>,
}

fn main() -> std::io::Result<()> {
    let all_args: Vec<String> = env::args().skip(1).collect();
    if all_args.first().map(String::as_str) == Some("--version") {
        println!("castle_lanes_server {}", castle_lanes::VERSION);
        return Ok(());
    }
    let mut args = all_args.into_iter();
    let bind_addr = args
        .next()
        .unwrap_or_else(|| DEFAULT_SERVER_ADDR.to_string());
    let balance_path = args
        .next()
        .unwrap_or_else(|| DEFAULT_BALANCE_PATH.to_string());
    let socket = UdpSocket::bind(&bind_addr)?;
    socket.set_nonblocking(true)?;
    println!(
        "Castle Lanes server v{} listening on {bind_addr}",
        castle_lanes::VERSION
    );

    let balance = BalanceConfig::load_or_default(&balance_path);
    println!("Loaded balance config from {balance_path}");
    let mut rooms: HashMap<GameId, GameRoom> = HashMap::new();
    let mut clients: HashMap<SocketAddr, ClientSession> = HashMap::new();
    let mut next_game_id: GameId = 1;
    let mut reconnect_deadlines: ReconnectDeadlines = HashMap::new();
    let mut last_tick = Instant::now();
    let mut last_snapshot = Instant::now();
    let mut buf = [0_u8; 4096];

    loop {
        loop {
            match socket.recv_from(&mut buf) {
                Ok((len, addr)) => {
                    if let Err(err) = handle_packet(
                        &socket,
                        &balance,
                        &mut rooms,
                        &mut clients,
                        &mut next_game_id,
                        addr,
                        &buf[..len],
                    ) {
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
            for room in rooms.values_mut() {
                room.sim.tick(TICK_RATE.as_secs_f32());
            }
            last_tick += TICK_RATE;
        }

        let timed_out: Vec<SocketAddr> = clients
            .iter()
            .filter(|(_, session)| now.duration_since(session.last_seen) > CLIENT_TIMEOUT)
            .map(|(addr, _)| *addr)
            .collect();
        for addr in timed_out {
            disconnect_session(&mut rooms, &mut clients, addr);
        }
        remove_empty_rooms(&mut rooms);
        if update_reconnect_deadlines(&mut rooms, &mut reconnect_deadlines, now) {
            broadcast_game_lists(&socket, &rooms, &clients);
        }

        if now.duration_since(last_snapshot) >= SNAPSHOT_RATE {
            broadcast_rooms(&socket, &mut rooms, &mut clients);
            last_snapshot = now;
        }

        thread::sleep(Duration::from_millis(2));
    }
}

fn handle_packet(
    socket: &UdpSocket,
    balance: &BalanceConfig,
    rooms: &mut HashMap<GameId, GameRoom>,
    clients: &mut HashMap<SocketAddr, ClientSession>,
    next_game_id: &mut GameId,
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
            let name = clean_name(name);
            if clients
                .iter()
                .any(|(other_addr, session)| *other_addr != addr && session.name == name)
            {
                return Err("That player name is already connected.".to_string());
            }
            if let Some(session) = clients.get_mut(&addr) {
                session.last_seen = Instant::now();
                session.name = name.clone();
            } else {
                clients.insert(
                    addr,
                    ClientSession {
                        name: name.clone(),
                        game_id: None,
                        player_id: None,
                        last_seen: Instant::now(),
                        last_snapshot: None,
                        seen_enemy_buildings: HashMap::new(),
                        last_applied_seq: None,
                    },
                );
            };
            send_packet(socket, addr, &ServerPacket::Connected { name })?;
            send_balance(socket, addr, balance)?;
            send_game_list(socket, addr, rooms)?;
        }
        ClientPacket::ListGames => {
            touch_lobby(clients, addr)?;
            send_game_list(socket, addr, rooms)?;
        }
        ClientPacket::CreateGame { name } => {
            touch_lobby(clients, addr)?;
            leave_room(rooms, clients, addr);
            remove_empty_rooms(rooms);
            let game_id = *next_game_id;
            *next_game_id += 1;
            let room_name = clean_game_name(name, game_id);
            let seed = room_seed(game_id);
            let mut room = GameRoom {
                id: game_id,
                name: room_name,
                sim: GameSim::with_seed(balance.clone(), seed),
                clients: HashSet::new(),
            };
            join_room(socket, clients, addr, &mut room, balance)?;
            rooms.insert(game_id, room);
            broadcast_game_lists(socket, rooms, clients);
        }
        ClientPacket::JoinGame { game_id } => {
            touch_lobby(clients, addr)?;
            leave_room(rooms, clients, addr);
            remove_empty_rooms(rooms);
            let room = rooms
                .get_mut(&game_id)
                .ok_or_else(|| "That game no longer exists.".to_string())?;
            join_room(socket, clients, addr, room, balance)?;
            broadcast_game_lists(socket, rooms, clients);
        }
        ClientPacket::LeaveGame => {
            touch_lobby(clients, addr)?;
            leave_room(rooms, clients, addr);
            remove_empty_rooms(rooms);
            send_game_list(socket, addr, rooms)?;
            broadcast_game_lists(socket, rooms, clients);
        }
        ClientPacket::SetReady { player_id, ready } => {
            let room = room_for_player(rooms, clients, addr, player_id)?;
            room.sim.set_ready(player_id, ready)?;
        }
        ClientPacket::SetRace { player_id, race } => {
            let room = room_for_player(rooms, clients, addr, player_id)?;
            room.sim.set_race(player_id, race)?;
        }
        ClientPacket::PlaceBuilding {
            player_id,
            kind,
            lane,
            zone,
            cell,
            seq,
        } => {
            // At-most-once: a retry of an already-applied seq just re-acks.
            let duplicate = seq.is_some()
                && clients
                    .get(&addr)
                    .is_some_and(|session| session.last_applied_seq == seq);
            if duplicate {
                send_packet(socket, addr, &ServerPacket::Ack { seq })?;
                return Ok(());
            }
            let room = room_for_player(rooms, clients, addr, player_id)?;
            room.sim.place_building(player_id, kind, lane, zone, cell)?;
            if let Some(session) = clients.get_mut(&addr) {
                session.last_applied_seq = seq;
            }
            if seq.is_some() {
                send_packet(socket, addr, &ServerPacket::Ack { seq })?;
            }
        }
        ClientPacket::VoteRematch { player_id } => {
            let room = room_for_player(rooms, clients, addr, player_id)?;
            room.sim.vote_rematch(player_id);
        }
        ClientPacket::Surrender { player_id } => {
            let room = room_for_player(rooms, clients, addr, player_id)?;
            room.sim.surrender(player_id)?;
        }
        ClientPacket::Disconnect { player_id } => {
            let session = clients
                .get(&addr)
                .ok_or_else(|| "Join before disconnecting.".to_string())?;
            if session.player_id != Some(player_id) {
                return Err("Player id does not match this connection.".to_string());
            }
            disconnect_session(rooms, clients, addr);
            remove_empty_rooms(rooms);
            broadcast_game_lists(socket, rooms, clients);
        }
    }
    Ok(())
}

/// Track reserved seats for disconnected players and reset matches whose
/// grace window lapses without a reconnect. Returns true when any lobby
/// changed and game lists should be re-broadcast.
fn update_reconnect_deadlines(
    rooms: &mut HashMap<GameId, GameRoom>,
    pending: &mut ReconnectDeadlines,
    now: Instant,
) -> bool {
    let mut changed_games = false;
    for room in rooms.values_mut() {
        let waiting = room.sim.waiting_for_reconnect();
        if waiting {
            for player in &room.sim.players {
                if !player.connected {
                    pending
                        .entry((room.id, player.id))
                        .or_insert(now + RECONNECT_GRACE);
                }
            }
        } else {
            pending.retain(|(game_id, _), _| *game_id != room.id);
        }
    }
    let expired: Vec<(GameId, PlayerId)> = pending
        .iter()
        .filter(|(_, deadline)| **deadline <= now)
        .map(|(key, _)| *key)
        .collect();
    for (game_id, player_id) in expired {
        pending.remove(&(game_id, player_id));
        let Some(room) = rooms.get_mut(&game_id) else {
            continue;
        };
        let player_name = room
            .sim
            .players
            .iter()
            .find(|player| player.id == player_id)
            .map(|player| player.name.clone());
        if room.sim.waiting_for_reconnect()
            && room
                .sim
                .players
                .iter()
                .any(|player| player.id == player_id && !player.connected)
        {
            room.sim.reset_to_lobby();
            if let Some(name) = player_name {
                room.sim.message = format!("{name} failed to reconnect. Set ready to restart.");
            }
            changed_games = true;
        }
    }
    changed_games
}

fn room_seed(game_id: GameId) -> u64 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    game_id as u64 ^ nanos.rotate_left(32)
}

fn touch_lobby(
    clients: &mut HashMap<SocketAddr, ClientSession>,
    addr: SocketAddr,
) -> Result<(), String> {
    let Some(session) = clients.get_mut(&addr) else {
        return Err("Join before sending commands.".to_string());
    };
    session.last_seen = Instant::now();
    Ok(())
}

fn room_for_player<'a>(
    rooms: &'a mut HashMap<GameId, GameRoom>,
    clients: &mut HashMap<SocketAddr, ClientSession>,
    addr: SocketAddr,
    player_id: PlayerId,
) -> Result<&'a mut GameRoom, String> {
    let Some(session) = clients.get_mut(&addr) else {
        return Err("Join before sending commands.".to_string());
    };
    session.last_seen = Instant::now();
    if session.player_id != Some(player_id) {
        return Err("Player id does not match this connection.".to_string());
    }
    let game_id = session
        .game_id
        .ok_or_else(|| "Join or create a game first.".to_string())?;
    rooms
        .get_mut(&game_id)
        .ok_or_else(|| "Your game no longer exists.".to_string())
}

fn join_room(
    socket: &UdpSocket,
    clients: &mut HashMap<SocketAddr, ClientSession>,
    addr: SocketAddr,
    room: &mut GameRoom,
    balance: &BalanceConfig,
) -> Result<(), String> {
    let session = clients
        .get_mut(&addr)
        .ok_or_else(|| "Join the lobby server first.".to_string())?;
    let player = room.sim.join_or_update_player(session.name.clone())?;
    room.clients.insert(addr);
    session.game_id = Some(room.id);
    session.player_id = Some(player.id);
    session.last_snapshot = None;
    session.seen_enemy_buildings.clear();
    session.last_seen = Instant::now();
    send_packet(
        socket,
        addr,
        &ServerPacket::Welcome {
            game_id: room.id,
            player,
        },
    )?;
    send_balance(socket, addr, balance)?;
    Ok(())
}

fn leave_room(
    rooms: &mut HashMap<GameId, GameRoom>,
    clients: &mut HashMap<SocketAddr, ClientSession>,
    addr: SocketAddr,
) {
    let Some((game_id, player_id)) = clients
        .get(&addr)
        .and_then(|session| Some((session.game_id?, session.player_id?)))
    else {
        return;
    };
    if let Some(room) = rooms.get_mut(&game_id) {
        room.sim.disconnect_player(player_id);
        room.clients.remove(&addr);
    }
    if let Some(session) = clients.get_mut(&addr) {
        session.game_id = None;
        session.player_id = None;
        session.last_snapshot = None;
        session.seen_enemy_buildings.clear();
    }
}

fn clean_name(name: String) -> String {
    let cleaned = name.trim().chars().take(18).collect::<String>();
    if cleaned.is_empty() {
        "Player".to_string()
    } else {
        cleaned
    }
}

fn clean_game_name(name: String, game_id: GameId) -> String {
    let cleaned = name.trim().chars().take(32).collect::<String>();
    if cleaned.is_empty() {
        format!("Game {game_id}")
    } else {
        cleaned
    }
}

fn disconnect_session(
    rooms: &mut HashMap<GameId, GameRoom>,
    clients: &mut HashMap<SocketAddr, ClientSession>,
    addr: SocketAddr,
) {
    leave_room(rooms, clients, addr);
    clients.remove(&addr);
}

fn remove_empty_rooms(rooms: &mut HashMap<GameId, GameRoom>) {
    rooms.retain(|_, room| !room.clients.is_empty());
}

fn game_list(rooms: &HashMap<GameId, GameRoom>) -> Vec<GameInfo> {
    let mut games: Vec<GameInfo> = rooms
        .values()
        .map(|room| GameInfo {
            id: room.id,
            name: room.name.clone(),
            phase: room.sim.phase,
            players: room
                .sim
                .players
                .iter()
                .filter(|player| player.connected)
                .count(),
            max_players: 2,
        })
        .collect();
    games.sort_by_key(|game| game.id);
    games
}

fn send_game_list(
    socket: &UdpSocket,
    addr: SocketAddr,
    rooms: &HashMap<GameId, GameRoom>,
) -> Result<(), String> {
    send_packet(
        socket,
        addr,
        &ServerPacket::GameList {
            games: game_list(rooms),
        },
    )
}

fn broadcast_game_lists(
    socket: &UdpSocket,
    rooms: &HashMap<GameId, GameRoom>,
    clients: &HashMap<SocketAddr, ClientSession>,
) {
    let packet = ServerPacket::GameList {
        games: game_list(rooms),
    };
    for addr in clients.keys() {
        let _ = send_packet(socket, *addr, &packet);
    }
}

fn broadcast_rooms(
    socket: &UdpSocket,
    rooms: &mut HashMap<GameId, GameRoom>,
    clients: &mut HashMap<SocketAddr, ClientSession>,
) {
    for room in rooms.values_mut() {
        broadcast_room_snapshot(socket, clients, room);
    }
}

fn broadcast_room_snapshot(
    socket: &UdpSocket,
    clients: &mut HashMap<SocketAddr, ClientSession>,
    room: &GameRoom,
) {
    let current = room.sim.snapshot();
    for addr in &room.clients {
        let Some(session) = clients.get_mut(addr) else {
            continue;
        };
        // Authoritative fog: each client only receives entities its side can
        // see (plan.md Phase 1). The diff runs on the filtered snapshot so
        // deltas stay consistent per client.
        let viewer = session
            .player_id
            .and_then(|player_id| room.sim.player(player_id))
            .map(|player| player.team);
        let visible =
            filter_snapshot_for_viewer(&current, viewer, &mut session.seen_enemy_buildings);
        let needs_baseline = session
            .last_snapshot
            .as_ref()
            .map(|snapshot| snapshot.phase != visible.phase)
            .unwrap_or(true);

        if needs_baseline {
            send_entityless_baseline(socket, *addr, &visible, room.sim.tick);
            let mut empty = entityless_snapshot(&visible);
            empty.tick = session
                .last_snapshot
                .as_ref()
                .map(|snapshot| snapshot.tick)
                .unwrap_or(0);
            let mut delta = diff_snapshot(&empty, &visible);
            delta.clear_entities = true;
            send_delta_chunks(socket, *addr, delta, room.sim.tick);
        } else if let Some(previous) = &session.last_snapshot {
            let delta = diff_snapshot(previous, &visible);
            send_delta_chunks(socket, *addr, delta, room.sim.tick);
        }
        session.last_snapshot = Some(visible);
    }
}

fn send_entityless_baseline(
    socket: &UdpSocket,
    addr: SocketAddr,
    snapshot: &MatchSnapshot,
    tick: u64,
) {
    let packet = ServerPacket::Snapshot(entityless_snapshot(snapshot));
    if let Err(err) = send_packet(socket, addr, &packet) {
        eprintln!("baseline send failed for {addr}: {err}");
    } else {
        log_packet_size(
            "baseline",
            &packet,
            tick,
            snapshot.units.len(),
            snapshot.buildings.len(),
        );
    }
}

fn log_packet_size(label: &str, packet: &ServerPacket, tick: u64, units: usize, buildings: usize) {
    if let Ok(bytes) = encode(packet) {
        if bytes.len() > SAFE_UDP_PAYLOAD_BYTES && tick % SNAPSHOT_SIZE_LOG_INTERVAL_TICKS == 0 {
            eprintln!(
                "{label} is {} bytes for {units} units / {buildings} buildings; UDP fragmentation is likely",
                bytes.len()
            );
        }
    }
}

fn log_packet_bytes(label: &str, bytes: usize, tick: u64) {
    if bytes > SAFE_UDP_PAYLOAD_BYTES && tick % SNAPSHOT_SIZE_LOG_INTERVAL_TICKS == 0 {
        eprintln!("{label} chunk is {bytes} bytes; UDP fragmentation is likely");
    }
}

fn send_delta_chunks(socket: &UdpSocket, addr: SocketAddr, delta: SnapshotDelta, tick: u64) {
    let chunks = chunk_delta(delta);
    for chunk in chunks {
        let packet = ServerPacket::SnapshotDelta(chunk);
        match encode(&packet) {
            Ok(bytes) => {
                log_packet_bytes("delta", bytes.len(), tick);
                if bytes.len() > MAX_UDP_PAYLOAD_BYTES {
                    eprintln!(
                        "delta is {} bytes and too large for UDP; skipped chunk",
                        bytes.len()
                    );
                    continue;
                }
                let _ = socket.send_to(&bytes, addr);
            }
            Err(err) => eprintln!("delta encode failed: {err}"),
        }
    }
}

fn chunk_delta(delta: SnapshotDelta) -> Vec<SnapshotDelta> {
    let mut chunks = Vec::new();
    let mut chunk = empty_delta_like(&delta);
    chunk.clear_entities = delta.clear_entities;
    chunk.unit_removes = delta.unit_removes;
    chunk.building_removes = delta.building_removes;

    for unit in delta.unit_creates {
        chunk.unit_creates.push(unit);
        if encoded_delta_len(&chunk) > SAFE_UDP_PAYLOAD_BYTES && delta_entity_count(&chunk) > 1 {
            let unit = chunk.unit_creates.pop().expect("just pushed");
            flush_delta_chunk(&mut chunks, &mut chunk);
            chunk.unit_creates.push(unit);
        }
    }
    for unit in delta.unit_updates {
        chunk.unit_updates.push(unit);
        if encoded_delta_len(&chunk) > SAFE_UDP_PAYLOAD_BYTES && delta_entity_count(&chunk) > 1 {
            let unit = chunk.unit_updates.pop().expect("just pushed");
            flush_delta_chunk(&mut chunks, &mut chunk);
            chunk.unit_updates.push(unit);
        }
    }
    for building in delta.building_creates {
        chunk.building_creates.push(building);
        if encoded_delta_len(&chunk) > SAFE_UDP_PAYLOAD_BYTES && delta_entity_count(&chunk) > 1 {
            let building = chunk.building_creates.pop().expect("just pushed");
            flush_delta_chunk(&mut chunks, &mut chunk);
            chunk.building_creates.push(building);
        }
    }
    for building in delta.building_updates {
        chunk.building_updates.push(building);
        if encoded_delta_len(&chunk) > SAFE_UDP_PAYLOAD_BYTES && delta_entity_count(&chunk) > 1 {
            let building = chunk.building_updates.pop().expect("just pushed");
            flush_delta_chunk(&mut chunks, &mut chunk);
            chunk.building_updates.push(building);
        }
    }

    chunks.push(chunk);
    chunks
}

fn flush_delta_chunk(chunks: &mut Vec<SnapshotDelta>, chunk: &mut SnapshotDelta) {
    let next = empty_delta_like(chunk);
    let flushed = std::mem::replace(chunk, next);
    chunk.clear_entities = false;
    chunks.push(flushed);
}

fn encoded_delta_len(delta: &SnapshotDelta) -> usize {
    encode(&ServerPacket::SnapshotDelta(delta.clone()))
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX)
}

fn delta_entity_count(delta: &SnapshotDelta) -> usize {
    delta.unit_creates.len()
        + delta.unit_updates.len()
        + delta.building_creates.len()
        + delta.building_updates.len()
}

fn empty_delta_like(delta: &SnapshotDelta) -> SnapshotDelta {
    SnapshotDelta {
        clear_entities: false,
        phase: delta.phase,
        tick: delta.tick,
        elapsed_secs: delta.elapsed_secs,
        sudden_death: delta.sudden_death,
        players: delta.players.clone(),
        economies: delta.economies.clone(),
        castles: delta.castles.clone(),
        bounty_events: delta.bounty_events.clone(),
        winner: delta.winner,
        message: delta.message.clone(),
        unit_creates: Vec::new(),
        unit_updates: Vec::new(),
        unit_removes: Vec::new(),
        building_creates: Vec::new(),
        building_updates: Vec::new(),
        building_removes: Vec::new(),
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
