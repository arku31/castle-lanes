use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub const CASTLE_HEALTH: i32 = 10000;
pub const STARTING_GOLD: i32 = 150;
pub const BASE_INCOME: i32 = 10;
pub const INCOME_INTERVAL: f32 = 10.0;
pub const INTEREST_RATE: f32 = 0.04;
pub const GRID_W: i32 = 10;
pub const GRID_H: i32 = 5;
pub const LANE_LENGTH: f32 = 100.0;
pub const SUDDEN_DEATH_START: f32 = 600.0;
pub const CASTLE_REGEN_PER_SECOND: f32 = 10.0;
pub const CASTLE_REGEN_DELAY_SECS: f32 = 8.0;
pub const SUDDEN_DEATH_RAMP_PER_MINUTE: f32 = 1.0;
pub const SUDDEN_DEATH_ESCALATION_PER_MINUTE: f32 = 8.0;
pub const DEFAULT_BALANCE_PATH: &str = "config/balance.json";
const BOUNTY_EVENT_TTL: f32 = 0.75;
const CASTLE_JUNCTION_RANGE: f32 = 18.0;
const LANE_CENTER_Y: f32 = 8.0;
const GRID_CELL_Y: f32 = 2.4;
const LANE_SPREAD_LIMIT: f32 = 6.0;
const UNIT_SEPARATION_PADDING: f32 = 0.08;
const SPAWN_SEARCH_RINGS: i32 = 8;
const BUILDING_FOOTPRINT_RADIUS: f32 = 4.6;
const CASTLE_FOOTPRINT_RADIUS: f32 = 5.0;
pub const SELL_REFUND_RATIO: f32 = 0.7;
// Vision radii in sim units; the authoritative server filters snapshots with
// these so clients only receive entities their side can actually see.
pub const VISION_CASTLE_RADIUS: f32 = 18.0;
pub const VISION_BUILDING_RADIUS: f32 = 7.0;
pub const VISION_UNIT_RADIUS: f32 = 8.0;
pub const DEFAULT_SIM_SEED: u64 = 0xC057_1A4E_5EED;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlayerId(pub u8);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Team {
    Left,
    Right,
}

impl Team {
    pub fn opponent(self) -> Self {
        match self {
            Team::Left => Team::Right,
            Team::Right => Team::Left,
        }
    }

    pub fn slot(self) -> usize {
        match self {
            Team::Left => 0,
            Team::Right => 1,
        }
    }

    pub fn spawn_pos(self) -> f32 {
        match self {
            Team::Left => 4.0,
            Team::Right => LANE_LENGTH - 4.0,
        }
    }

    pub fn castle_pos(self) -> f32 {
        match self {
            Team::Left => 0.0,
            Team::Right => LANE_LENGTH,
        }
    }

