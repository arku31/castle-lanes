use crate::sim::{
    BuildZone, BuildingConfig, BuildingKind, GridCell, Lane, MatchSnapshot, PlayerId, PlayerInfo,
    RaceConfig, RaceKind, UnitConfig,
};
use serde::{Deserialize, Serialize};

pub const DEFAULT_SERVER_ADDR: &str = "127.0.0.1:4000";
pub const PROTOCOL_VERSION: u16 = 4;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientPacket {
    Join {
        version: u16,
        name: String,
    },
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
    Welcome {
        player: PlayerInfo,
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
    Error {
        message: String,
    },
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
