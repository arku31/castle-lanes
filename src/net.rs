use crate::sim::{BuildingKind, GridCell, MatchSnapshot, PlayerId, PlayerInfo, RaceKind};
use serde::{Deserialize, Serialize};

pub const DEFAULT_SERVER_ADDR: &str = "127.0.0.1:4000";
pub const PROTOCOL_VERSION: u16 = 1;

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
    Welcome { player: PlayerInfo },
    Snapshot(MatchSnapshot),
    Error { message: String },
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
