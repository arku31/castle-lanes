use crate::sim::{
    BuildZone, Building, BuildingConfig, BuildingKind, Castle, Economy, GridCell, Lane, MatchPhase,
    MatchSnapshot, PlayerId, PlayerInfo, RaceConfig, RaceKind, Team, Unit, UnitConfig, WorldPos,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

pub const DEFAULT_SERVER_ADDR: &str = "127.0.0.1:4000";
pub const PROTOCOL_VERSION: u16 = 5;
pub type GameId = u32;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientPacket {
    Join {
        version: u16,
        name: String,
    },
    ListGames,
    CreateGame {
        name: String,
    },
    JoinGame {
        game_id: GameId,
    },
    LeaveGame,
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
    },
    VoteRematch {
        player_id: PlayerId,
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
    pub economies: [Economy; 2],
    pub castles: [Castle; 2],
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

pub fn encode<T: Serialize>(packet: &T) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec(packet)
}

pub fn decode_client(bytes: &[u8]) -> Result<ClientPacket, serde_json::Error> {
    serde_json::from_slice(bytes)
}

pub fn decode_server(bytes: &[u8]) -> Result<ServerPacket, serde_json::Error> {
    serde_json::from_slice(bytes)
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
            owner: Team::Left,
            kind: UnitKind::VanguardGuard,
            lane: Lane::Top,
            health: 100,
            lane_pos: 20.0,
            pos: crate::sim::lane_position(Lane::Top, 20.0),
            velocity: crate::sim::WorldPos::new(0.0, 0.0),
            radius: sim.balance.unit(UnitKind::VanguardGuard).radius,
            attack_timer: 0.4,
        });
        let previous = sim.snapshot();

        sim.units[0].health = 75;
        sim.units[0].lane_pos = 22.0;
        sim.units[0].pos = crate::sim::lane_position(Lane::Top, 22.0);
        sim.units.push(crate::sim::Unit {
            id: 901,
            owner: Team::Right,
            kind: UnitKind::EmberRunner,
            lane: Lane::Top,
            health: 80,
            lane_pos: 23.0,
            pos: crate::sim::lane_position(Lane::Top, 23.0),
            velocity: crate::sim::WorldPos::new(0.0, 0.0),
            radius: sim.balance.unit(UnitKind::EmberRunner).radius,
            attack_timer: 0.2,
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
}