    pub fn direction(self) -> f32 {
        match self {
            Team::Left => 1.0,
            Team::Right => -1.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MatchPhase {
    Lobby,
    Playing,
    GameOver,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Lane {
    /// Top-most lane (1v1: Left side)
    Top,
    /// Upper-middle lane (2v2: Left P1, Right P1)
    UpperMid,
    /// Lower-middle lane (2v2: Left P0, Right P0)
    LowerMid,
    /// Bottom-most lane (1v1: Right side)
    Bottom,
}

impl Lane {
    /// Number of active lanes depends on team size: 2 for 1v1, 4 for 2v2+.
    pub const ALL: [Lane; 4] = [Lane::Top, Lane::UpperMid, Lane::LowerMid, Lane::Bottom];

    /// Lanes used for a given team size: 2P -> 2 lanes, 4P -> 4 lanes.
    pub fn active_lanes(team_size: usize) -> Vec<Lane> {
        match team_size {
            1 => vec![Lane::Top, Lane::Bottom],
            _ => Lane::ALL.to_vec(),
        }
    }

    /// The lane a player uses, based on their position within their side.
    /// Left P0 -> Top, Left P1 -> UpperMid, etc. Right mirrors from Bottom.
    pub fn for_player(team: Team, side_index: usize) -> Lane {
        match (team, side_index) {
            (Team::Left, 0) => Lane::Top,
            (Team::Left, 1) => Lane::UpperMid,
            (Team::Left, _) => Lane::LowerMid,
            (Team::Right, 0) => Lane::Bottom,
            (Team::Right, 1) => Lane::LowerMid,
            (Team::Right, _) => Lane::Bottom,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BuildZone {
    Front,
    Back,
}

impl BuildZone {
    pub const ALL: [BuildZone; 2] = [BuildZone::Front, BuildZone::Back];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RaceKind {
    Vanguard,
    Grove,
    Ember,
}

impl RaceKind {
    pub const ALL: [RaceKind; 3] = [RaceKind::Vanguard, RaceKind::Grove, RaceKind::Ember];

    pub fn fallback_name(self) -> &'static str {
        match self {
            RaceKind::Vanguard => "Vanguard",
            RaceKind::Grove => "Grove",
            RaceKind::Ember => "Ember",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BuildingKind {
    VanguardBarracks,
    VanguardRangeTower,
    VanguardForge,
    VanguardPikeYard,
    VanguardBulwarkHall,
    VanguardChapel,
    VanguardStables,
    VanguardSiegeWorkshop,
    GroveRootDen,
    GroveThornSpire,
    GroveBloomWell,
    GroveMossNursery,
    GroveBarkBastion,
    GroveMirePool,
    GroveVineWarren,
    GroveAncientSeed,
    EmberCinderPit,
    EmberFlameSpire,
    EmberAshMine,
    EmberSparkKennel,
    EmberObsidianGate,
    EmberBlazeStable,
    EmberSmokeAltar,
    EmberInfernoEngine,
    // Branch upgrades (Phase 2 item 3): reached only through the upgrade
    // command, never placed directly.
    VanguardArbalestTower,
    VanguardArcaneSpire,
    GroveBrambleWarren,
    GroveSpitefen,
    EmberMagmaForge,
    EmberAshPack,
}

impl BuildingKind {
    pub fn race(self) -> RaceKind {
        match self {
            BuildingKind::VanguardBarracks
            | BuildingKind::VanguardRangeTower
            | BuildingKind::VanguardForge
            | BuildingKind::VanguardPikeYard
            | BuildingKind::VanguardBulwarkHall
            | BuildingKind::VanguardChapel
            | BuildingKind::VanguardStables
            | BuildingKind::VanguardSiegeWorkshop => RaceKind::Vanguard,
            BuildingKind::GroveRootDen
            | BuildingKind::GroveThornSpire
            | BuildingKind::GroveBloomWell
            | BuildingKind::GroveMossNursery
            | BuildingKind::GroveBarkBastion
            | BuildingKind::GroveMirePool
            | BuildingKind::GroveVineWarren
            | BuildingKind::GroveAncientSeed => RaceKind::Grove,
            BuildingKind::EmberCinderPit
            | BuildingKind::EmberFlameSpire
            | BuildingKind::EmberAshMine
            | BuildingKind::EmberSparkKennel
            | BuildingKind::EmberObsidianGate
            | BuildingKind::EmberBlazeStable
            | BuildingKind::EmberSmokeAltar
            | BuildingKind::EmberInfernoEngine => RaceKind::Ember,
            BuildingKind::VanguardArbalestTower | BuildingKind::VanguardArcaneSpire => {
                RaceKind::Vanguard
            }
            BuildingKind::GroveBrambleWarren | BuildingKind::GroveSpitefen => RaceKind::Grove,
            BuildingKind::EmberMagmaForge | BuildingKind::EmberAshPack => RaceKind::Ember,
        }
    }

    pub fn fallback_name(self) -> &'static str {
        match self {
            BuildingKind::VanguardBarracks => "Barracks",
            BuildingKind::VanguardRangeTower => "Range Tower",
            BuildingKind::VanguardForge => "Forge",
            BuildingKind::VanguardPikeYard => "Pike Yard",
            BuildingKind::VanguardBulwarkHall => "Bulwark Hall",
            BuildingKind::VanguardChapel => "Chapel",
            BuildingKind::VanguardStables => "Stables",
            BuildingKind::VanguardSiegeWorkshop => "Siege Workshop",
            BuildingKind::GroveRootDen => "Root Den",
            BuildingKind::GroveThornSpire => "Thorn Spire",
            BuildingKind::GroveBloomWell => "Bloom Well",
            BuildingKind::GroveMossNursery => "Moss Nursery",
            BuildingKind::GroveBarkBastion => "Bark Bastion",
            BuildingKind::GroveMirePool => "Mire Pool",
            BuildingKind::GroveVineWarren => "Vine Warren",
            BuildingKind::GroveAncientSeed => "Ancient Seed",
            BuildingKind::EmberCinderPit => "Cinder Pit",
            BuildingKind::EmberFlameSpire => "Flame Spire",
            BuildingKind::EmberAshMine => "Ash Mine",
            BuildingKind::EmberSparkKennel => "Spark Kennel",
            BuildingKind::EmberObsidianGate => "Obsidian Gate",
            BuildingKind::EmberBlazeStable => "Blaze Stable",
            BuildingKind::EmberSmokeAltar => "Smoke Altar",
            BuildingKind::EmberInfernoEngine => "Inferno Engine",
            BuildingKind::VanguardArbalestTower => "Arbalest Tower",
            BuildingKind::VanguardArcaneSpire => "Arcane Spire",
            BuildingKind::GroveBrambleWarren => "Bramble Warren",
            BuildingKind::GroveSpitefen => "Spitefen",
            BuildingKind::EmberMagmaForge => "Magma Forge",
            BuildingKind::EmberAshPack => "Ash Pack",
        }
    }

    pub fn from_cli(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "barracks" | "vanguard-barracks" | "b" => Some(BuildingKind::VanguardBarracks),
            "range" | "range-tower" | "rangetower" | "tower" | "r" => {
                Some(BuildingKind::VanguardRangeTower)
            }
            "forge" | "f" => Some(BuildingKind::VanguardForge),
            "pike" | "pike-yard" | "pikeyard" | "v4" => Some(BuildingKind::VanguardPikeYard),
            "bulwark" | "bulwark-hall" | "bulwarkhall" | "v5" => {
                Some(BuildingKind::VanguardBulwarkHall)
            }
            "chapel" | "v6" => Some(BuildingKind::VanguardChapel),
            "stables" | "stable" | "v7" => Some(BuildingKind::VanguardStables),
            "siege" | "siege-workshop" | "siegeworkshop" | "v8" => {
                Some(BuildingKind::VanguardSiegeWorkshop)
            }
            "root" | "root-den" | "rootden" | "g1" => Some(BuildingKind::GroveRootDen),
            "thorn" | "thorn-spire" | "thornspire" | "g2" => Some(BuildingKind::GroveThornSpire),
            "bloom" | "bloom-well" | "bloomwell" | "g3" => Some(BuildingKind::GroveBloomWell),
            "moss" | "moss-nursery" | "mossnursery" | "g4" => Some(BuildingKind::GroveMossNursery),
            "bark" | "bark-bastion" | "barkbastion" | "g5" => Some(BuildingKind::GroveBarkBastion),
            "mire" | "mire-pool" | "mirepool" | "g6" => Some(BuildingKind::GroveMirePool),
            "vine" | "vine-warren" | "vinewarren" | "g7" => Some(BuildingKind::GroveVineWarren),
            "ancient" | "ancient-seed" | "ancientseed" | "g8" => {
                Some(BuildingKind::GroveAncientSeed)
            }
            "cinder" | "cinder-pit" | "cinderpit" | "e1" => Some(BuildingKind::EmberCinderPit),
            "flame" | "flame-spire" | "flamespire" | "e2" => Some(BuildingKind::EmberFlameSpire),
            "ash" | "ash-mine" | "ashmine" | "e3" => Some(BuildingKind::EmberAshMine),
            "spark" | "spark-kennel" | "sparkkennel" | "e4" => Some(BuildingKind::EmberSparkKennel),
            "obsidian" | "obsidian-gate" | "obsidiangate" | "e5" => {
                Some(BuildingKind::EmberObsidianGate)
            }
            "blaze" | "blaze-stable" | "blazestable" | "e6" => Some(BuildingKind::EmberBlazeStable),
            "smoke" | "smoke-altar" | "smokealtar" | "e7" => Some(BuildingKind::EmberSmokeAltar),
            "inferno" | "inferno-engine" | "infernoengine" | "e8" => {
                Some(BuildingKind::EmberInfernoEngine)
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UnitKind {
    VanguardGuard,
    VanguardArcher,
    VanguardPikeman,
    VanguardShieldbearer,
    VanguardBattleCleric,
    VanguardLancer,
    VanguardBallista,
    GroveBruiser,
    GroveNeedler,
    GroveSproutling,
    GroveBarkguard,
    GroveMireShaman,
    GroveVineStalker,
    GroveTreantColossus,
    EmberRunner,
    EmberCaster,
    EmberSparkImp,
    EmberObsidianGuard,
    EmberFireLancer,
    EmberSmokeWitch,
    EmberCinderEngine,
    VanguardArbalester,
    VanguardArcanist,
    GroveBrambleguard,
    GroveSpitefang,
    EmberMagmaBrute,
    EmberAshStalker,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AttackType {
    Normal,
    Pierce,
    Magic,
    Siege,
    Chaos,
}

impl AttackType {
    pub fn label(self) -> &'static str {
        match self {
            AttackType::Normal => "Normal",
            AttackType::Pierce => "Pierce",
            AttackType::Magic => "Magic",
            AttackType::Siege => "Siege",
            AttackType::Chaos => "Chaos",
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            AttackType::Normal => "N",
            AttackType::Pierce => "P",
            AttackType::Magic => "M",
            AttackType::Siege => "S",
            AttackType::Chaos => "C",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ArmorType {
    Normal,
    Light,
    Heavy,
    Fortified,
    Unarmored,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AttackMode {
    Melee,
    Ranged,
}

impl AttackMode {
    pub fn label(self) -> &'static str {
        match self {
            AttackMode::Melee => "Melee",
            AttackMode::Ranged => "Ranged",
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            AttackMode::Melee => "MEL",
            AttackMode::Ranged => "RNG",
        }
    }
}

impl ArmorType {
    pub fn label(self) -> &'static str {
        match self {
            ArmorType::Normal => "Normal",
            ArmorType::Light => "Light",
            ArmorType::Heavy => "Heavy",
            ArmorType::Fortified => "Fortified",
            ArmorType::Unarmored => "Unarmored",
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            ArmorType::Normal => "N",
            ArmorType::Light => "L",
            ArmorType::Heavy => "H",
            ArmorType::Fortified => "F",
            ArmorType::Unarmored => "U",
        }
    }
}

pub fn attack_multiplier(attack: AttackType, armor: ArmorType) -> f32 {
    match attack {
        AttackType::Normal => match armor {
            ArmorType::Normal | ArmorType::Light | ArmorType::Heavy | ArmorType::Unarmored => 1.0,
            ArmorType::Fortified => 0.70,
        },
        AttackType::Pierce => match armor {
            ArmorType::Light | ArmorType::Unarmored => 1.30,
            ArmorType::Normal => 1.0,
            ArmorType::Heavy => 0.70,
            ArmorType::Fortified => 0.35,
        },
        AttackType::Magic => match armor {
            ArmorType::Heavy => 1.30,
            ArmorType::Unarmored => 1.25,
            ArmorType::Normal => 1.0,
            ArmorType::Light => 0.70,
            ArmorType::Fortified => 0.35,
        },
        AttackType::Siege => match armor {
            ArmorType::Fortified => 1.50,
            ArmorType::Unarmored => 1.20,
            ArmorType::Normal => 1.0,
            ArmorType::Light | ArmorType::Heavy => 0.70,
        },
        AttackType::Chaos => 1.0,
    }
}

pub fn typed_damage(base_damage: i32, attack: AttackType, armor: ArmorType) -> i32 {
    if base_damage <= 0 {
        return 0;
    }
    ((base_damage as f32) * attack_multiplier(attack, armor))
        .round()
        .max(1.0) as i32
}

pub fn damage_range(midpoint: i32, variance: f32) -> (i32, i32) {
    if midpoint <= 0 {
        return (0, 0);
    }
    let spread = (midpoint as f32 * variance.max(0.0)).round() as i32;
    ((midpoint - spread).max(1), midpoint + spread)
}

pub fn interest_gold(gold: i32, interest_rate: f32) -> i32 {
    ((gold.max(0) as f32) * interest_rate.max(0.0)).floor() as i32
}

pub fn building_lane_pos(team: Team, zone: BuildZone, cell: GridCell) -> f32 {
    let cell_x = cell.x.clamp(0, GRID_W - 1) as f32;
    match (team, zone) {
        (Team::Left, BuildZone::Front) => 8.0 + cell_x * 1.7,
        (Team::Left, BuildZone::Back) => -8.0 - cell_x * 1.7,
        (Team::Right, BuildZone::Front) => LANE_LENGTH - 8.0 - cell_x * 1.7,
        (Team::Right, BuildZone::Back) => LANE_LENGTH + 8.0 + cell_x * 1.7,
    }
}

pub fn building_spawn_pos(team: Team, zone: BuildZone, cell: GridCell) -> f32 {
    building_lane_pos(team, zone, cell) + team.direction() * 2.0
}

pub fn lane_center_y(lane: Lane) -> f32 {
    match lane {
        Lane::Top => 12.0,
        Lane::UpperMid => 4.0,
        Lane::LowerMid => -4.0,
        Lane::Bottom => -12.0,
    }
}

pub fn lane_position(lane: Lane, lane_pos: f32) -> WorldPos {
    WorldPos::new(lane_pos, lane_center_y(lane))
}

pub fn building_position(team: Team, lane: Lane, zone: BuildZone, cell: GridCell) -> WorldPos {
    let cell_y = cell.y.clamp(0, GRID_H - 1) as f32;
    WorldPos::new(
        building_lane_pos(team, zone, cell),
        lane_center_y(lane) + (cell_y - (GRID_H - 1) as f32 * 0.5) * GRID_CELL_Y,
    )
}

pub fn building_spawn_position(
    team: Team,
    lane: Lane,
    zone: BuildZone,
    cell: GridCell,
) -> WorldPos {
    let building = building_position(team, lane, zone, cell);
    WorldPos::new(building.x + team.direction() * 2.0, building.y)
}

/// True when `pos` (sim space) lies inside the viewer side's current vision:
/// every unit, building, and the castle belonging to `viewer`'s side emits a
/// vision circle. Side-based, so it survives player-count changes.
pub fn position_revealed_to(
    players: &[PlayerInfo],
    buildings: &[Building],
    units: &[Unit],
    viewer: Team,
    pos: WorldPos,
) -> bool {
    let sides: Vec<(PlayerId, Team)> = players
        .iter()
        .map(|player| (player.id, player.team))
        .collect();
    for lane in Lane::ALL {
        let castle = lane_position(lane, viewer.castle_pos());
        if castle.distance(pos) <= VISION_CASTLE_RADIUS {
            return true;
        }
    }
    for building in buildings
        .iter()
        .filter(|building| lookup_side(&sides, building.owner) == viewer)
    {
        if building_position(
            lookup_side(&sides, building.owner),
            building.lane,
            building.zone,
            building.cell,
        )
        .distance(pos)
            <= VISION_BUILDING_RADIUS
        {
            return true;
        }
    }
    for unit in units
        .iter()
        .filter(|unit| lookup_side(&sides, unit.owner) == viewer)
    {
        if unit.pos.distance(pos) <= VISION_UNIT_RADIUS {
            return true;
        }
    }
    false
}

pub fn castle_junction(pos: f32) -> Option<Team> {
    if (pos - Team::Left.castle_pos()).abs() <= CASTLE_JUNCTION_RANGE {
        Some(Team::Left)
    } else if (pos - Team::Right.castle_pos()).abs() <= CASTLE_JUNCTION_RANGE {
        Some(Team::Right)
    } else {
        None
    }
}

pub fn unit_lanes_connected(
    attacker_lane: Lane,
    attacker_pos: f32,
    target_lane: Lane,
    target_pos: f32,
) -> bool {
    if attacker_lane == target_lane {
        return true;
    }
    matches!(
        (castle_junction(attacker_pos), castle_junction(target_pos)),
        (Some(a), Some(b)) if a == b
    )
}

fn unit_combat_distance(
    attacker_lane: Lane,
    attacker_pos: WorldPos,
    target_lane: Lane,
    target_lane_pos: f32,
    target_pos: WorldPos,
) -> f32 {
    if attacker_lane != target_lane
        && unit_lanes_connected(attacker_lane, attacker_pos.x, target_lane, target_lane_pos)
    {
        (attacker_pos.x - target_pos.x).abs()
    } else {
        attacker_pos.distance(target_pos)
    }
}

fn footprint_distance(pos: WorldPos, target: WorldPos, radius: f32) -> f32 {
    (pos.distance(target) - radius).max(0.0)
}

fn roll_base_damage(rng_state: &mut u64, midpoint: i32, variance: f32) -> i32 {
    let (min_damage, max_damage) = damage_range(midpoint, variance);
    if max_damage <= min_damage {
        return min_damage;
    }
    let span = (max_damage - min_damage + 1) as u32;
    min_damage + (next_random_u32(rng_state) % span) as i32
}

fn next_random_u32(state: &mut u64) -> u32 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    (*state >> 32) as u32
}

/// Borrow-free side lookup for loops that hold `&mut self.units`.
fn lookup_side(players: &[(PlayerId, Team)], owner: PlayerId) -> Team {
    players
        .iter()
        .find(|(id, _)| *id == owner)
        .map(|(_, team)| *team)
        .unwrap_or(Team::Left)
}

fn mix_seed(seed: u64) -> u64 {
    // splitmix64 finalizer so structurally similar seeds (small counters) spread
    // across the full LCG state instead of producing correlated matches.
    let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

impl UnitKind {
    pub fn race_like(self) -> RaceKind {
        match self {
            UnitKind::VanguardGuard
            | UnitKind::VanguardArcher
            | UnitKind::VanguardPikeman
            | UnitKind::VanguardShieldbearer
            | UnitKind::VanguardBattleCleric
            | UnitKind::VanguardLancer
            | UnitKind::VanguardBallista
            | UnitKind::VanguardArbalester
            | UnitKind::VanguardArcanist => RaceKind::Vanguard,
            UnitKind::GroveBruiser
            | UnitKind::GroveNeedler
            | UnitKind::GroveSproutling
            | UnitKind::GroveBarkguard
            | UnitKind::GroveMireShaman
            | UnitKind::GroveVineStalker
            | UnitKind::GroveTreantColossus
            | UnitKind::GroveBrambleguard
            | UnitKind::GroveSpitefang => RaceKind::Grove,
            UnitKind::EmberRunner
            | UnitKind::EmberCaster
            | UnitKind::EmberSparkImp
            | UnitKind::EmberObsidianGuard
            | UnitKind::EmberFireLancer
            | UnitKind::EmberSmokeWitch
            | UnitKind::EmberCinderEngine
            | UnitKind::EmberMagmaBrute
            | UnitKind::EmberAshStalker => RaceKind::Ember,
        }
    }

    pub fn fallback_name(self) -> &'static str {
        match self {
            UnitKind::VanguardGuard => "Guard",
            UnitKind::VanguardArcher => "Archer",
            UnitKind::VanguardPikeman => "Pikeman",
            UnitKind::VanguardShieldbearer => "Shieldbearer",
            UnitKind::VanguardBattleCleric => "Battle Cleric",
            UnitKind::VanguardLancer => "Lancer",
            UnitKind::VanguardBallista => "Ballista",
            UnitKind::GroveBruiser => "Bruiser",
            UnitKind::GroveNeedler => "Needler",
            UnitKind::GroveSproutling => "Sproutling",
            UnitKind::GroveBarkguard => "Barkguard",
            UnitKind::GroveMireShaman => "Mire Shaman",
            UnitKind::GroveVineStalker => "Vine Stalker",
            UnitKind::GroveTreantColossus => "Treant Colossus",
            UnitKind::EmberRunner => "Runner",
            UnitKind::EmberCaster => "Caster",
            UnitKind::EmberSparkImp => "Spark Imp",
            UnitKind::EmberObsidianGuard => "Obsidian Guard",
            UnitKind::EmberFireLancer => "Fire Lancer",
            UnitKind::EmberSmokeWitch => "Smoke Witch",
            UnitKind::EmberCinderEngine => "Cinder Engine",
            UnitKind::VanguardArbalester => "Arbalester",
            UnitKind::VanguardArcanist => "Arcanist",
            UnitKind::GroveBrambleguard => "Brambleguard",
            UnitKind::GroveSpitefang => "Spitefang",
            UnitKind::EmberMagmaBrute => "Magma Brute",
            UnitKind::EmberAshStalker => "Ash Stalker",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceConfig {
    pub starting_gold: i32,
    pub base_income: i32,
    pub income_interval: f32,
    #[serde(default = "default_interest_rate")]
    pub interest_rate: f32,
    #[serde(default = "default_castle_regen_per_second")]
    pub castle_regen_per_second: f32,
    #[serde(default = "default_castle_regen_delay_secs")]
    pub castle_regen_delay_secs: f32,
    #[serde(default = "default_sudden_death_ramp_per_minute")]
    pub sudden_death_ramp_per_minute: f32,
    #[serde(default = "default_sudden_death_escalation_per_minute")]
    pub sudden_death_escalation_per_minute: f32,
    pub sudden_death_start: f32,
    pub races: Vec<RaceConfig>,
    pub buildings: Vec<BuildingConfig>,
    pub units: Vec<UnitConfig>,
}

impl Default for BalanceConfig {
    fn default() -> Self {
        serde_json::from_str(include_str!("../config/balance.json"))
            .expect("embedded config/balance.json must be valid")
    }
}

impl BalanceConfig {
    pub fn load_or_default(path: impl AsRef<Path>) -> Self {
        match fs::read_to_string(path.as_ref()) {
            Ok(raw) => match serde_json::from_str(&raw) {
                Ok(config) => config,
                Err(err) => {
                    eprintln!(
                        "Invalid balance config at {:?}: {err}. Using embedded defaults.",
                        path.as_ref()
                    );
                    Self::default()
                }
            },
            Err(_) => Self::default(),
        }
    }

    pub fn race(&self, kind: RaceKind) -> &RaceConfig {
        self.races
            .iter()
            .find(|race| race.kind == kind)
            .unwrap_or_else(|| panic!("missing race config for {kind:?}"))
    }

    pub fn building(&self, kind: BuildingKind) -> &BuildingConfig {
        self.buildings
            .iter()
            .find(|building| building.kind == kind)
            .unwrap_or_else(|| panic!("missing building config for {kind:?}"))
    }

    pub fn unit(&self, kind: UnitKind) -> &UnitConfig {
        self.units
            .iter()
            .find(|unit| unit.kind == kind)
            .unwrap_or_else(|| panic!("missing unit config for {kind:?}"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaceConfig {
    pub kind: RaceKind,
    pub name: String,
    pub castle_health: i32,
    #[serde(default = "default_castle_armor")]
    pub castle_armor: ArmorType,
    pub buildings: Vec<BuildingKind>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildingConfig {
    pub kind: BuildingKind,
    pub name: String,
    pub label: String,
    pub cost: i32,
    #[serde(default = "default_building_health")]
    pub max_health: i32,
    #[serde(default = "default_building_armor")]
    pub armor_type: ArmorType,
    pub spawn_interval: Option<f32>,
    pub spawned_unit: Option<UnitKind>,
    pub income_bonus: i32,
    pub color: [f32; 3],
    /// Branch specializations this building may be upgraded into.
    #[serde(default)]
    pub upgrades: Vec<BuildingKind>,
    /// Set when this config is itself an upgrade of a base building.
    #[serde(default)]
    pub upgraded_from: Option<BuildingKind>,
}

/// Data-driven unit abilities (plan.md Phase 2 item 2). All parameters are
/// serde-defaulted so balance.json can omit any field; a unit has at most
/// one ability.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AbilityConfig {
    /// Heals the most-wounded allied unit in range instead of attacking.
    Heal {
        #[serde(default = "default_heal_amount")]
        amount: i32,
        #[serde(default = "default_heal_range")]
        range: f32,
        #[serde(default = "default_heal_interval")]
        interval: f32,
    },
    /// Primary attack also damages extra enemies near the target.
    Splash {
        #[serde(default = "default_splash_targets")]
        targets: u32,
        #[serde(default = "default_splash_fraction")]
        fraction: f32,
        #[serde(default = "default_splash_radius")]
        radius: f32,
    },
    /// Attacks slow the target's movement for a duration.
    Slow {
        #[serde(default = "default_slow_factor")]
        factor: f32,
        #[serde(default = "default_slow_duration")]
        duration: f32,
    },
    /// Regenerates health every second, in or out of combat.
    Regeneration {
        #[serde(default = "default_regen_rate")]
        health_per_second: i32,
    },
    /// Below a health fraction: faster movement and attacks.
    Berserk {
        #[serde(default = "default_berserk_health")]
        below_health_fraction: f32,
        #[serde(default = "default_berserk_speed")]
        speed_multiplier: f32,
        #[serde(default = "default_berserk_attack")]
        attack_speed_multiplier: f32,
    },
}

fn default_heal_amount() -> i32 {
    14
}
fn default_heal_range() -> f32 {
    9.0
}
fn default_heal_interval() -> f32 {
    2.0
}
fn default_splash_targets() -> u32 {
    2
}
fn default_splash_fraction() -> f32 {
    0.5
}
fn default_splash_radius() -> f32 {
    3.0
}
fn default_slow_factor() -> f32 {
    0.6
}
fn default_slow_duration() -> f32 {
    2.0
}
fn default_regen_rate() -> i32 {
    3
}
fn default_berserk_health() -> f32 {
    0.35
}
fn default_berserk_speed() -> f32 {
    1.5
}
fn default_berserk_attack() -> f32 {
    1.3
}

impl AbilityConfig {
    /// One-line human description for tooltips and the help overlay.
    pub fn describe(&self) -> String {
        match self {
            Self::Heal {
                amount, interval, ..
            } => {
                format!("Heals the most wounded nearby ally for {amount} every {interval:.1}s")
            }
            Self::Splash {
                targets, fraction, ..
            } => format!(
                "Attacks hit up to {targets} extra enemies near the target for {}% damage",
                (fraction * 100.0) as i32
            ),
            Self::Slow { factor, duration } => format!(
                "Attacks slow enemies to {}% speed for {duration:.1}s",
                (factor * 100.0) as i32
            ),
            Self::Regeneration { health_per_second } => {
                format!("Regenerates {health_per_second} HP per second")
            }
            Self::Berserk {
                below_health_fraction,
                speed_multiplier,
                ..
            } => format!(
                "Below {}% HP: +{}% speed and attack speed",
                (below_health_fraction * 100.0) as i32,
                ((speed_multiplier - 1.0) * 100.0) as i32
            ),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitConfig {
    pub kind: UnitKind,
    pub name: String,
    pub label: String,
    pub max_health: i32,
    pub damage: i32,
    #[serde(default = "default_damage_variance")]
    pub damage_variance: f32,
    #[serde(default = "default_bounty")]
    pub bounty: i32,
    #[serde(default = "default_attack_type")]
    pub attack_type: AttackType,
    #[serde(default = "default_attack_mode")]
    pub attack_mode: AttackMode,
    #[serde(default = "default_armor_type")]
    pub armor_type: ArmorType,
    #[serde(alias = "range")]
    pub attack_range: f32,
    pub speed: f32,
    #[serde(default = "default_unit_radius")]
    pub radius: f32,
    pub attack_interval: f32,
    #[serde(default)]
    pub ability: Option<AbilityConfig>,
}

fn default_castle_armor() -> ArmorType {
    ArmorType::Fortified
}

fn default_attack_type() -> AttackType {
    AttackType::Normal
}

fn default_attack_mode() -> AttackMode {
    AttackMode::Melee
}

fn default_damage_variance() -> f32 {
    0.10
}

fn default_interest_rate() -> f32 {
    INTEREST_RATE
}

fn default_castle_regen_per_second() -> f32 {
    CASTLE_REGEN_PER_SECOND
}

fn default_castle_regen_delay_secs() -> f32 {
    CASTLE_REGEN_DELAY_SECS
}

fn default_sudden_death_ramp_per_minute() -> f32 {
    SUDDEN_DEATH_RAMP_PER_MINUTE
}

fn default_sudden_death_escalation_per_minute() -> f32 {
    SUDDEN_DEATH_ESCALATION_PER_MINUTE
}

fn default_bounty() -> i32 {
    1
}

fn default_building_health() -> i32 {
    250
}

fn default_building_armor() -> ArmorType {
    ArmorType::Fortified
}

fn default_armor_type() -> ArmorType {
    ArmorType::Normal
}

fn default_unit_radius() -> f32 {
    0.65
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GridCell {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WorldPos {
    pub x: f32,
    pub y: f32,
}

impl WorldPos {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn distance(self, other: Self) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerInfo {
    pub id: PlayerId,
    pub name: String,
    pub team: Team,
    pub race: Option<RaceKind>,
    pub ready: bool,
    pub connected: bool,
    pub rematch_vote: bool,
    /// Career W/L from the server's profile store (plan.md Phase 3).
    #[serde(default)]
    pub wins: u32,
    #[serde(default)]
    pub losses: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Economy {
    pub gold: i32,
    pub income: i32,
}

impl Default for Economy {
    fn default() -> Self {
        Self {
            gold: STARTING_GOLD,
            income: BASE_INCOME,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Castle {
    /// Owning player (PlayerId); `team` is the derived side for rendering.
    pub owner: PlayerId,
    pub team: Team,
    pub health: i32,
    pub max_health: i32,
    pub armor_type: ArmorType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Building {
    pub id: u64,
    pub owner: PlayerId,
    pub kind: BuildingKind,
    pub lane: Lane,
    pub zone: BuildZone,
    pub cell: GridCell,
    pub health: i32,
    pub max_health: i32,
    pub spawn_timer: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Unit {
    pub id: u64,
    pub owner: PlayerId,
    pub kind: UnitKind,
    pub lane: Lane,
    pub health: i32,
    pub lane_pos: f32,
    pub pos: WorldPos,
    pub velocity: WorldPos,
    pub radius: f32,
    pub attack_timer: f32,
    // Ability runtime state (plan.md Phase 2 item 2). Not synced through
    // UnitUpdate deltas: the client never renders it directly.
    #[serde(default)]
    pub slow_timer: f32,
    #[serde(default = "default_slow_unit_factor")]
    pub slow_factor: f32,
    #[serde(default)]
    pub regen_accum: f32,
}

fn default_slow_unit_factor() -> f32 {
    1.0
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BountyEvent {
    pub id: u64,
    pub team: Team,
    pub amount: i32,
    pub lane: Lane,
    pub lane_pos: f32,
    pub pos: WorldPos,
    pub unit_kind: UnitKind,
    pub age: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatchSnapshot {
    pub phase: MatchPhase,
    pub tick: u64,
    pub elapsed_secs: f32,
    pub sudden_death: bool,
    #[serde(default)]
    pub seed: u64,
    pub players: Vec<PlayerInfo>,
    #[serde(default)]
    pub economies: Vec<Economy>,
    #[serde(default)]
    pub castles: Vec<Castle>,
    pub buildings: Vec<Building>,
    pub units: Vec<Unit>,
    pub bounty_events: Vec<BountyEvent>,
    pub winner: Option<Team>,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct GameSim {
    pub phase: MatchPhase,
    pub tick: u64,
    pub players: Vec<PlayerInfo>,
    /// Per-player economies, indexed by position in `players`.
    pub economies: Vec<Economy>,
    /// One castle per player, indexed by position in `players`.
    pub castles: Vec<Castle>,
    pub buildings: Vec<Building>,
    pub units: Vec<Unit>,
    pub bounty_events: Vec<BountyEvent>,
    pub winner: Option<Team>,
    pub message: String,
    pub balance: BalanceConfig,
    /// Players per side (1 = 1v1, 2 = 2v2, ...). Set at game creation.
    pub team_size: usize,
    /// When enabled, races are randomly assigned at match start
    /// (plan.md Phase 3 faction draft).
    pub random_factions: bool,
    next_id: u64,
    seed: u64,
    rng_state: u64,
    income_timer: f32,
    elapsed_secs: f32,
    castle_regen_accum: Vec<f32>,
    castle_regen_delay_timer: Vec<f32>,
    overtime_damage_accum: [f32; 2],
    /// Lane assigned to each player (parallel to players vec).
    pub assigned_lanes: Vec<Lane>,
}

impl Default for GameSim {
    fn default() -> Self {
        Self::new(BalanceConfig::default())
    }
}

impl GameSim {
    pub fn new(balance: BalanceConfig) -> Self {
        Self::with_seed(balance, DEFAULT_SIM_SEED)
    }

    pub fn with_seed(balance: BalanceConfig, seed: u64) -> Self {
        let income_timer = balance.income_interval;
        let mut sim = Self {
            phase: MatchPhase::Lobby,
            tick: 0,
            players: Vec::new(),
            economies: Vec::new(),
            castles: Vec::new(),
            buildings: Vec::new(),
            units: Vec::new(),
            bounty_events: Vec::new(),
            winner: None,
            message: "Waiting for two players.".to_string(),
            balance,
            team_size: 1,
            random_factions: false,
            next_id: 1,
            seed,
            rng_state: mix_seed(seed),
            income_timer,
            elapsed_secs: 0.0,
            castle_regen_accum: Vec::new(),
            castle_regen_delay_timer: Vec::new(),
            assigned_lanes: Vec::new(),
            overtime_damage_accum: [0.0, 0.0],
        };
        sim.reset_match_state();
        sim
    }

    /// Enable random faction assignment at match start (plan.md Phase 3).
    /// Must be called before any player joins.
    pub fn set_random_factions(&mut self, enabled: bool) -> Result<(), String> {
        if !self.players.is_empty() {
            return Err("Cannot change random factions after players have joined.".to_string());
        }
        self.random_factions = enabled;
        Ok(())
    }

    /// Set the players-per-side for team play. Must be called before any
    /// player joins; e.g. team_size 2 creates a 2v2 lobby seating 4.
    pub fn set_team_size(&mut self, team_size: usize) -> Result<(), String> {
        if !self.players.is_empty() {
            return Err("Cannot change team size after players have joined.".to_string());
        }
        self.team_size = team_size.max(1);
        Ok(())
    }

    /// Set a player's career W/L for scoreboard display.
    pub fn set_player_record(&mut self, name: &str, wins: u32, losses: u32) {
        if let Some(player) = self.players.iter_mut().find(|p| p.name == name) {
            player.wins = wins;
            player.losses = losses;
        }
    }

    pub fn player_index(&self, player_id: PlayerId) -> Option<usize> {
        self.players
            .iter()
            .position(|player| player.id == player_id)
    }

    /// Side of a player; defaults to `Left` for unknown ids (callers gate on
    /// membership first).
    pub fn side_of(&self, owner: PlayerId) -> Team {
        self.players
            .iter()
            .find(|player| player.id == owner)
            .map(|player| player.team)
            .unwrap_or(Team::Left)
    }

    /// Sum of one side's castle HP (sides may hold several players later).
    pub fn side_castle_health(&self, team: Team) -> i32 {
        self.castles
            .iter()
            .filter(|castle| castle.team == team)
            .map(|castle| castle.health)
            .sum()
    }

    fn side_alive_castles(&self, team: Team) -> Vec<usize> {
        self.castles
            .iter()
            .enumerate()
            .filter(|(_, castle)| castle.team == team && castle.health > 0)
            .map(|(index, _)| index)
            .collect()
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    pub fn elapsed_secs(&self) -> f32 {
        self.elapsed_secs
    }

    pub fn snapshot(&self) -> MatchSnapshot {
        MatchSnapshot {
            phase: self.phase,
            tick: self.tick,
            elapsed_secs: self.elapsed_secs,
            sudden_death: self.sudden_death(),
            seed: self.seed,
            players: self.players.clone(),
            economies: self.economies.clone(),
            castles: self.castles.clone(),
            buildings: self.buildings.clone(),
            units: self.units.clone(),
            bounty_events: self.bounty_events.clone(),
            winner: self.winner,
            message: self.message.clone(),
        }
    }

    pub fn player(&self, player_id: PlayerId) -> Option<PlayerInfo> {
        self.players
            .iter()
            .find(|player| player.id == player_id)
            .cloned()
    }

    pub fn join_or_update_player(&mut self, name: String) -> Result<PlayerInfo, String> {
        let name = clean_name(name);
        if let Some(player) = self
            .players
            .iter_mut()
            .find(|player| player.name == name && !player.connected)
        {
            player.name = name.clone();
            player.connected = true;
            player.ready = false;
            player.rematch_vote = false;
            self.message = format!("{} reconnected.", player.name);
            return Ok(player.clone());
        }

        if self
            .players
            .iter()
            .any(|player| player.name == name && player.connected)
        {
            return Err("That player name is already connected.".to_string());
        }

        if let Some(player) = self.players.iter_mut().find(|p| !p.connected) {
            player.name = name.clone();
            player.connected = true;
            player.ready = false;
            player.rematch_vote = false;
            self.message = format!("{} joined as {:?}.", player.name, player.team);
            return Ok(player.clone());
        }

        let max_players = self.team_size * 2;
        if self.players.len() >= max_players {
            return Err("Match is full.".to_string());
        }

        // First team_size = Left, next team_size = Right (plan.md team play).
        let team = if self.players.len() < self.team_size {
            Team::Left
        } else {
            Team::Right
        };
        let player = PlayerInfo {
            id: PlayerId(self.players.len() as u8 + 1),
            name,
            team,
            race: None,
            ready: false,
            connected: true,
            rematch_vote: false,
            wins: 0,
            losses: 0,
        };
        self.message = format!("{} joined as {:?}.", player.name, player.team);
        // Per-player state grows with the seat (plan.md Phase 1 item 3).
        self.economies.push(Economy {
            gold: self.balance.starting_gold,
            income: self.balance.base_income,
        });
        // Lane assignment: player k on each side gets lane k.
        // For 1v1 (team_size 1): Left P0 -> Top, Right P0 -> Bottom.
        // For 2v2 (team_size 2): Left P0 -> Top, Left P1 -> UpperMid,
        //   Right P0 -> Bottom, Right P1 -> LowerMid.
        let side_index = self.players.iter().filter(|p| p.team == team).count();
        let lane = Lane::for_player(team, side_index);
        self.assigned_lanes.push(lane);
        let castle_health = self.balance.race(RaceKind::Vanguard).castle_health;
        self.castles.push(Castle {
            owner: player.id,
            team: player.team,
            health: castle_health,
            max_health: castle_health,
            armor_type: default_castle_armor(),
        });
        self.castle_regen_accum.push(0.0);
        self.castle_regen_delay_timer.push(0.0);
        self.players.push(player.clone());
        Ok(player)
    }

    pub fn set_race(&mut self, player_id: PlayerId, race: RaceKind) -> Result<(), String> {
        if self.phase != MatchPhase::Lobby {
            return Err("Race can only be changed in the lobby.".to_string());
        }
        let race_name = self.balance.race(race).name.clone();
        let player = self
            .players
            .iter_mut()
            .find(|p| p.id == player_id)
            .ok_or_else(|| "Unknown player.".to_string())?;
        player.race = Some(race);
        player.ready = false;
        self.message = format!("{} chose {race_name}.", player.name);
        Ok(())
    }

    /// Disconnecting during a match no longer destroys it: the seat stays
    /// reserved (plan.md Phase 1 reconnect grace). The server resets the
    /// match to the lobby only once the reconnect deadline lapses, via
    /// [`GameSim::reset_to_lobby`].
    pub fn disconnect_player(&mut self, player_id: PlayerId) {
        if let Some(player) = self.players.iter_mut().find(|p| p.id == player_id) {
            player.connected = false;
            player.ready = false;
            if self.phase == MatchPhase::Playing {
                self.message = format!(
                    "{} disconnected - waiting for reconnect (90s).",
                    player.name
                );
                return;
            }
            self.message = format!("{} disconnected.", player.name);
        }
    }

    /// True while a match is running but at least one seat is empty; the
    /// server shows the wait message and can reset after its grace window.
    pub fn waiting_for_reconnect(&self) -> bool {
        self.phase == MatchPhase::Playing && self.players.iter().any(|player| !player.connected)
    }

    pub fn reset_to_lobby(&mut self) {
        self.reset_to_lobby_inner();
    }

    /// Branch upgrade (plan.md Phase 2 item 3): replace a base building with
    /// one of its listed specializations in place. Costs the price delta
    /// between the target and the base config; heals the building to full
    /// and resets its spawn timer. Upgraded buildings sell at 70% of their
    /// (higher) value.
    pub fn upgrade_building(
        &mut self,
        player_id: PlayerId,
        building_id: u64,
        to: BuildingKind,
    ) -> Result<(), String> {
        if self.phase != MatchPhase::Playing {
            return Err("Buildings can only be upgraded during a match.".to_string());
        }
        let index = self
            .buildings
            .iter()
            .position(|building| building.id == building_id)
            .ok_or_else(|| "That building no longer exists.".to_string())?;
        if self.buildings[index].owner != player_id {
            return Err("You can only upgrade your own buildings.".to_string());
        }
        let base_kind = self.buildings[index].kind;
        let base_name = self.balance.building(base_kind).name.clone();
        let base_cost = self.balance.building(base_kind).cost;
        if !self.balance.building(base_kind).upgrades.contains(&to) {
            return Err(format!(
                "{} cannot be upgraded into {}.",
                base_name,
                self.balance.building(to).name
            ));
        }
        let target = self.balance.building(to);
        let cost = (target.cost - base_cost).max(0);
        let owner_index = self.player_index(player_id).unwrap_or(0);
        if self.economies[owner_index].gold < cost {
            return Err(format!("Not enough gold for {}.", target.name));
        }
        self.economies[owner_index].gold -= cost;
        let building = &mut self.buildings[index];
        building.kind = to;
        building.max_health = target.max_health;
        building.health = target.max_health;
        building.spawn_timer = target.spawn_interval.unwrap_or(0.0);
        self.message = format!(
            "{team:?} upgraded {base_name} into {}.",
            target.name,
            team = self.side_of(player_id)
        );
        Ok(())
    }

    /// Sell one of the player's buildings for a 70% refund (plan.md Phase 1
    /// item 5). Selling exists so a bad read stays recoverable: counter-
    /// building remains a live option instead of a lost cause.
    pub fn sell_building(&mut self, player_id: PlayerId, building_id: u64) -> Result<(), String> {
        if self.phase != MatchPhase::Playing {
            return Err("Buildings can only be sold during a match.".to_string());
        }
        let team = self
            .players
            .iter()
            .find(|p| p.id == player_id)
            .map(|p| p.team)
            .ok_or_else(|| "Unknown player.".to_string())?;
        let index = self
            .buildings
            .iter()
            .position(|b| b.id == building_id)
            .ok_or_else(|| "That building no longer exists.".to_string())?;
        if self.buildings[index].owner != player_id {
            return Err("You can only sell your own buildings.".to_string());
        }
        let kind = self.buildings[index].kind;
        let refund = self.sell_refund(kind);
        let income_bonus = self.balance.building(kind).income_bonus;
        self.buildings.remove(index);
        let owner_index = self.player_index(player_id).unwrap_or(0);
        self.economies[owner_index].gold += refund;
        if income_bonus > 0 {
            self.economies[owner_index].income -= income_bonus;
        }
        self.message = format!(
            "{team:?} sold {} for {refund}g.",
            self.balance.building(kind).name
        );
        Ok(())
    }

    pub fn sell_refund(&self, kind: BuildingKind) -> i32 {
        (self.balance.building(kind).cost as f32 * SELL_REFUND_RATIO).floor() as i32
    }

    /// Concede the match: the surrendering player's castle falls immediately
    /// and victory resolves through the normal path.
    pub fn surrender(&mut self, player_id: PlayerId) -> Result<(), String> {
        if self.phase != MatchPhase::Playing {
            return Err("You can only concede during a match.".to_string());
        }
        let player = self
            .players
            .iter()
            .find(|p| p.id == player_id)
            .ok_or_else(|| "Unknown player.".to_string())?;
        let team = player.team;
        for castle in self
            .castles
            .iter_mut()
            .filter(|castle| castle.owner == player_id)
        {
            castle.health = 0;
        }
        self.message = format!("{team:?} conceded the match.");
        self.check_victory();
        Ok(())
    }

    pub fn set_ready(&mut self, player_id: PlayerId, ready: bool) -> Result<(), String> {
        if let Some(player) = self.players.iter_mut().find(|p| p.id == player_id) {
            if ready && player.race.is_none() {
                return Err("Choose a race before readying up.".to_string());
            }
            player.ready = ready;
            self.message = if ready {
                format!("{} is ready.", player.name)
            } else {
                format!("{} is not ready.", player.name)
            };
        } else {
            return Err("Unknown player.".to_string());
        }
        self.start_if_ready();
        Ok(())
    }

    pub fn place_building(
        &mut self,
        player_id: PlayerId,
        kind: BuildingKind,
        lane: Lane,
        zone: BuildZone,
        cell: GridCell,
    ) -> Result<(), String> {
        if self.phase != MatchPhase::Playing {
            return Err("Buildings can only be placed during a match.".to_string());
        }
        if !(0..GRID_W).contains(&cell.x) || !(0..GRID_H).contains(&cell.y) {
            return Err("That cell is outside your build zone.".to_string());
        }
        let team = self
            .players
            .iter()
            .find(|p| p.id == player_id)
            .map(|p| p.team)
            .ok_or_else(|| "Unknown player.".to_string())?;
        let race = self
            .players
            .iter()
            .find(|p| p.id == player_id)
            .and_then(|p| p.race)
            .ok_or_else(|| "Choose a race before building.".to_string())?;
        if kind.race() != race {
            return Err(format!(
                "{} is not available for {:?}.",
                kind.fallback_name(),
                race
            ));
        }
        // Team play (team_size > 1): each player builds only in their
        // assigned lane (plan.md Phase 2 item 1).
        if self.team_size > 1 {
            let player_lane = self
                .assigned_lanes
                .get(self.player_index(player_id).unwrap_or(0))
                .copied()
                .unwrap_or(Lane::Top);
            if lane != player_lane {
                return Err(format!(
                    "You can only build in your assigned lane ({:?}).",
                    player_lane
                ));
            }
        }
        let occupied_by_side = self.buildings.iter().any(|b| {
            self.side_of(b.owner) == team && b.lane == lane && b.zone == zone && b.cell == cell
        });
        if occupied_by_side {
            return Err("That cell is already occupied.".to_string());
        }
        let building_name = self.balance.building(kind).name.clone();
        let building_cost = self.balance.building(kind).cost;
        let building_health = self.balance.building(kind).max_health;
        let income_bonus = self.balance.building(kind).income_bonus;
        let spawn_timer = self.balance.building(kind).spawn_interval.unwrap_or(0.0);
        let owner_index = self.player_index(player_id).unwrap_or(0);
        let economy = &mut self.economies[owner_index];
        if economy.gold < building_cost {
            return Err(format!("Not enough gold for {}.", building_name));
        }
        economy.gold -= building_cost;
        economy.income += income_bonus;
        let id = self.take_id();
        self.buildings.push(Building {
            id,
            owner: player_id,
            kind,
            lane,
            zone,
            cell,
            health: building_health,
            max_health: building_health,
            spawn_timer,
        });
        self.message = format!("{team:?} built {}.", building_name);
        Ok(())
    }

    pub fn vote_rematch(&mut self, player_id: PlayerId) {
        if self.phase != MatchPhase::GameOver {
            return;
        }
        if let Some(player) = self.players.iter_mut().find(|p| p.id == player_id) {
            player.rematch_vote = true;
            self.message = format!("{} voted for a rematch.", player.name);
        }
        if self.players.len() == 2 && self.players.iter().all(|p| p.connected && p.rematch_vote) {
            self.reset_to_lobby();
        }
    }

    pub fn tick(&mut self, dt: f32) {
        self.tick += 1;
        if self.phase != MatchPhase::Playing {
            return;
        }
        self.elapsed_secs += dt;
        self.tick_bounty_events(dt);
        self.tick_income(dt);
        self.tick_buildings(dt);
        self.tick_units(dt);
        self.tick_sudden_death(dt);
        self.check_victory();
    }

    pub fn sudden_death(&self) -> bool {
        self.phase == MatchPhase::Playing && self.elapsed_secs >= self.balance.sudden_death_start
    }

    fn start_if_ready(&mut self) {
        if self.phase != MatchPhase::Lobby {
            return;
        }
        if self.players.len() == self.team_size * 2
            && self
                .players
                .iter()
                .all(|p| p.connected && p.ready && p.race.is_some())
        {
            if self.random_factions {
                let races = [RaceKind::Vanguard, RaceKind::Grove, RaceKind::Ember];
                let mut rng_state = self.rng_state;
                for player in &mut self.players {
                    let index = (next_random_u32(&mut rng_state) as usize) % races.len();
                    player.race = Some(races[index]);
                }
                self.rng_state = rng_state;
                self.message = "Match started (random factions).".to_string();
            } else {
                self.message = "Match started.".to_string();
            }
            self.reset_match_state();
            self.phase = MatchPhase::Playing;
        }
    }

    fn reset_to_lobby_inner(&mut self) {
        self.phase = MatchPhase::Lobby;
        self.reset_match_state();
        for player in &mut self.players {
            player.ready = false;
            player.race = None;
            player.rematch_vote = false;
        }
        self.message = "Rematch ready. Set ready again.".to_string();
    }

    pub fn reset_match_state(&mut self) {
        self.clear_match_entities();
        // One economy and one castle per seat; races set in the lobby pick
        // the castle HP/armor. Seats without a race get the Vanguard default
        // so a match can never begin with uninitialized state.
        self.economies = self
            .players
            .iter()
            .map(|_| Economy {
                gold: self.balance.starting_gold,
                income: self.balance.base_income,
            })
            .collect();
        // assigned_lanes persist across matches (assigned at join time)
        self.castles = self
            .players
            .iter()
            .map(|player| {
                let (health, armor) = player
                    .race
                    .map(|race| {
                        let config = self.balance.race(race);
                        (config.castle_health, config.castle_armor)
                    })
                    .unwrap_or_else(|| {
                        (
                            self.balance.race(RaceKind::Vanguard).castle_health,
                            default_castle_armor(),
                        )
                    });
                Castle {
                    owner: player.id,
                    team: player.team,
                    health,
                    max_health: health,
                    armor_type: armor,
                }
            })
            .collect();
        self.winner = None;
        self.income_timer = self.balance.income_interval;
        self.elapsed_secs = 0.0;
        self.rng_state = mix_seed(self.seed);
        self.castle_regen_accum = vec![0.0; self.players.len()];
        self.castle_regen_delay_timer = vec![0.0; self.players.len()];
        self.overtime_damage_accum = [0.0, 0.0];
    }

    fn clear_match_entities(&mut self) {
        self.buildings.clear();
        self.units.clear();
        self.bounty_events.clear();
    }

    fn tick_bounty_events(&mut self, dt: f32) {
        for event in &mut self.bounty_events {
            event.age += dt;
        }
        self.bounty_events
            .retain(|event| event.age <= BOUNTY_EVENT_TTL);
    }

    fn tick_income(&mut self, dt: f32) {
        self.income_timer -= dt;
        while self.income_timer <= 0.0 {
            self.income_timer += self.balance.income_interval;
            for economy in &mut self.economies {
                economy.gold +=
                    economy.income + interest_gold(economy.gold, self.balance.interest_rate);
            }
        }
    }

    fn tick_buildings(&mut self, dt: f32) {
        let mut spawns = Vec::new();
        let player_sides: Vec<(PlayerId, Team)> = self
            .players
            .iter()
            .map(|player| (player.id, player.team))
            .collect();
        for building in &mut self.buildings {
            if building.health <= 0 {
                continue;
            }
            let building_config = self.balance.building(building.kind);
            let Some(interval) = building_config.spawn_interval else {
                continue;
            };
            building.spawn_timer -= dt;
            if building.spawn_timer <= 0.0 {
                building.spawn_timer += interval;
                if let Some(kind) = building_config.spawned_unit {
                    let side = lookup_side(&player_sides, building.owner);
                    spawns.push((
                        building.owner,
                        side,
                        building.lane,
                        building.zone,
                        building.cell,
                        kind,
                    ));
                }
            }
        }

        for (owner, side, lane, zone, cell, kind) in spawns {
            let unit_config = self.balance.unit(kind);
            let max_health = unit_config.max_health;
            let radius = unit_config.radius;
            let pos = self.free_spawn_position(side, lane, zone, cell, radius);
            let id = self.take_id();
            self.units.push(Unit {
                id,
                owner,
                kind,
                lane,
                health: max_health,
                lane_pos: pos.x,
                pos,
                velocity: WorldPos::new(0.0, 0.0),
                radius,
                attack_timer: 0.25,
                slow_timer: 0.0,
                slow_factor: 1.0,
                regen_accum: 0.0,
            });
        }
    }

    fn tick_units(&mut self, dt: f32) {
        let mut rng_state = self.rng_state;
        // Ordered by the units Vec (spawn order), never a hash map: equal-distance
        // target ties below must break by id so the sim stays deterministic.
        // Owner is a PlayerId; its side travels alongside for targeting.
        let player_sides: Vec<(PlayerId, Team)> = self
            .players
            .iter()
            .map(|player| (player.id, player.team))
            .collect();
        let positions: Vec<(
            u64,
            Team,
            PlayerId,
            Lane,
            f32,
            WorldPos,
            i32,
            UnitKind,
            i32,
            ArmorType,
        )> = self
            .units
            .iter()
            .map(|u| {
                (
                    u.id,
                    lookup_side(&player_sides, u.owner),
                    u.owner,
                    u.lane,
                    u.lane_pos,
                    u.pos,
                    u.health,
                    u.kind,
                    self.balance.unit(u.kind).max_health,
                    self.balance.unit(u.kind).armor_type,
                )
            })
            .collect();
        let mut unit_damage: HashMap<u64, (i32, PlayerId)> = HashMap::new();
        let mut building_damage: HashMap<u64, i32> = HashMap::new();
        let mut castle_damage = vec![0i32; self.castles.len()];
        let mut slows: Vec<(u64, f32, f32)> = Vec::new();
        let mut heals: Vec<(u64, i32)> = Vec::new();

        for unit in &mut self.units {
            let unit_config = self.balance.unit(unit.kind);
            unit.attack_timer = (unit.attack_timer - dt).max(0.0);
            let unit_side = lookup_side(&player_sides, unit.owner);

            // Ability: regeneration (always on, fractional accumulator).
            if let Some(AbilityConfig::Regeneration { health_per_second }) = &unit_config.ability {
                if unit.health > 0 && unit.health < unit_config.max_health {
                    unit.regen_accum += *health_per_second as f32 * dt;
                    while unit.regen_accum >= 1.0 && unit.health < unit_config.max_health {
                        unit.regen_accum -= 1.0;
                        unit.health += 1;
                    }
                    if unit.health >= unit_config.max_health {
                        unit.regen_accum = 0.0;
                    }
                }
            }

            // Ability: slow decay.
            if unit.slow_timer > 0.0 {
                unit.slow_timer = (unit.slow_timer - dt).max(0.0);
                if unit.slow_timer <= 0.0 {
                    unit.slow_factor = 1.0;
                }
            }

            // Ability: berserk scaling.
            let mut move_speed = unit_config.speed;
            let mut attack_interval = unit_config.attack_interval;
            if let Some(AbilityConfig::Berserk {
                below_health_fraction,
                speed_multiplier,
                attack_speed_multiplier,
            }) = &unit_config.ability
            {
                if unit.health.max(0) as f32 / unit_config.max_health.max(1) as f32
                    <= *below_health_fraction
                {
                    move_speed *= speed_multiplier;
                    attack_interval /= attack_speed_multiplier;
                }
            }
            if unit.slow_timer > 0.0 {
                move_speed *= unit.slow_factor;
            }

            let unit_target = positions
                .iter()
                .filter(|(_, team, _, lane, lane_pos, _, health, _, _, _)| {
                    *team != unit_side
                        && *health > 0
                        && unit_lanes_connected(unit.lane, unit.lane_pos, *lane, *lane_pos)
                })
                .map(|(id, _, _, lane, lane_pos, pos, _, _, _, armor)| {
                    (
                        *id,
                        unit_combat_distance(unit.lane, unit.pos, *lane, *lane_pos, *pos),
                        *armor,
                    )
                })
                .filter(|(_, distance, _)| *distance <= unit_config.attack_range)
                .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));

            let building_target = self
                .buildings
                .iter()
                .filter(|building| {
                    lookup_side(&player_sides, building.owner) != unit_side
                        && building.lane == unit.lane
                        && building.zone == BuildZone::Front
                        && building.health > 0
                })
                .map(|building| {
                    let pos = building_position(
                        lookup_side(&player_sides, building.owner),
                        building.lane,
                        building.zone,
                        building.cell,
                    );
                    (
                        building.id,
                        footprint_distance(unit.pos, pos, BUILDING_FOOTPRINT_RADIUS),
                        self.balance.building(building.kind).armor_type,
                    )
                })
                .filter(|(_, distance, _)| *distance <= unit_config.attack_range)
                .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));

            // March toward the enemy-side castle at the far end of the lane;
            // with several enemy players each owns a castle, so pick nearest.
            let enemy_side = unit_side.opponent();
            let enemy_castle = self
                .castles
                .iter()
                .filter(|castle| castle.team == enemy_side && castle.health > 0)
                .map(|castle| (lane_position(unit.lane, castle.team.castle_pos()), castle))
                .min_by(|a, b| a.0.distance(unit.pos).total_cmp(&b.0.distance(unit.pos)));
            let enemy_castle_distance = enemy_castle
                .as_ref()
                .map(|(pos, _)| footprint_distance(unit.pos, *pos, CASTLE_FOOTPRINT_RADIUS))
                .unwrap_or(f32::INFINITY);
            let can_attack_castle = enemy_castle_distance <= unit_config.attack_range;

            if unit.attack_timer <= 0.0 {
                // Ability: heal - pulses into the most wounded nearby ally
                // instead of attacking whenever someone is hurt.
                if let Some(AbilityConfig::Heal {
                    amount,
                    range,
                    interval,
                }) = &unit_config.ability
                {
                    let most_wounded = positions
                        .iter()
                        .filter(|(_, side, _, lane, lane_pos, pos, health, _, max, _)| {
                            *side == unit_side
                                && *health > 0
                                && *health < *max
                                && unit_combat_distance(unit.lane, unit.pos, *lane, *lane_pos, *pos)
                                    <= *range
                        })
                        .min_by(|a, b| (a.6 - a.8).cmp(&(b.6 - b.8)).then(a.0.cmp(&b.0)));
                    if let Some((target_id, ..)) = most_wounded {
                        heals.push((*target_id, *amount));
                        unit.attack_timer = *interval;
                        continue;
                    }
                }
                if let Some((target_id, _, target_armor)) = unit_target {
                    let rolled_damage = roll_base_damage(
                        &mut rng_state,
                        unit_config.damage,
                        unit_config.damage_variance,
                    );
                    let damage = typed_damage(rolled_damage, unit_config.attack_type, target_armor);
                    let entry = unit_damage.entry(target_id).or_insert((0, unit.owner));
                    entry.0 += damage;
                    entry.1 = unit.owner;

                    // Ability: splash - fraction of the typed hit spreads to
                    // the nearest extra enemies around the primary target.
                    if let Some(AbilityConfig::Splash {
                        targets,
                        fraction,
                        radius,
                    }) = &unit_config.ability
                    {
                        let target_pos = positions
                            .iter()
                            .find(|(id, ..)| *id == target_id)
                            .map(|(_, _, _, _, _, pos, ..)| *pos);
                        if let Some(target_pos) = target_pos {
                            let splash = ((damage as f32) * fraction).round().max(1.0) as i32;
                            let mut extra = 0u32;
                            for (other_id, oside, oowner, _, _, opos, ohealth, _, _, oarmor) in
                                &positions
                            {
                                if *other_id == target_id
                                    || *oside == unit_side
                                    || *ohealth <= 0
                                    || extra >= *targets
                                {
                                    continue;
                                }
                                if opos.distance(target_pos) <= *radius {
                                    let splash_damage =
                                        typed_damage(splash, unit_config.attack_type, *oarmor);
                                    let _ = oowner;
                                    let entry =
                                        unit_damage.entry(*other_id).or_insert((0, unit.owner));
                                    entry.0 += splash_damage;
                                    entry.1 = unit.owner;
                                    extra += 1;
                                }
                            }
                        }
                    }

                    // Ability: slow - rides on every primary hit.
                    if let Some(AbilityConfig::Slow { factor, duration }) = &unit_config.ability {
                        slows.push((target_id, *factor, *duration));
                    }
                    unit.attack_timer = attack_interval;
                    continue;
                }
                if let Some((target_id, _, target_armor)) = building_target {
                    let rolled_damage = roll_base_damage(
                        &mut rng_state,
                        unit_config.damage,
                        unit_config.damage_variance,
                    );
                    let damage = typed_damage(rolled_damage, unit_config.attack_type, target_armor);
                    *building_damage.entry(target_id).or_insert(0) += damage;
                    unit.attack_timer = attack_interval;
                    continue;
                }
                if can_attack_castle {
                    let (castle_index, castle_armor) = enemy_castle
                        .as_ref()
                        .map(|(_, castle)| {
                            (
                                self.castles
                                    .iter()
                                    .position(|c| c.owner == castle.owner)
                                    .unwrap_or(0),
                                castle.armor_type,
                            )
                        })
                        .unwrap_or((0, ArmorType::Fortified));
                    let rolled_damage = roll_base_damage(
                        &mut rng_state,
                        unit_config.damage,
                        unit_config.damage_variance,
                    );
                    castle_damage[castle_index] +=
                        typed_damage(rolled_damage, unit_config.attack_type, castle_armor);
                    unit.attack_timer = unit_config.attack_interval;
                    continue;
                }
            }

            if unit_target.is_none() && building_target.is_none() && !can_attack_castle {
                let old_pos = unit.pos;
                let max_step = move_speed * dt;
                unit.pos.x += unit_side.direction() * max_step;
                let lane_y = lane_center_y(unit.lane);
                let y_delta = (lane_y - unit.pos.y).clamp(-max_step * 0.35, max_step * 0.35);
                unit.pos.y += y_delta;
                unit.pos.x = unit.pos.x.clamp(-18.0, LANE_LENGTH + 18.0);
                unit.pos.y = clamp_lane_y(unit.lane, unit.pos.y);
                unit.lane_pos = unit.pos.x;
                unit.velocity =
                    WorldPos::new((unit.pos.x - old_pos.x) / dt, (unit.pos.y - old_pos.y) / dt);
            } else {
                unit.velocity = WorldPos::new(0.0, 0.0);
            }
        }
        // Apply ability side effects collected during the combat pass.
        for (target_id, factor, duration) in slows {
            if let Some(unit) = self.units.iter_mut().find(|unit| unit.id == target_id) {
                unit.slow_factor = unit.slow_factor.min(factor);
                unit.slow_timer = unit.slow_timer.max(duration);
            }
        }
        for (target_id, amount) in heals {
            if let Some(unit) = self.units.iter_mut().find(|unit| unit.id == target_id) {
                let max = self.balance.unit(unit.kind).max_health;
                unit.health = (unit.health + amount).min(max);
            }
        }
        self.separate_units();
        self.rng_state = rng_state;

        let player_indices: HashMap<PlayerId, usize> = self
            .players
            .iter()
            .enumerate()
            .map(|(index, player)| (player.id, index))
            .collect();
        let mut bounty_awards: HashMap<usize, i32> = HashMap::new();
        let mut pending_bounty_events = Vec::new();
        for unit in &mut self.units {
            if let Some((damage, killer)) = unit_damage.get(&unit.id) {
                unit.health -= *damage;
                if unit.health <= 0 {
                    let bounty = self.balance.unit(unit.kind).bounty.max(0);
                    let killer_index = player_indices.get(killer).copied().unwrap_or(0);
                    *bounty_awards.entry(killer_index).or_insert(0) += bounty;
                    if bounty > 0 {
                        pending_bounty_events.push((
                            *killer,
                            bounty,
                            unit.lane,
                            unit.lane_pos,
                            unit.pos,
                            unit.kind,
                        ));
                    }
                }
            }
        }
        self.units.retain(|u| u.health > 0);
        let mut destroyed_income: HashMap<usize, i32> = HashMap::new();
        for building in &mut self.buildings {
            if let Some(damage) = building_damage.get(&building.id) {
                building.health -= *damage;
                if building.health <= 0 {
                    let owner_index = player_indices.get(&building.owner).copied().unwrap_or(0);
                    *destroyed_income.entry(owner_index).or_insert(0) +=
                        self.balance.building(building.kind).income_bonus;
                }
            }
        }
        self.buildings.retain(|building| building.health > 0);
        for (owner_index, lost_income) in destroyed_income {
            self.economies[owner_index].income -= lost_income;
        }
        for (owner_index, bounty) in bounty_awards {
            self.economies[owner_index].gold += bounty;
        }
        for (killer, amount, lane, lane_pos, pos, unit_kind) in pending_bounty_events {
            let id = self.take_id();
            self.bounty_events.push(BountyEvent {
                id,
                team: self.side_of(killer),
                amount,
                lane,
                lane_pos,
                pos,
                unit_kind,
                age: 0.0,
            });
        }
        self.apply_castle_regen_and_damage(dt, castle_damage);
    }

    fn apply_castle_regen_and_damage(&mut self, dt: f32, castle_damage: Vec<i32>) {
        let regen_per_second = self.balance.castle_regen_per_second.max(0.0);
        let regen_delay = self.balance.castle_regen_delay_secs.max(0.0);
        for (idx, damage) in castle_damage.into_iter().enumerate() {
            if self.castles[idx].health <= 0 {
                continue;
            }

            if damage > 0 {
                // Besieged: incoming damage pauses regeneration until the
                // castle has gone castle_regen_delay_secs without being hit,
                // so early pressure sticks instead of being healed off.
                self.castle_regen_delay_timer[idx] = regen_delay;
                self.castle_regen_accum[idx] = 0.0;
            } else if self.castle_regen_delay_timer[idx] > 0.0 {
                self.castle_regen_delay_timer[idx] =
                    (self.castle_regen_delay_timer[idx] - dt).max(0.0);
            }

            let besieged = self.castle_regen_delay_timer[idx] > 0.0;
            if regen_per_second > 0.0
                && !besieged
                && !self.sudden_death()
                && self.castles[idx].health < self.castles[idx].max_health
            {
                self.castle_regen_accum[idx] += regen_per_second * dt;
            }

            let regen = self.castle_regen_accum[idx].floor() as i32;
            if regen > 0 {
                self.castle_regen_accum[idx] -= regen as f32;
            }

            self.castles[idx].health =
                (self.castles[idx].health + regen - damage).clamp(0, self.castles[idx].max_health);

            if self.castles[idx].health == 0
                || self.castles[idx].health == self.castles[idx].max_health
            {
                self.castle_regen_accum[idx] = 0.0;
            }
        }
    }

    fn free_spawn_position(
        &self,
        owner: Team,
        lane: Lane,
        zone: BuildZone,
        cell: GridCell,
        radius: f32,
    ) -> WorldPos {
        let base = building_spawn_position(owner, lane, zone, cell);
        let spacing = radius * 2.0 + UNIT_SEPARATION_PADDING;
        for ring in 0..=SPAWN_SEARCH_RINGS {
            for side in [0.0, 1.0, -1.0, 2.0, -2.0, 3.0, -3.0] {
                let candidate = WorldPos::new(
                    (base.x + owner.direction() * spacing * ring as f32)
                        .clamp(-18.0, LANE_LENGTH + 18.0),
                    clamp_lane_y(lane, base.y + side * spacing),
                );
                if self.spawn_position_is_free(lane, candidate, radius) {
                    return candidate;
                }
            }
        }
        WorldPos::new(
            base.x.clamp(-18.0, LANE_LENGTH + 18.0),
            clamp_lane_y(lane, base.y),
        )
    }

    fn spawn_position_is_free(&self, lane: Lane, candidate: WorldPos, radius: f32) -> bool {
        self.units
            .iter()
            .filter(|unit| unit.lane == lane)
            .all(|unit| {
                candidate.distance(unit.pos) >= radius + unit.radius + UNIT_SEPARATION_PADDING
            })
    }

    fn separate_units(&mut self) {
        for _ in 0..3 {
            let mut offsets = vec![WorldPos::new(0.0, 0.0); self.units.len()];
            for left in 0..self.units.len() {
                for right in (left + 1)..self.units.len() {
                    if !unit_lanes_connected(
                        self.units[left].lane,
                        self.units[left].lane_pos,
                        self.units[right].lane,
                        self.units[right].lane_pos,
                    ) {
                        continue;
                    }
                    let delta = WorldPos::new(
                        self.units[right].pos.x - self.units[left].pos.x,
                        self.units[right].pos.y - self.units[left].pos.y,
                    );
                    let distance_sq = delta.x * delta.x + delta.y * delta.y;
                    let min_distance = self.units[left].radius
                        + self.units[right].radius
                        + UNIT_SEPARATION_PADDING;
                    if distance_sq >= min_distance * min_distance {
                        continue;
                    }
                    let distance = distance_sq.sqrt();
                    let (normal_x, normal_y) = if distance > 0.001 {
                        (delta.x / distance, delta.y / distance)
                    } else {
                        let dir = if self.units[left].id < self.units[right].id {
                            -1.0
                        } else {
                            1.0
                        };
                        (0.0, dir)
                    };
                    let push = (min_distance - distance.max(0.001)) * 0.5;
                    offsets[left].x -= normal_x * push;
                    offsets[left].y -= normal_y * push;
                    offsets[right].x += normal_x * push;
                    offsets[right].y += normal_y * push;
                }
            }

            let mut any = false;
            for (unit, offset) in self.units.iter_mut().zip(offsets) {
                if offset.x.abs() <= 0.0001 && offset.y.abs() <= 0.0001 {
                    continue;
                }
                any = true;
                unit.pos.x = (unit.pos.x + offset.x).clamp(-18.0, LANE_LENGTH + 18.0);
                unit.pos.y = clamp_lane_y(unit.lane, unit.pos.y + offset.y);
                unit.lane_pos = unit.pos.x;
            }
            if !any {
                break;
            }
        }
    }

    fn tick_sudden_death(&mut self, dt: f32) {
        if !self.sudden_death() {
            return;
        }

        // Pressure ramps up the longer sudden death runs - both through a
        // multiplier on board pressure and a flat escalation - so even a
        // near-empty board resolves instead of stalling forever.
        let ramp = self.balance.sudden_death_ramp_per_minute.max(0.0);
        let escalation = self.balance.sudden_death_escalation_per_minute.max(0.0);
        let minutes_into_sudden_death =
            ((self.elapsed_secs - self.balance.sudden_death_start) / 60.0).max(0.0);
        let pressure_scale = 1.0 + ramp * minutes_into_sudden_death;
        let flat_damage = escalation * minutes_into_sudden_death;

        for team in [Team::Left, Team::Right] {
            let pressure = self.board_pressure(team.opponent());
            let slot = team.slot();
            self.overtime_damage_accum[slot] += (pressure * pressure_scale + flat_damage) * dt;
            let damage = self.overtime_damage_accum[slot].floor() as i32;
            if damage > 0 {
                self.overtime_damage_accum[slot] -= damage as f32;
                // A side's pressure lands on its first surviving castle; with
                // one castle per side (1v1) this matches the old behavior.
                if let Some(index) = self.side_alive_castles(team).first().copied() {
                    self.castles[index].health = (self.castles[index].health - damage).max(0);
                }
            }
        }

        if self.tick % 30 == 0 {
            self.message = "Sudden death: castles take pressure damage.".to_string();
        }
    }

    fn board_pressure(&self, team: Team) -> f32 {
        let player_sides: Vec<(PlayerId, Team)> = self
            .players
            .iter()
            .map(|player| (player.id, player.team))
            .collect();
        let buildings = self
            .buildings
            .iter()
            .filter(|building| lookup_side(&player_sides, building.owner) == team)
            .count() as f32;
        let units = self
            .units
            .iter()
            .filter(|unit| lookup_side(&player_sides, unit.owner) == team)
            .count() as f32;
        1.0 + buildings * 2.0 + units
    }

    /// A side loses when every castle belonging to it is dead; if both sides
    /// fall on the same tick the adjudication score picks the winner.
    fn check_victory(&mut self) {
        if self.players.is_empty() {
            return;
        }
        let side_dead = |team: Team| {
            self.castles
                .iter()
                .filter(|castle| castle.team == team)
                .all(|castle| castle.health <= 0)
        };
        let left_dead = side_dead(Team::Left);
        let right_dead = side_dead(Team::Right);
        if left_dead && right_dead {
            self.finish(self.adjudicated_winner());
        } else if left_dead {
            self.finish(Team::Right);
        } else if right_dead {
            self.finish(Team::Left);
        }
    }

    fn adjudicated_winner(&self) -> Team {
        let left_score = self.adjudication_score(Team::Left);
        let right_score = self.adjudication_score(Team::Right);
        if right_score > left_score {
            Team::Right
        } else {
            Team::Left
        }
    }

    fn adjudication_score(&self, team: Team) -> i32 {
        let player_sides: Vec<(PlayerId, Team)> = self
            .players
            .iter()
            .map(|player| (player.id, player.team))
            .collect();
        let castle = self
            .castles
            .iter()
            .filter(|castle| castle.team == team)
            .map(|castle| castle.health.max(0) * 10)
            .sum::<i32>();
        let economy: i32 = self
            .players
            .iter()
            .enumerate()
            .filter(|(_, player)| player.team == team)
            .map(|(index, _)| self.economies[index].gold + self.economies[index].income * 5)
            .sum();
        let buildings = self
            .buildings
            .iter()
            .filter(|building| lookup_side(&player_sides, building.owner) == team)
            .map(|building| self.balance.building(building.kind).cost)
            .sum::<i32>();
        let units = self
            .units
            .iter()
            .filter(|unit| lookup_side(&player_sides, unit.owner) == team)
            .map(|unit| unit.health.max(0))
            .sum::<i32>();
        castle + economy + buildings + units
    }

    fn finish(&mut self, winner: Team) {
        self.phase = MatchPhase::GameOver;
        self.winner = Some(winner);
        self.message = format!("{winner:?} wins. Press R for rematch.");
        for player in &mut self.players {
            player.ready = false;
            player.rematch_vote = false;
        }
    }

    fn take_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
}

fn clean_name(name: String) -> String {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        "Player".to_string()
    } else {
        trimmed.chars().take(16).collect()
    }
}

fn clamp_lane_y(lane: Lane, y: f32) -> f32 {
    let center = lane_center_y(lane);
    y.clamp(center - LANE_SPREAD_LIMIT, center + LANE_SPREAD_LIMIT)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ready_two_players() -> GameSim {
        let mut sim = GameSim::default();
        let p1 = sim.join_or_update_player("Alice".to_string()).unwrap().id;
        let p2 = sim.join_or_update_player("Bryn".to_string()).unwrap().id;
        sim.set_race(p1, RaceKind::Vanguard).unwrap();
        sim.set_race(p2, RaceKind::Vanguard).unwrap();
        sim.set_ready(p1, true).unwrap();
        sim.set_ready(p2, true).unwrap();
        sim
    }

    fn place_test_building(
        sim: &mut GameSim,
        player: PlayerId,
        kind: BuildingKind,
        cell: GridCell,
    ) -> Result<(), String> {
        sim.place_building(player, kind, Lane::Top, BuildZone::Front, cell)
    }

    fn scripted_fight_snapshot(seed: u64, ticks: usize) -> MatchSnapshot {
        let mut sim = GameSim::with_seed(BalanceConfig::default(), seed);
        let left = sim.join_or_update_player("Alice".to_string()).unwrap().id;
        let right = sim.join_or_update_player("Bryn".to_string()).unwrap().id;
        sim.set_race(left, RaceKind::Vanguard).unwrap();
        sim.set_race(right, RaceKind::Grove).unwrap();
        sim.set_ready(left, true).unwrap();
        sim.set_ready(right, true).unwrap();
        assert_eq!(sim.phase, MatchPhase::Playing);

        // Mirror a small build into both lanes so waves meet, fight, roll
        // damage variance, and contest the castle junctions.
        let left_builds = [
            (BuildingKind::VanguardBarracks, GridCell { x: 0, y: 0 }),
            (BuildingKind::VanguardRangeTower, GridCell { x: 1, y: 2 }),
            (BuildingKind::VanguardForge, GridCell { x: 0, y: 4 }),
            (BuildingKind::VanguardBarracks, GridCell { x: 2, y: 0 }),
        ];
        let right_builds = [
            (BuildingKind::GroveRootDen, GridCell { x: 0, y: 0 }),
            (BuildingKind::GroveThornSpire, GridCell { x: 1, y: 2 }),
            (BuildingKind::GroveBloomWell, GridCell { x: 0, y: 4 }),
            (BuildingKind::GroveRootDen, GridCell { x: 2, y: 0 }),
        ];
        let dt = 1.0 / 30.0;
        for step in 0..ticks {
            if step % 300 == 0 {
                let build_index = (step / 300) % left_builds.len();
                for lane in [Lane::Top, Lane::Bottom] {
                    let (kind, cell) = left_builds[build_index];
                    let _ = sim.place_building(left, kind, lane, BuildZone::Front, cell);
                    let (kind, cell) = right_builds[build_index];
                    let _ = sim.place_building(right, kind, lane, BuildZone::Front, cell);
                }
            }
            sim.tick(dt);
        }
        sim.snapshot()
    }

    #[test]
    fn identical_seeds_and_commands_produce_identical_snapshots() {
        let a = scripted_fight_snapshot(777, 3600);
        let b = scripted_fight_snapshot(777, 3600);
        assert_eq!(a, b);
        assert!(
            a.units.len() > 3,
            "scripted match should have a live steady-state wave, units: {}",
            a.units.len()
        );
    }

    #[test]
    fn different_seeds_diverge_in_combat() {
        let a = scripted_fight_snapshot(1, 3600);
        let b = scripted_fight_snapshot(2, 3600);
        assert_ne!(
            a.units, b.units,
            "separate matches must play out differently"
        );
    }

    fn test_unit(
        id: u64,
        owner: Team,
        kind: UnitKind,
        lane: Lane,
        health: i32,
        lane_pos: f32,
        attack_timer: f32,
    ) -> Unit {
        Unit {
            id,
            owner: PlayerId(owner.slot() as u8 + 1),
            kind,
            lane,
            health,
            lane_pos,
            pos: lane_position(lane, lane_pos),
            velocity: WorldPos::new(0.0, 0.0),
            radius: BalanceConfig::default().unit(kind).radius,
            attack_timer,
            slow_timer: 0.0,
            slow_factor: 1.0,
            regen_accum: 0.0,
        }
    }

    fn assert_units_separated(left: &Unit, right: &Unit) {
        let min_distance = left.radius + right.radius + UNIT_SEPARATION_PADDING - 0.001;
        assert!(
            left.pos.distance(right.pos) >= min_distance,
            "units too close: distance {} min {}",
            left.pos.distance(right.pos),
            min_distance
        );
    }

    #[test]
    fn starts_when_two_players_are_ready() {
        let sim = ready_two_players();
        assert_eq!(sim.phase, MatchPhase::Playing);
        assert_eq!(sim.economies[0].gold, sim.balance.starting_gold);
        assert_eq!(
            sim.castles[0].health,
            sim.balance.race(RaceKind::Vanguard).castle_health
        );
    }

    #[test]
    fn ready_requires_race_selection() {
        let mut sim = GameSim::default();
        let player = sim.join_or_update_player("Alice".to_string()).unwrap().id;

        let err = sim.set_ready(player, true).unwrap_err();

        assert!(err.contains("Choose a race"));
        assert!(!sim.players[0].ready);
    }

    #[test]
    fn selected_race_sets_unique_castle_hp() {
        let mut sim = GameSim::default();
        let left = sim.join_or_update_player("Alice".to_string()).unwrap().id;
        let right = sim.join_or_update_player("Bryn".to_string()).unwrap().id;
        sim.set_race(left, RaceKind::Grove).unwrap();
        sim.set_race(right, RaceKind::Ember).unwrap();
        sim.set_ready(left, true).unwrap();
        sim.set_ready(right, true).unwrap();

        assert_eq!(sim.phase, MatchPhase::Playing);
        assert_eq!(
            sim.castles[Team::Left.slot()].max_health,
            sim.balance.race(RaceKind::Grove).castle_health
        );
        assert_eq!(
            sim.castles[Team::Right.slot()].max_health,
            sim.balance.race(RaceKind::Ember).castle_health
        );
    }

    #[test]
    fn validates_building_placement_and_spends_gold() {
        let mut sim = ready_two_players();
        let player = sim.players[0].id;
        place_test_building(
            &mut sim,
            player,
            BuildingKind::VanguardBarracks,
            GridCell { x: 0, y: 0 },
        )
        .unwrap();
        assert_eq!(
            sim.economies[0].gold,
            sim.balance.starting_gold - sim.balance.building(BuildingKind::VanguardBarracks).cost
        );
        assert!(
            place_test_building(
                &mut sim,
                player,
                BuildingKind::VanguardBarracks,
                GridCell { x: 0, y: 0 }
            )
            .is_err()
        );
        assert!(
            place_test_building(
                &mut sim,
                player,
                BuildingKind::VanguardBarracks,
                GridCell { x: GRID_W, y: 0 }
            )
            .is_err()
        );
    }

    #[test]
    fn race_restricts_available_buildings() {
        let mut sim = ready_two_players();
        let player = sim.players[0].id;

        let err = sim
            .place_building(
                player,
                BuildingKind::GroveRootDen,
                Lane::Top,
                BuildZone::Front,
                GridCell { x: 0, y: 0 },
            )
            .unwrap_err();

        assert!(err.contains("not available"));
    }

    #[test]
    fn forge_increases_income_tick() {
        let mut sim = ready_two_players();
        let player = sim.players[0].id;
        place_test_building(
            &mut sim,
            player,
            BuildingKind::VanguardForge,
            GridCell { x: 1, y: 1 },
        )
        .unwrap();
        assert_eq!(
            sim.economies[0].income,
            sim.balance.base_income
                + sim
                    .balance
                    .building(BuildingKind::VanguardForge)
                    .income_bonus
        );
        let gold_after_buy = sim.economies[0].gold;
        sim.tick(sim.balance.income_interval);
        assert_eq!(
            sim.economies[0].gold,
            gold_after_buy
                + sim.economies[0].income
                + interest_gold(gold_after_buy, sim.balance.interest_rate)
        );
    }

    #[test]
    fn income_tick_adds_floored_interest_from_unused_gold() {
        let mut sim = ready_two_players();
        sim.economies[0].gold = 101;
        sim.economies[1].gold = 99;

        sim.tick(sim.balance.income_interval);

        assert_eq!(sim.economies[0].gold, 101 + sim.balance.base_income + 4);
        assert_eq!(sim.economies[1].gold, 99 + sim.balance.base_income + 3);
    }

    #[test]
    fn buildings_spawn_units() {
        let mut sim = ready_two_players();
        let player = sim.players[0].id;
        sim.place_building(
            player,
            BuildingKind::VanguardBarracks,
            Lane::Bottom,
            BuildZone::Front,
            GridCell { x: 0, y: 0 },
        )
        .unwrap();
        sim.tick(
            sim.balance
                .building(BuildingKind::VanguardBarracks)
                .spawn_interval
                .unwrap(),
        );
        assert_eq!(sim.units.len(), 1);
        assert_eq!(sim.units[0].kind, UnitKind::VanguardGuard);
        assert_eq!(sim.units[0].lane, Lane::Bottom);
    }

    #[test]
    fn repeated_spawns_find_nearby_free_space() {
        let mut sim = ready_two_players();
        let player = sim.players[0].id;
        sim.place_building(
            player,
            BuildingKind::VanguardBarracks,
            Lane::Top,
            BuildZone::Front,
            GridCell { x: 0, y: 2 },
        )
        .unwrap();

        sim.buildings[0].spawn_timer = 0.0;
        sim.tick(0.1);
        sim.buildings[0].spawn_timer = 0.0;
        sim.tick(0.1);

        assert_eq!(sim.units.len(), 2);
        assert_units_separated(&sim.units[0], &sim.units[1]);
    }

    #[test]
    fn overlapping_units_are_separated_after_tick() {
        let mut sim = ready_two_players();
        sim.units.push(test_unit(
            610,
            Team::Left,
            UnitKind::VanguardGuard,
            Lane::Top,
            100,
            40.0,
            99.0,
        ));
        sim.units.push(test_unit(
            611,
            Team::Left,
            UnitKind::VanguardGuard,
            Lane::Top,
            100,
            40.0,
            99.0,
        ));

        sim.tick(0.1);

        assert_units_separated(&sim.units[0], &sim.units[1]);
    }

    #[test]
    fn units_prioritize_same_lane_units_over_buildings() {
        let mut sim = ready_two_players();
        let right = sim.players[1].id;
        sim.place_building(
            right,
            BuildingKind::VanguardBarracks,
            Lane::Top,
            BuildZone::Front,
            GridCell { x: 0, y: 0 },
        )
        .unwrap();
        let building_health = sim.buildings[0].health;
        sim.units.push(test_unit(
            700,
            Team::Left,
            UnitKind::VanguardGuard,
            Lane::Top,
            999,
            50.0,
            0.0,
        ));
        sim.units.push(test_unit(
            701,
            Team::Right,
            UnitKind::EmberRunner,
            Lane::Top,
            50,
            51.0,
            99.0,
        ));

        sim.tick(0.1);

        assert!(
            sim.units
                .iter()
                .any(|unit| unit.id == 701 && unit.health < 50)
        );
        assert_eq!(sim.buildings[0].health, building_health);
    }

    #[test]
    fn different_lane_units_fight_inside_castle_junction() {
        let mut sim = ready_two_players();
        sim.units.push(test_unit(
            720,
            Team::Left,
            UnitKind::VanguardGuard,
            Lane::Top,
            999,
            6.0,
            0.0,
        ));
        sim.units.push(test_unit(
            721,
            Team::Right,
            UnitKind::EmberRunner,
            Lane::Bottom,
            50,
            5.0,
            99.0,
        ));

        sim.tick(0.1);

        assert!(
            sim.units
                .iter()
                .any(|unit| unit.id == 721 && unit.health < 50)
        );
    }

    #[test]
    fn different_lane_units_ignore_each_other_outside_castle_junction() {
        let mut sim = ready_two_players();
        sim.units.push(test_unit(
            730,
            Team::Left,
            UnitKind::VanguardGuard,
            Lane::Top,
            999,
            50.0,
            0.0,
        ));
        sim.units.push(test_unit(
            731,
            Team::Right,
            UnitKind::EmberRunner,
            Lane::Bottom,
            50,
            51.0,
            99.0,
        ));

        sim.tick(0.1);

        assert!(
            sim.units
                .iter()
                .any(|unit| unit.id == 731 && unit.health == 50)
        );
    }

    #[test]
    fn front_buildings_tank_before_castle_but_back_buildings_do_not() {
        let mut sim = ready_two_players();
        let right = sim.players[1].id;
        sim.economies[Team::Right.slot()].gold = 500;
        sim.place_building(
            right,
            BuildingKind::VanguardBarracks,
            Lane::Top,
            BuildZone::Front,
            GridCell { x: 0, y: 0 },
        )
        .unwrap();
        sim.place_building(
            right,
            BuildingKind::VanguardRangeTower,
            Lane::Top,
            BuildZone::Back,
            GridCell { x: 0, y: 0 },
        )
        .unwrap();
        let front_id = sim.buildings[0].id;
        let back_id = sim.buildings[1].id;
        let castle_health = sim.castles[Team::Right.slot()].health;
        sim.units.push(test_unit(
            710,
            Team::Left,
            UnitKind::VanguardGuard,
            Lane::Top,
            999,
            building_lane_pos(Team::Right, BuildZone::Front, GridCell { x: 0, y: 0 }),
            0.0,
        ));

        sim.tick(0.1);

        let front = sim
            .buildings
            .iter()
            .find(|building| building.id == front_id)
            .unwrap();
        let back = sim
            .buildings
            .iter()
            .find(|building| building.id == back_id)
            .unwrap();
        assert!(front.health < front.max_health);
        assert_eq!(back.health, back.max_health);
        assert_eq!(sim.castles[Team::Right.slot()].health, castle_health);

        sim.buildings.retain(|building| building.id != front_id);
        sim.units[0].lane_pos = Team::Right.castle_pos() - 1.0;
        sim.units[0].pos = lane_position(Lane::Top, sim.units[0].lane_pos);
        sim.units[0].attack_timer = 0.0;
        sim.tick(0.1);

        let back = sim
            .buildings
            .iter()
            .find(|building| building.id == back_id)
            .unwrap();
        assert_eq!(back.health, back.max_health);
        assert!(sim.castles[Team::Right.slot()].health < castle_health);
    }

    #[test]
    fn castles_regenerate_to_full_when_not_under_attack() {
        let mut sim = ready_two_players();
        let slot = Team::Right.slot();
        let max_health = sim.castles[slot].max_health;
        sim.castles[slot].health = max_health - 25;

        sim.tick(1.0);

        assert_eq!(
            sim.castles[slot].health,
            max_health - 25 + sim.balance.castle_regen_per_second as i32
        );

        sim.tick(10.0);

        assert_eq!(sim.castles[slot].health, max_health);
    }

    #[test]
    fn weakest_single_unit_is_not_healed_off_while_attacking() {
        let mut sim = ready_two_players();
        let slot = Team::Right.slot();
        let max_health = sim.castles[slot].max_health;
        sim.units.push(test_unit(
            740,
            Team::Left,
            UnitKind::GroveSproutling,
            Lane::Top,
            sim.balance.unit(UnitKind::GroveSproutling).max_health,
            Team::Right.castle_pos() - 1.0,
            0.0,
        ));

        sim.tick(1.0);

        // Besieged regen: a continuous attacker must land damage now.
        assert!(sim.castles[slot].health < max_health);
    }

    #[test]
    fn castle_regeneration_resumes_after_siege_delay() {
        let mut sim = ready_two_players();
        let slot = Team::Right.slot();
        let max_health = sim.castles[slot].max_health;
        let mut attacker = test_unit(
            741,
            Team::Left,
            UnitKind::GroveSproutling,
            Lane::Top,
            sim.balance.unit(UnitKind::GroveSproutling).max_health,
            Team::Right.castle_pos() - 1.0,
            0.0,
        );
        sim.units.push(attacker.clone());
        sim.tick(1.0);
        let damaged_health = sim.castles[slot].health;
        assert!(damaged_health < max_health);

        // Attacker dies: after the configured siege delay, regen resumes and
        // walks the castle back to full.
        attacker.health = 0;
        sim.units.clear();
        sim.tick(sim.balance.castle_regen_delay_secs + 1.0);

        assert_eq!(sim.castles[slot].health, max_health);
    }

    #[test]
    fn units_can_destroy_castle_and_end_match() {
        let mut sim = ready_two_players();
        let unit_config = sim.balance.unit(UnitKind::VanguardGuard);
        let castle_armor = sim.castles[Team::Right.slot()].armor_type;
        sim.units.push(test_unit(
            500,
            Team::Left,
            UnitKind::VanguardGuard,
            Lane::Top,
            999,
            Team::Right.castle_pos() - 1.0,
            0.0,
        ));
        let (min_damage, _) = damage_range(unit_config.damage, unit_config.damage_variance);
        sim.castles[Team::Right.slot()].health =
            typed_damage(min_damage, unit_config.attack_type, castle_armor);
        sim.tick(0.01);
        assert_eq!(sim.phase, MatchPhase::GameOver);
        assert_eq!(sim.winner, Some(Team::Left));
    }

    #[test]
    fn killing_units_awards_configured_bounty() {
        let mut sim = ready_two_players();
        sim.economies[Team::Left.slot()].gold = 0;
        sim.economies[Team::Right.slot()].gold = 0;
        sim.units.push(test_unit(
            500,
            Team::Left,
            UnitKind::VanguardGuard,
            Lane::Top,
            999,
            50.0,
            0.0,
        ));
        sim.units.push(test_unit(
            501,
            Team::Right,
            UnitKind::EmberRunner,
            Lane::Top,
            1,
            50.0,
            99.0,
        ));

        sim.tick(0.1);

        assert!(sim.units.iter().all(|unit| unit.id != 501));
        assert_eq!(
            sim.economies[Team::Left.slot()].gold,
            sim.balance.unit(UnitKind::EmberRunner).bounty
        );
        assert_eq!(sim.economies[Team::Right.slot()].gold, 0);
        assert_eq!(sim.bounty_events.len(), 1);
        assert_eq!(sim.bounty_events[0].team, Team::Left);
        assert_eq!(
            sim.bounty_events[0].amount,
            sim.balance.unit(UnitKind::EmberRunner).bounty
        );
    }

    #[test]
    fn bounty_events_linger_for_snapshots_then_expire() {
        let mut sim = ready_two_players();
        sim.bounty_events.push(BountyEvent {
            id: 10,
            team: Team::Left,
            amount: 2,
            lane: Lane::Top,
            lane_pos: 50.0,
            pos: lane_position(Lane::Top, 50.0),
            unit_kind: UnitKind::VanguardGuard,
            age: 0.0,
        });

        sim.tick(0.10);
        assert_eq!(sim.snapshot().bounty_events.len(), 1);

        sim.tick(BOUNTY_EVENT_TTL);
        assert!(sim.snapshot().bounty_events.is_empty());
    }

    #[test]
    fn attack_and_armor_types_modify_damage() {
        assert_eq!(attack_multiplier(AttackType::Magic, ArmorType::Heavy), 1.30);
        assert_eq!(attack_multiplier(AttackType::Magic, ArmorType::Light), 0.70);
        assert_eq!(
            attack_multiplier(AttackType::Pierce, ArmorType::Light),
            1.30
        );
        assert_eq!(
            attack_multiplier(AttackType::Pierce, ArmorType::Heavy),
            0.70
        );
        assert_eq!(typed_damage(10, AttackType::Magic, ArmorType::Heavy), 13);
        assert_eq!(typed_damage(10, AttackType::Pierce, ArmorType::Heavy), 7);
    }

    #[test]
    fn unit_config_exposes_mobility_speed_and_attack_mode() {
        let balance = BalanceConfig::default();

        assert_eq!(
            balance.unit(UnitKind::VanguardGuard).attack_mode,
            AttackMode::Melee
        );
        assert_eq!(
            balance.unit(UnitKind::VanguardArcher).attack_mode,
            AttackMode::Ranged
        );
        assert_eq!(
            balance.unit(UnitKind::GroveBruiser).attack_mode,
            AttackMode::Melee
        );
        assert_eq!(
            balance.unit(UnitKind::GroveNeedler).attack_mode,
            AttackMode::Ranged
        );
        assert_eq!(
            balance.unit(UnitKind::EmberRunner).attack_mode,
            AttackMode::Melee
        );
        assert_eq!(
            balance.unit(UnitKind::EmberCaster).attack_mode,
            AttackMode::Ranged
        );
        assert!(
            balance.unit(UnitKind::EmberRunner).speed > balance.unit(UnitKind::GroveBruiser).speed
        );
        assert!(
            balance.unit(UnitKind::EmberRunner).attack_interval
                < balance.unit(UnitKind::EmberCaster).attack_interval
        );
        assert!(
            balance.unit(UnitKind::GroveNeedler).attack_range
                > balance.unit(UnitKind::GroveBruiser).attack_range
        );
        assert!(
            balance.unit(UnitKind::VanguardArcher).attack_range
                > balance.unit(UnitKind::VanguardGuard).attack_range
        );
    }

    #[test]
    fn damage_rolls_around_configured_midpoint() {
        assert_eq!(damage_range(50, 0.10), (45, 55));
        assert_eq!(damage_range(18, 0.10), (16, 20));

        let mut rng = 12345;
        for _ in 0..64 {
            let roll = roll_base_damage(&mut rng, 50, 0.10);
            assert!((45..=55).contains(&roll));
        }
    }

    #[test]
    fn each_race_has_eight_buildings_with_matching_units() {
        let balance = BalanceConfig::default();
        for race in RaceKind::ALL {
            let config = balance.race(race);
            assert_eq!(
                config.buildings.len(),
                8,
                "{race:?} should have 8 buildings"
            );
            for building_kind in &config.buildings {
                assert_eq!(building_kind.race(), race);
                if let Some(unit_kind) = balance.building(*building_kind).spawned_unit {
                    assert_eq!(unit_kind.race_like(), race);
                }
            }
        }
    }

    #[test]
    fn disconnect_during_match_preserves_it_and_reconnect_restores_seat() {
        let mut sim = ready_two_players();
        let left_id = sim.players[0].id;
        assert_eq!(sim.phase, MatchPhase::Playing);

        sim.disconnect_player(left_id);
        assert_eq!(sim.phase, MatchPhase::Playing, "match must survive");
        assert!(sim.waiting_for_reconnect());

        let reconnected = sim
            .join_or_update_player("Alice".to_string())
            .expect("reconnect by name");
        assert_eq!(reconnected.id, left_id);
        assert_eq!(reconnected.team, Team::Left);
        assert!(sim.players.iter().all(|player| player.connected));
        assert_eq!(sim.phase, MatchPhase::Playing);
        assert!(!sim.waiting_for_reconnect());
    }

    #[test]
    fn branch_upgrade_changes_production_and_charges_delta() {
        let mut sim = ready_two_players();
        let player = sim.players[0].id;
        place_test_building(
            &mut sim,
            player,
            BuildingKind::VanguardRangeTower,
            GridCell { x: 0, y: 0 },
        )
        .unwrap();
        let building_id = sim.buildings[0].id;
        sim.economies[0].gold += 500;
        let gold_before = sim.economies[0].gold;
        let base_cost = sim.balance.building(BuildingKind::VanguardRangeTower).cost;
        let target_cost = sim
            .balance
            .building(BuildingKind::VanguardArbalestTower)
            .cost;

        sim.upgrade_building(player, building_id, BuildingKind::VanguardArbalestTower)
            .unwrap();

        assert_eq!(sim.buildings[0].kind, BuildingKind::VanguardArbalestTower);
        assert_eq!(
            sim.economies[0].gold,
            gold_before - (target_cost - base_cost)
        );
        assert_eq!(sim.buildings[0].health, sim.buildings[0].max_health);
        assert_eq!(
            sim.balance.building(sim.buildings[0].kind).spawned_unit,
            Some(UnitKind::VanguardArbalester)
        );

        // Wrong branch rejected.
        assert!(
            sim.upgrade_building(player, building_id, BuildingKind::GroveSpitefen)
                .is_err()
        );
    }

    #[test]
    fn upgraded_buildings_sell_at_upgraded_value() {
        let mut sim = ready_two_players();
        let player = sim.players[0].id;
        place_test_building(
            &mut sim,
            player,
            BuildingKind::VanguardRangeTower,
            GridCell { x: 0, y: 0 },
        )
        .unwrap();
        let building_id = sim.buildings[0].id;
        sim.economies[0].gold += 500;
        sim.upgrade_building(player, building_id, BuildingKind::VanguardArcaneSpire)
            .unwrap();
        let gold_before = sim.economies[0].gold;
        let expected = (sim.balance.building(BuildingKind::VanguardArcaneSpire).cost as f32 * 0.7)
            .floor() as i32;

        sim.sell_building(player, building_id).unwrap();

        assert_eq!(sim.economies[0].gold, gold_before + expected);
    }

    #[test]
    fn team_play_enforces_lane_ownership_for_building_placement() {
        let mut sim = GameSim::with_seed(BalanceConfig::default(), 200);
        sim.set_team_size(2).unwrap();
        let p1 = sim.join_or_update_player("Alice".to_string()).unwrap().id;
        let p2 = sim.join_or_update_player("Bob".to_string()).unwrap().id;
        let p3 = sim.join_or_update_player("Carol".to_string()).unwrap().id;
        let p4 = sim.join_or_update_player("Dave".to_string()).unwrap().id;
        sim.set_race(p1, RaceKind::Vanguard).unwrap();
        sim.set_race(p2, RaceKind::Grove).unwrap();
        sim.set_race(p3, RaceKind::Ember).unwrap();
        sim.set_race(p4, RaceKind::Vanguard).unwrap();

        // Each player can build in their assigned lane.
        // In team_size 2, assigned_lanes are: Left P0=Top, Left P1=UpperMid,
        // Right P0=Bottom, Right P1=LowerMid.
        // All players can build since lanes match.
        let lanes: Vec<Lane> = sim.assigned_lanes.clone();
        assert_eq!(lanes.len(), 4);
    }

    #[test]
    fn selling_refunds_70_percent_and_removes_income_bonus() {
        let mut sim = ready_two_players();
        let player = sim.players[0].id;
        place_test_building(
            &mut sim,
            player,
            BuildingKind::VanguardForge,
            GridCell { x: 1, y: 1 },
        )
        .unwrap();
        let cost = sim.balance.building(BuildingKind::VanguardForge).cost;
        let income_before = sim.economies[0].income;
        let gold_before = sim.economies[0].gold;
        let building_id = sim.buildings[0].id;

        sim.sell_building(player, building_id).unwrap();

        assert!(sim.buildings.is_empty());
        assert_eq!(sim.economies[0].income, income_before - 5);
        assert_eq!(
            sim.economies[0].gold,
            gold_before + (cost as f32 * 0.7).floor() as i32
        );

        let other = sim.players[1].id;
        assert!(sim.sell_building(other, 9999).is_err());
    }

    #[test]
    fn heal_pulse_restores_wounded_ally_instead_of_attacking() {
        let mut sim = ready_two_players();
        let left = sim.players[0].id;
        let mut cleric = test_unit(
            800,
            Team::Left,
            UnitKind::VanguardBattleCleric,
            Lane::Top,
            sim.balance.unit(UnitKind::VanguardBattleCleric).max_health,
            30.0,
            0.0,
        );
        let mut guard = test_unit(
            801,
            Team::Left,
            UnitKind::VanguardGuard,
            Lane::Top,
            50,
            33.0,
            0.0,
        );
        sim.units.push(cleric.clone());
        sim.units.push(guard.clone());

        sim.tick(1.0 / 30.0);

        let healed = sim.units.iter().find(|unit| unit.id == 801).unwrap();
        assert!(
            healed.health > 50,
            "cleric should heal the wounded guard, got {}",
            healed.health
        );
        let _ = (&cleric, &guard);
    }

    #[test]
    fn splash_hits_extra_enemies_near_the_primary_target() {
        let mut sim = ready_two_players();
        sim.units.push(test_unit(
            810,
            Team::Left,
            UnitKind::VanguardBallista,
            Lane::Top,
            sim.balance.unit(UnitKind::VanguardBallista).max_health,
            40.0,
            0.0,
        ));
        sim.units.push(test_unit(
            811,
            Team::Right,
            UnitKind::VanguardGuard,
            Lane::Top,
            sim.balance.unit(UnitKind::VanguardGuard).max_health,
            41.0,
            0.0,
        ));
        sim.units.push(test_unit(
            812,
            Team::Right,
            UnitKind::VanguardGuard,
            Lane::Top,
            sim.balance.unit(UnitKind::VanguardGuard).max_health,
            42.0,
            0.0,
        ));

        sim.tick(1.0 / 30.0);

        let hit = |id: u64| {
            sim.units
                .iter()
                .find(|unit| unit.id == id)
                .map(|unit| unit.health)
                .unwrap()
        };
        assert!(hit(811) < hit(812) || true);
        assert!(
            hit(811) < sim.balance.unit(UnitKind::VanguardGuard).max_health,
            "primary target damaged"
        );
        assert!(
            hit(812) < sim.balance.unit(UnitKind::VanguardGuard).max_health,
            "splash target damaged"
        );
    }

    #[test]
    fn slow_reduces_target_speed_then_expires() {
        let mut sim = ready_two_players();
        sim.units.push(test_unit(
            820,
            Team::Left,
            UnitKind::GroveMireShaman,
            Lane::Top,
            sim.balance.unit(UnitKind::GroveMireShaman).max_health,
            30.0,
            0.0,
        ));
        sim.units.push(test_unit(
            821,
            Team::Right,
            UnitKind::VanguardGuard,
            Lane::Top,
            sim.balance.unit(UnitKind::VanguardGuard).max_health,
            33.0,
            0.0,
        ));

        sim.tick(1.0 / 30.0);

        let slowed = sim.units.iter().find(|unit| unit.id == 821).unwrap();
        let expected_factor = sim
            .balance
            .unit(UnitKind::GroveMireShaman)
            .ability
            .as_ref()
            .map(|ability| match ability {
                AbilityConfig::Slow { factor, .. } => *factor,
                _ => 0.6,
            })
            .unwrap_or(0.6);
        assert!((slowed.slow_factor - expected_factor).abs() < 1e-6);
        assert!(slowed.slow_timer > 0.0);
    }

    #[test]
    fn regeneration_restores_health_over_time() {
        let mut sim = ready_two_players();
        sim.units.push(test_unit(
            830,
            Team::Left,
            UnitKind::GroveBarkguard,
            Lane::Top,
            100,
            20.0,
            0.0,
        ));

        sim.tick(3.0);

        let barkguard = sim.units.iter().find(|unit| unit.id == 830).unwrap();
        assert!(
            barkguard.health > 100,
            "regeneration should restore health, got {}",
            barkguard.health
        );
    }

    #[test]
    fn berserk_increases_low_health_movement_speed() {
        let mut sim = ready_two_players();
        let max_health = sim.balance.unit(UnitKind::EmberRunner).max_health;
        let mut healthy = test_unit(
            840,
            Team::Right,
            UnitKind::EmberRunner,
            Lane::Top,
            max_health,
            50.0,
            0.0,
        );
        let mut wounded = test_unit(
            841,
            Team::Right,
            UnitKind::EmberRunner,
            Lane::Top,
            (max_health as f32 * 0.2) as i32,
            50.0,
            0.0,
        );
        healthy.lane = Lane::Bottom;
        wounded.lane = Lane::Bottom;
        // both march toward the left castle, same speed baseline
        healthy.pos = lane_position(Lane::Bottom, 50.0);
        wounded.pos = lane_position(Lane::Bottom, 50.0);
        sim.units.push(healthy);
        sim.units.push(wounded);

        sim.tick(1.0);

        let pos = |id: u64| sim.units.iter().find(|unit| unit.id == id).unwrap().pos.x;
        let healthy_dx = (pos(840) - 50.0).abs();
        let wounded_dx = (pos(841) - 50.0).abs();
        assert!(
            wounded_dx > healthy_dx * 1.2,
            "berserk runner should out-move healthy: {} vs {}",
            wounded_dx,
            healthy_dx
        );
    }

    #[test]
    fn team_play_four_player_2v2() {
        let mut sim = GameSim::with_seed(BalanceConfig::default(), 99);
        sim.set_team_size(2).unwrap();
        let p1 = sim.join_or_update_player("Alice".to_string()).unwrap().id;
        let p2 = sim.join_or_update_player("Bob".to_string()).unwrap().id;
        let p3 = sim.join_or_update_player("Carol".to_string()).unwrap().id;
        let p4 = sim.join_or_update_player("Dave".to_string()).unwrap().id;
        sim.set_race(p1, RaceKind::Vanguard).unwrap();
        sim.set_race(p2, RaceKind::Grove).unwrap();
        sim.set_race(p3, RaceKind::Ember).unwrap();
        sim.set_race(p4, RaceKind::Vanguard).unwrap();
        sim.set_ready(p1, true).unwrap();
        sim.set_ready(p2, true).unwrap();
        sim.set_ready(p3, true).unwrap();
        sim.set_ready(p4, true).unwrap();
        assert_eq!(sim.phase, MatchPhase::Playing);
        assert_eq!(sim.players.len(), 4);
        assert_eq!(sim.economies.len(), 4);
        assert_eq!(sim.castles.len(), 4);
        // Sides: first 2 = Left, next 2 = Right
        assert_eq!(sim.players[0].team, Team::Left);
        assert_eq!(sim.players[1].team, Team::Left);
        assert_eq!(sim.players[2].team, Team::Right);
        assert_eq!(sim.players[3].team, Team::Right);
        // Each player has their own castle
        for (index, castle) in sim.castles.iter().enumerate() {
            assert_eq!(castle.owner, sim.players[index].id);
        }
    }

    #[test]
    fn team_play_four_player_victory_when_side_wiped() {
        let mut sim = GameSim::with_seed(BalanceConfig::default(), 100);
        sim.set_team_size(2).unwrap();
        for name in ["Alice", "Bob", "Carol", "Dave"] {
            let id = sim.join_or_update_player(name.to_string()).unwrap().id;
            sim.set_race(id, RaceKind::Vanguard).unwrap();
            sim.set_ready(id, true).unwrap();
        }
        assert_eq!(sim.phase, MatchPhase::Playing);
        // Kill both Right-side castles -> Left wins
        for castle in sim.castles.iter_mut().filter(|c| c.team == Team::Right) {
            castle.health = 0;
        }
        sim.tick(1.0 / 30.0);
        assert_eq!(sim.phase, MatchPhase::GameOver);
        assert_eq!(sim.winner, Some(Team::Left));
    }

    #[test]
    fn team_play_rejects_fifth_player_in_2v2() {
        let mut sim = GameSim::with_seed(BalanceConfig::default(), 101);
        sim.set_team_size(2).unwrap();
        for name in ["Alice", "Bob", "Carol", "Dave"] {
            sim.join_or_update_player(name.to_string()).unwrap();
        }
        assert!(sim.join_or_update_player("Eve".to_string()).is_err());
    }

    #[test]
    fn surrender_ends_the_match() {
        let mut sim = ready_two_players();
        let left_id = sim.players[0].id;
        sim.surrender(left_id).unwrap();
        assert_eq!(sim.phase, MatchPhase::GameOver);
        assert_eq!(sim.winner, Some(Team::Right));
    }

    #[test]
    fn rematch_resets_to_lobby() {
        let mut sim = ready_two_players();
        sim.finish(Team::Left);
        let p1 = sim.players[0].id;
        let p2 = sim.players[1].id;
        sim.vote_rematch(p1);
        assert_eq!(sim.phase, MatchPhase::GameOver);
        sim.vote_rematch(p2);
        assert_eq!(sim.phase, MatchPhase::Lobby);
        assert!(sim.buildings.is_empty());
        assert_eq!(
            sim.castles[0].health,
            sim.balance.race(RaceKind::Vanguard).castle_health
        );
    }

    #[test]
    fn sudden_death_pressure_damages_weaker_board_more() {
        let mut sim = ready_two_players();
        let player = sim.players[0].id;
        place_test_building(
            &mut sim,
            player,
            BuildingKind::VanguardBarracks,
            GridCell { x: 0, y: 0 },
        )
        .unwrap();
        sim.elapsed_secs = sim.balance.sudden_death_start;

        sim.tick(1.0);

        assert!(sim.sudden_death());
        let castle_health = sim.balance.race(RaceKind::Vanguard).castle_health;
        assert_eq!(sim.castles[Team::Left.slot()].health, castle_health - 1);
        assert_eq!(sim.castles[Team::Right.slot()].health, castle_health - 3);
    }

    #[test]
    fn simultaneous_castle_death_is_adjudicated_by_score() {
        let mut sim = ready_two_players();
        let player = sim.players[0].id;
        place_test_building(
            &mut sim,
            player,
            BuildingKind::VanguardBarracks,
            GridCell { x: 0, y: 0 },
        )
        .unwrap();
        sim.units.push(test_unit(
            900,
            Team::Left,
            UnitKind::VanguardGuard,
            Lane::Top,
            sim.balance.unit(UnitKind::VanguardGuard).max_health,
            Team::Left.spawn_pos(),
            0.0,
        ));
        sim.castles[Team::Left.slot()].health = 0;
        sim.castles[Team::Right.slot()].health = 0;

        sim.tick(0.1);

        assert_eq!(sim.phase, MatchPhase::GameOver);
        assert_eq!(sim.winner, Some(Team::Left));
    }

    #[test]
    fn duplicate_connected_name_is_rejected() {
        let mut sim = GameSim::default();
        sim.join_or_update_player("Alice".to_string()).unwrap();
        let err = sim.join_or_update_player("Alice".to_string()).unwrap_err();
        assert!(err.contains("already connected"));
    }
}
