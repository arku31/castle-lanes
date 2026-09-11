use crate::sim::{
    BuildZone, Building, BuildingConfig, BuildingKind, Castle, Economy, GridCell, Lane, MatchPhase,
    MatchSnapshot, PlayerId, PlayerInfo, RaceConfig, RaceKind, Team, Unit, UnitConfig, WorldPos,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU8, Ordering};

/// Default match server for plain `castle_lanes_client` launches (the hosted
/// test server). Override per launch with `--server host:port`.
pub const DEFAULT_SERVER_ADDR: &str = "127.0.0.1:4000";
pub const PROTOCOL_VERSION: u16 = 9;
pub type GameId = u32;

/// Wire format tag. Bincode is the default (5-10x smaller than JSON);
/// `--legacy-json` switches a peer to tagged JSON for debugging/migration.
/// Decode auto-detects: byte 0 = JSON, byte 1 = bincode; anything else is
/// retried as untagged JSON so pre-tagging peers still decode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacketFormat {
    Json,
    Bincode,
}

impl PacketFormat {
    pub fn tag(self) -> u8 {
        match self {
            Self::Json => 0,
            Self::Bincode => 1,
        }
    }

    fn from_tag(tag: u8) -> Option<Self> {
        match tag {
            0 => Some(Self::Json),
            1 => Some(Self::Bincode),
            _ => None,
        }
    }
}

static OUTGOING_FORMAT: AtomicU8 = AtomicU8::new(PacketFormat::Bincode as u8);

/// Set the outgoing wire format; incoming is always auto-detected.
pub fn set_outgoing_format(format: PacketFormat) {
    OUTGOING_FORMAT.store(format as u8, Ordering::Relaxed);
}

pub fn outgoing_format() -> PacketFormat {
    match OUTGOING_FORMAT.load(Ordering::Relaxed) {
        0 => PacketFormat::Json,
        _ => PacketFormat::Bincode,
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PacketError {
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("bincode: {0}")]
    Bincode(#[from] bincode::Error),
    #[error("unknown packet format tag")]
    UnknownFormat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientPacket {
    Join {
        version: u16,
        name: String,
    },
    ListGames,
    CreateGame {
        name: String,
        #[serde(default)]
        team_size: Option<usize>,
        #[serde(default)]
        random_factions: bool,
    },
    JoinGame {
        game_id: GameId,
        /// Join as a read-only spectator (no seat consumed).
        #[serde(default)]
        spectator: bool,
    },
    LeaveGame,
    /// Join the matchmaking queue for a game with this team size.
    QueueForMatch {
        team_size: usize,
    },
    /// Leave the matchmaking queue.
    LeaveQueue,
    SetReady {
        player_id: PlayerId,
        ready: bool,
    },
    SetRace {
        player_id: PlayerId,
        race: RaceKind,
    },
    PlaceBuilding {
        player_id: PlayerId,
        kind: BuildingKind,
        lane: Lane,
        zone: BuildZone,
        cell: GridCell,
        /// Client-generated id for at-most-once application; the server
        /// echoes it in `ServerPacket::Ack` (plan.md Phase 1 command acks).
        #[serde(default)]
        seq: Option<u32>,
    },
    VoteRematch {
        player_id: PlayerId,
    },
    Surrender {
        player_id: PlayerId,
    },
    SellBuilding {
        player_id: PlayerId,
        building_id: u64,
        #[serde(default)]
        seq: Option<u32>,
    },
    UpgradeBuilding {
        player_id: PlayerId,
        building_id: u64,
        to: BuildingKind,
        #[serde(default)]
        seq: Option<u32>,
    },
    Disconnect {
        player_id: PlayerId,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerPacket {
    Connected {
        name: String,
    },
    Welcome {
        game_id: GameId,
        player: PlayerInfo,
    },
    GameList {
        games: Vec<GameInfo>,
    },
    QueueStatus {
        queued: usize,
        needed: usize,
        team_size: usize,
    },
    BalanceRaces {
        starting_gold: i32,
        base_income: i32,
        income_interval: f32,
        interest_rate: f32,
        #[serde(default = "default_castle_regen_per_second")]
        castle_regen_per_second: f32,
        sudden_death_start: f32,
        races: Vec<RaceConfig>,
    },
    BalanceBuildings {
        buildings: Vec<BuildingConfig>,
    },
    BalanceUnits {
        units: Vec<UnitConfig>,
    },
    Snapshot(MatchSnapshot),
    SnapshotDelta(SnapshotDelta),
    Ack {
        #[serde(default)]
        seq: Option<u32>,
    },
    ProfileData {
        wins: u32,
        losses: u32,
    },
    Error {
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameInfo {
    pub id: GameId,
    pub name: String,
    pub phase: MatchPhase,
    pub players: usize,
    pub max_players: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotDelta {
    pub clear_entities: bool,
    pub phase: MatchPhase,
    pub tick: u64,
    pub elapsed_secs: f32,
    pub sudden_death: bool,
    pub players: Vec<PlayerInfo>,
    #[serde(default)]
    pub economies: Vec<Economy>,
    #[serde(default)]
    pub castles: Vec<Castle>,
    pub bounty_events: Vec<crate::sim::BountyEvent>,
    pub winner: Option<Team>,
    pub message: String,
    pub unit_creates: Vec<Unit>,
    pub unit_updates: Vec<UnitUpdate>,
    pub unit_removes: Vec<u64>,
    pub building_creates: Vec<Building>,
    pub building_updates: Vec<BuildingUpdate>,
    pub building_removes: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnitUpdate {
    pub id: u64,
    pub health: i32,
    pub lane_pos: f32,
    pub pos: WorldPos,
    pub velocity: WorldPos,
    pub attack_timer: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuildingUpdate {
    pub id: u64,
    pub health: i32,
    pub spawn_timer: f32,
}

impl UnitUpdate {
    fn from_unit(unit: &Unit) -> Self {
        Self {
            id: unit.id,
            health: unit.health,
            lane_pos: unit.lane_pos,
            pos: unit.pos,
            velocity: unit.velocity,
            attack_timer: unit.attack_timer,
        }
    }

    fn apply_to(self, unit: &mut Unit) {
        unit.health = self.health;
        unit.lane_pos = self.lane_pos;
        unit.pos = self.pos;
        unit.velocity = self.velocity;
        unit.attack_timer = self.attack_timer;
    }
}

impl BuildingUpdate {
    fn from_building(building: &Building) -> Self {
        Self {
            id: building.id,
            health: building.health,
            spawn_timer: building.spawn_timer,
        }
    }

    fn apply_to(self, building: &mut Building) {
        building.health = self.health;
        building.spawn_timer = self.spawn_timer;
    }
}

impl SnapshotDelta {
    pub fn empty_from(snapshot: &MatchSnapshot) -> Self {
        Self {
            clear_entities: false,
            phase: snapshot.phase,
            tick: snapshot.tick,
            elapsed_secs: snapshot.elapsed_secs,
            sudden_death: snapshot.sudden_death,
            players: snapshot.players.clone(),
            economies: snapshot.economies.clone(),
            castles: snapshot.castles.clone(),
            bounty_events: snapshot.bounty_events.clone(),
            winner: snapshot.winner,
            message: snapshot.message.clone(),
            unit_creates: Vec::new(),
            unit_updates: Vec::new(),
            unit_removes: Vec::new(),
            building_creates: Vec::new(),
            building_updates: Vec::new(),
            building_removes: Vec::new(),
        }
    }

    pub fn has_entity_changes(&self) -> bool {
        self.clear_entities
            || !self.unit_creates.is_empty()
            || !self.unit_updates.is_empty()
            || !self.unit_removes.is_empty()
            || !self.building_creates.is_empty()
            || !self.building_updates.is_empty()
            || !self.building_removes.is_empty()
    }
}

/// Filter a snapshot down to what one side is allowed to know (plan.md
/// Phase 1): own entities always, enemy units only inside current vision,
/// enemy buildings only if currently visible or seen before (sent stale so
/// clients render tombstone silhouettes), and bounty events only for the
/// viewer's own side. With no viewer (lobby), no entities are sent. The
/// final snapshot of a match reveals everything.
pub fn filter_snapshot_for_viewer(
    snapshot: &MatchSnapshot,
    viewer: Option<Team>,
    seen_enemy_buildings: &mut HashMap<u64, crate::sim::Building>,
) -> MatchSnapshot {
    let mut filtered = snapshot.clone();
    let Some(viewer) = viewer else {
        filtered.units.clear();
        filtered.buildings.clear();
        filtered.bounty_events.clear();
        return filtered;
    };
    if snapshot.phase == MatchPhase::GameOver {
        seen_enemy_buildings.clear();
        return filtered;
    }

    let sides: HashMap<u64, Team> = snapshot
        .players
        .iter()
        .map(|player| (player.id.0 as u64, player.team))
        .collect();
    let side_of = |owner: crate::sim::PlayerId| -> Team {
        sides.get(&(owner.0 as u64)).copied().unwrap_or(Team::Left)
    };

    filtered.units.retain(|unit| {
        side_of(unit.owner) == viewer
            || crate::sim::position_revealed_to(
                &snapshot.players,
                &snapshot.buildings,
                &snapshot.units,
                viewer,
                unit.pos,
            )
    });

    let mut kept_buildings: Vec<crate::sim::Building> = Vec::new();
    for building in &snapshot.buildings {
        if side_of(building.owner) == viewer {
            kept_buildings.push(building.clone());
            continue;
        }
        let position = crate::sim::building_position(
            side_of(building.owner),
            building.lane,
            building.zone,
            building.cell,
        );
        if crate::sim::position_revealed_to(
            &snapshot.players,
            &snapshot.buildings,
            &snapshot.units,
            viewer,
            position,
        ) {
            seen_enemy_buildings.insert(building.id, building.clone());
            kept_buildings.push(building.clone());
        } else if let Some(stale) = seen_enemy_buildings.get(&building.id) {
            kept_buildings.push(stale.clone());
        }
    }
    filtered.buildings = kept_buildings;
    seen_enemy_buildings
        .retain(|id, _| snapshot.buildings.iter().any(|building| building.id == *id));

    filtered.bounty_events.retain(|event| event.team == viewer);
    filtered
}

pub fn entityless_snapshot(snapshot: &MatchSnapshot) -> MatchSnapshot {
    let mut snapshot = snapshot.clone();
    snapshot.units.clear();
    snapshot.buildings.clear();
    snapshot
}

pub fn diff_snapshot(previous: &MatchSnapshot, current: &MatchSnapshot) -> SnapshotDelta {
    let mut delta = SnapshotDelta::empty_from(current);

    let previous_units: HashMap<u64, &Unit> =
        previous.units.iter().map(|unit| (unit.id, unit)).collect();
    let current_unit_ids: HashSet<u64> = current.units.iter().map(|unit| unit.id).collect();
    for unit in &current.units {
        if let Some(previous) = previous_units.get(&unit.id) {
            if unit_static_changed(previous, unit) {
                delta.unit_creates.push(unit.clone());
            } else {
                let update = UnitUpdate::from_unit(unit);
                if update != UnitUpdate::from_unit(previous) {
                    delta.unit_updates.push(update);
                }
            }
        } else {
            delta.unit_creates.push(unit.clone());
        }
    }
    delta.unit_removes = previous
        .units
        .iter()
        .filter(|unit| !current_unit_ids.contains(&unit.id))
        .map(|unit| unit.id)
        .collect();

    let previous_buildings: HashMap<u64, &Building> = previous
        .buildings
        .iter()
        .map(|building| (building.id, building))
        .collect();
    let current_building_ids: HashSet<u64> = current
        .buildings
        .iter()
        .map(|building| building.id)
        .collect();
    for building in &current.buildings {
        if let Some(previous) = previous_buildings.get(&building.id) {
            if building_static_changed(previous, building) {
                delta.building_creates.push(building.clone());
            } else {
                let update = BuildingUpdate::from_building(building);
                if update != BuildingUpdate::from_building(previous) {
                    delta.building_updates.push(update);
                }
            }
        } else {
            delta.building_creates.push(building.clone());
        }
    }
    delta.building_removes = previous
        .buildings
        .iter()
        .filter(|building| !current_building_ids.contains(&building.id))
        .map(|building| building.id)
        .collect();

    delta
}

pub fn apply_snapshot_delta(snapshot: &mut MatchSnapshot, delta: SnapshotDelta) {
    snapshot.phase = delta.phase;
    snapshot.tick = delta.tick;
    snapshot.elapsed_secs = delta.elapsed_secs;
    snapshot.sudden_death = delta.sudden_death;
    snapshot.players = delta.players;
    snapshot.economies = delta.economies;
    snapshot.castles = delta.castles;
    snapshot.bounty_events = delta.bounty_events;
    snapshot.winner = delta.winner;
    snapshot.message = delta.message;

    if delta.clear_entities {
        snapshot.units.clear();
        snapshot.buildings.clear();
    }

    let removed_units: HashSet<u64> = delta.unit_removes.into_iter().collect();
    snapshot
        .units
        .retain(|unit| !removed_units.contains(&unit.id));
    for unit in delta.unit_creates {
        upsert_unit(&mut snapshot.units, unit);
    }
    for update in delta.unit_updates {
        if let Some(unit) = snapshot.units.iter_mut().find(|unit| unit.id == update.id) {
            update.apply_to(unit);
        }
    }

    let removed_buildings: HashSet<u64> = delta.building_removes.into_iter().collect();
    snapshot
        .buildings
        .retain(|building| !removed_buildings.contains(&building.id));
    for building in delta.building_creates {
        upsert_building(&mut snapshot.buildings, building);
    }
    for update in delta.building_updates {
        if let Some(building) = snapshot
            .buildings
            .iter_mut()
            .find(|building| building.id == update.id)
        {
            update.apply_to(building);
        }
    }
}

pub fn encode<T: Serialize>(packet: &T) -> Result<Vec<u8>, PacketError> {
    encode_as(packet, outgoing_format())
}

pub fn encode_as<T: Serialize + ?Sized>(
    packet: &T,
    format: PacketFormat,
) -> Result<Vec<u8>, PacketError> {
    let payload = match format {
        PacketFormat::Json => serde_json::to_vec(packet)?,
        PacketFormat::Bincode => bincode::serialize(packet)?,
    };
    let mut tagged = Vec::with_capacity(payload.len() + 1);
    tagged.push(format.tag());
    tagged.extend_from_slice(&payload);
    Ok(tagged)
}

pub fn decode_client(bytes: &[u8]) -> Result<ClientPacket, PacketError> {
    decode_any(bytes)
}

pub fn decode_server(bytes: &[u8]) -> Result<ServerPacket, PacketError> {
    decode_any(bytes)
}

fn decode_any<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, PacketError> {
    let (&tag, payload) = bytes.split_first().ok_or(PacketError::UnknownFormat)?;
    match PacketFormat::from_tag(tag) {
        Some(PacketFormat::Json) => Ok(serde_json::from_slice(payload)?),
        Some(PacketFormat::Bincode) => Ok(bincode::deserialize(payload)?),
        None => {
            // Untagged buffer: legacy peer speaking raw JSON.
            Ok(serde_json::from_slice(bytes)?)
        }
    }
}

fn default_castle_regen_per_second() -> f32 {
    crate::sim::CASTLE_REGEN_PER_SECOND
}

fn unit_static_changed(previous: &Unit, current: &Unit) -> bool {
    previous.owner != current.owner
        || previous.kind != current.kind
        || previous.lane != current.lane
        || previous.radius != current.radius
}

fn building_static_changed(previous: &Building, current: &Building) -> bool {
    previous.owner != current.owner
        || previous.kind != current.kind
        || previous.lane != current.lane
        || previous.zone != current.zone
        || previous.cell != current.cell
        || previous.max_health != current.max_health
}

fn upsert_unit(units: &mut Vec<Unit>, unit: Unit) {
    if let Some(existing) = units.iter_mut().find(|existing| existing.id == unit.id) {
        *existing = unit;
    } else {
        units.push(unit);
    }
}

fn upsert_building(buildings: &mut Vec<Building>, building: Building) {
    if let Some(existing) = buildings
        .iter_mut()
        .find(|existing| existing.id == building.id)
    {
        *existing = building;
    } else {
        buildings.push(building);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::{GameSim, Lane, MatchPhase, RaceKind, Team, UnitKind};

    #[test]
    fn snapshot_delta_reconstructs_current_snapshot() {
        let mut sim = ready_two_players();
        sim.units.push(crate::sim::Unit {
            id: 900,
            owner: crate::sim::PlayerId(1),
            kind: UnitKind::VanguardGuard,
            lane: Lane::Top,
            health: 100,
            lane_pos: 20.0,
            pos: crate::sim::lane_position(Lane::Top, 20.0),
            velocity: crate::sim::WorldPos::new(0.0, 0.0),
            radius: sim.balance.unit(UnitKind::VanguardGuard).radius,
            attack_timer: 0.4,
            slow_timer: 0.0,
            slow_factor: 1.0,
            regen_accum: 0.0,
        });
        let previous = sim.snapshot();

        sim.units[0].health = 75;
        sim.units[0].lane_pos = 22.0;
        sim.units[0].pos = crate::sim::lane_position(Lane::Top, 22.0);
        sim.units.push(crate::sim::Unit {
            id: 901,
            owner: crate::sim::PlayerId(2),
            kind: UnitKind::EmberRunner,
            lane: Lane::Top,
            health: 80,
            lane_pos: 23.0,
            pos: crate::sim::lane_position(Lane::Top, 23.0),
            velocity: crate::sim::WorldPos::new(0.0, 0.0),
            radius: sim.balance.unit(UnitKind::EmberRunner).radius,
            attack_timer: 0.2,
            slow_timer: 0.0,
            slow_factor: 1.0,
            regen_accum: 0.0,
        });
        let current = sim.snapshot();

        let delta = diff_snapshot(&previous, &current);
        assert_eq!(delta.unit_creates.len(), 1);
        assert_eq!(delta.unit_updates.len(), 1);

        let mut reconstructed = previous;
        apply_snapshot_delta(&mut reconstructed, delta);

        assert_eq!(reconstructed, current);
        assert_eq!(reconstructed.phase, MatchPhase::Playing);
    }

    fn ready_two_players() -> GameSim {
        let mut sim = GameSim::default();
        let left = sim.join_or_update_player("Alice".to_string()).unwrap().id;
        let right = sim.join_or_update_player("Bryn".to_string()).unwrap().id;
        sim.set_race(left, RaceKind::Vanguard).unwrap();
        sim.set_race(right, RaceKind::Vanguard).unwrap();
        sim.set_ready(left, true).unwrap();
        sim.set_ready(right, true).unwrap();
        sim
    }

    #[test]
    fn packet_formats_round_trip_and_cross_decode() {
        let packet = ClientPacket::PlaceBuilding {
            player_id: PlayerId(1),
            kind: crate::sim::BuildingKind::VanguardBarracks,
            lane: Lane::Top,
            zone: crate::sim::BuildZone::Front,
            cell: crate::sim::GridCell { x: 2, y: 3 },
            seq: Some(11),
        };

        let bincode_bytes = encode_as(&packet, PacketFormat::Bincode).unwrap();
        let json_bytes = encode_as(&packet, PacketFormat::Json).unwrap();
        assert_eq!(bincode_bytes[0], 1);
        assert_eq!(json_bytes[0], 0);
        assert!(
            bincode_bytes.len() * 3 < json_bytes.len(),
            "bincode ({} B) should be far smaller than json ({} B)",
            bincode_bytes.len(),
            json_bytes.len()
        );

        // Both formats decode through the same seam regardless of the
        // outgoing setting.
        let decoded_bincode: ClientPacket = decode_client(&bincode_bytes).unwrap();
        let decoded_json: ClientPacket = decode_client(&json_bytes).unwrap();
        assert_eq!(
            serde_json::to_string(&decoded_bincode).unwrap(),
            serde_json::to_string(&decoded_json).unwrap()
        );

        // Untagged raw JSON (legacy peers) still decodes.
        let raw_json = serde_json::to_vec(&packet).unwrap();
        let decoded_legacy: ClientPacket = decode_client(&raw_json).unwrap();
        assert_eq!(
            serde_json::to_string(&decoded_legacy).unwrap(),
            serde_json::to_string(&packet).unwrap()
        );

        let snapshot = ready_two_players().snapshot();
        let packet = ServerPacket::Snapshot(snapshot.clone());
        let snapshot_bincode = encode_as(&packet, PacketFormat::Bincode).unwrap();
        let decoded = match decode_server(&snapshot_bincode).unwrap() {
            ServerPacket::Snapshot(snapshot) => snapshot,
            other => panic!("expected snapshot packet, got {other:?}"),
        };
        assert_eq!(decoded, snapshot);
    }

    #[test]
    fn filtering_hides_unseen_enemies_and_stales_seen_buildings() {
        let mut sim = ready_two_players();
        let left = sim.players[0].id;
        let _ = left;
        // Left barracks mid-top, right spire at right's home lane, units apart.
        sim.place_building(
            sim.players[0].id,
            crate::sim::BuildingKind::VanguardBarracks,
            Lane::Top,
            crate::sim::BuildZone::Front,
            crate::sim::GridCell { x: 0, y: 0 },
        )
        .unwrap();
        sim.place_building(
            sim.players[1].id,
            crate::sim::BuildingKind::VanguardRangeTower,
            Lane::Top,
            crate::sim::BuildZone::Front,
            crate::sim::GridCell { x: 0, y: 0 },
        )
        .unwrap();
        sim.units.push(crate::sim::Unit {
            id: 900,
            owner: crate::sim::PlayerId(1),
            kind: UnitKind::VanguardGuard,
            lane: Lane::Top,
            health: 100,
            lane_pos: 50.0,
            pos: crate::sim::lane_position(Lane::Top, 50.0),
            velocity: crate::sim::WorldPos::new(0.0, 0.0),
            radius: 0.65,
            attack_timer: 0.0,
            slow_timer: 0.0,
            slow_factor: 1.0,
            regen_accum: 0.0,
        });
        sim.units.push(crate::sim::Unit {
            id: 901,
            owner: crate::sim::PlayerId(2),
            kind: UnitKind::VanguardGuard,
            lane: Lane::Top,
            health: 100,
            lane_pos: 95.0,
            pos: crate::sim::lane_position(Lane::Top, 95.0),
            velocity: crate::sim::WorldPos::new(0.0, 0.0),
            radius: 0.65,
            attack_timer: 0.0,
            slow_timer: 0.0,
            slow_factor: 1.0,
            regen_accum: 0.0,
        });
        let snapshot = sim.snapshot();
        let mut seen = std::collections::HashMap::new();

        let visible = filter_snapshot_for_viewer(&snapshot, Some(Team::Left), &mut seen);
        // Own unit and own building present; enemy unit at 95 and unseen
        // enemy building hidden.
        assert!(visible.units.iter().any(|unit| unit.id == 900));
        assert!(!visible.units.iter().any(|unit| unit.id == 901));
        assert_eq!(visible.buildings.len(), 1);
        assert!(visible.bounty_events.is_empty());

        // Scout the enemy building: becomes visible, then stays stale after
        // the scouting unit leaves.
        sim.units[0].pos = crate::sim::lane_position(Lane::Top, 88.0);
        let scouted = sim.snapshot();
        let scouted = filter_snapshot_for_viewer(&scouted, Some(Team::Left), &mut seen);
        assert_eq!(scouted.buildings.len(), 2);
        assert!(scouted.units.iter().any(|unit| unit.id == 901));

        sim.units[0].pos = crate::sim::lane_position(Lane::Top, 50.0);
        sim.units[1].pos = crate::sim::lane_position(Lane::Top, 95.0);
        let stale = sim.snapshot();
        let stale = filter_snapshot_for_viewer(&stale, Some(Team::Left), &mut seen);
        assert_eq!(
            stale.buildings.len(),
            2,
            "seen building stays as stale copy"
        );
        assert!(!stale.units.iter().any(|unit| unit.id == 901));

        // No viewer: nothing leaks.
        let empty = filter_snapshot_for_viewer(&stale, None, &mut seen);
        assert!(empty.units.is_empty() && empty.buildings.is_empty());

        // Destroyed buildings are pruned from the seen set.
        sim.buildings
            .retain(|building| building.owner == crate::sim::PlayerId(1));
        let destroyed = sim.snapshot();
        let destroyed = filter_snapshot_for_viewer(&destroyed, Some(Team::Left), &mut seen);
        assert_eq!(destroyed.buildings.len(), 1);
        assert!(seen.is_empty());
    }
}
