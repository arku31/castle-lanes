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
pub const SUDDEN_DEATH_RAMP_PER_MINUTE: f32 = 0.5;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatchPhase {
    Lobby,
    Playing,
    GameOver,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Lane {
    Top,
    Bottom,
}

impl Lane {
    pub const ALL: [Lane; 2] = [Lane::Top, Lane::Bottom];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BuildZone {
    Front,
    Back,
}

impl BuildZone {
    pub const ALL: [BuildZone; 2] = [BuildZone::Front, BuildZone::Back];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
        Lane::Top => LANE_CENTER_Y,
        Lane::Bottom => -LANE_CENTER_Y,
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
            | UnitKind::VanguardBallista => RaceKind::Vanguard,
            UnitKind::GroveBruiser
            | UnitKind::GroveNeedler
            | UnitKind::GroveSproutling
            | UnitKind::GroveBarkguard
            | UnitKind::GroveMireShaman
            | UnitKind::GroveVineStalker
            | UnitKind::GroveTreantColossus => RaceKind::Grove,
            UnitKind::EmberRunner
            | UnitKind::EmberCaster
            | UnitKind::EmberSparkImp
            | UnitKind::EmberObsidianGuard
            | UnitKind::EmberFireLancer
            | UnitKind::EmberSmokeWitch
            | UnitKind::EmberCinderEngine => RaceKind::Ember,
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
    pub team: Team,
    pub health: i32,
    pub max_health: i32,
    pub armor_type: ArmorType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Building {
    pub id: u64,
    pub owner: Team,
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
    pub owner: Team,
    pub kind: UnitKind,
    pub lane: Lane,
    pub health: i32,
    pub lane_pos: f32,
    pub pos: WorldPos,
    pub velocity: WorldPos,
    pub radius: f32,
    pub attack_timer: f32,
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
    pub economies: [Economy; 2],
    pub castles: [Castle; 2],
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
    pub economies: [Economy; 2],
    pub castles: [Castle; 2],
    pub buildings: Vec<Building>,
    pub units: Vec<Unit>,
    pub bounty_events: Vec<BountyEvent>,
    pub winner: Option<Team>,
    pub message: String,
    pub balance: BalanceConfig,
    next_id: u64,
    seed: u64,
    rng_state: u64,
    income_timer: f32,
    elapsed_secs: f32,
    castle_regen_accum: [f32; 2],
    castle_regen_delay_timer: [f32; 2],
    overtime_damage_accum: [f32; 2],
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
        let starting_economy = Economy {
            gold: balance.starting_gold,
            income: balance.base_income,
        };
        let default_health = balance.race(RaceKind::Vanguard).castle_health;
        let income_timer = balance.income_interval;
        Self {
            phase: MatchPhase::Lobby,
            tick: 0,
            players: Vec::new(),
            economies: [starting_economy.clone(), starting_economy],
            castles: [
                Castle {
                    team: Team::Left,
                    health: default_health,
                    max_health: default_health,
                    armor_type: default_castle_armor(),
                },
                Castle {
                    team: Team::Right,
                    health: default_health,
                    max_health: default_health,
                    armor_type: default_castle_armor(),
                },
            ],
            buildings: Vec::new(),
            units: Vec::new(),
            bounty_events: Vec::new(),
            winner: None,
            message: "Waiting for two players.".to_string(),
            balance,
            next_id: 1,
            seed,
            rng_state: mix_seed(seed),
            income_timer,
            elapsed_secs: 0.0,
            castle_regen_accum: [0.0, 0.0],
            castle_regen_delay_timer: [0.0, 0.0],
            overtime_damage_accum: [0.0, 0.0],
        }
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

        if self.players.len() >= 2 {
            return Err("Match is full.".to_string());
        }

        let team = if self.players.is_empty() {
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
        };
        self.message = format!("{} joined as {:?}.", player.name, player.team);
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

    pub fn disconnect_player(&mut self, player_id: PlayerId) {
        if let Some(player) = self.players.iter_mut().find(|p| p.id == player_id) {
            player.connected = false;
            player.ready = false;
            self.message = format!("{} disconnected.", player.name);
            if self.phase == MatchPhase::Playing {
                self.phase = MatchPhase::Lobby;
                self.clear_match_entities();
            }
        }
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
        if self
            .buildings
            .iter()
            .any(|b| b.owner == team && b.lane == lane && b.zone == zone && b.cell == cell)
        {
            return Err("That cell is already occupied.".to_string());
        }
        let building_name = self.balance.building(kind).name.clone();
        let building_cost = self.balance.building(kind).cost;
        let building_health = self.balance.building(kind).max_health;
        let income_bonus = self.balance.building(kind).income_bonus;
        let spawn_timer = self.balance.building(kind).spawn_interval.unwrap_or(0.0);
        let economy = &mut self.economies[team.slot()];
        if economy.gold < building_cost {
            return Err(format!("Not enough gold for {}.", building_name));
        }
        economy.gold -= building_cost;
        economy.income += income_bonus;
        let id = self.take_id();
        self.buildings.push(Building {
            id,
            owner: team,
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
        if self.players.len() == 2
            && self
                .players
                .iter()
                .all(|p| p.connected && p.ready && p.race.is_some())
        {
            self.reset_match_state();
            self.phase = MatchPhase::Playing;
            self.message = "Match started.".to_string();
        }
    }

    fn reset_to_lobby(&mut self) {
        self.phase = MatchPhase::Lobby;
        self.reset_match_state();
        for player in &mut self.players {
            player.ready = false;
            player.race = None;
            player.rematch_vote = false;
        }
        self.message = "Rematch ready. Set ready again.".to_string();
    }

    fn reset_match_state(&mut self) {
        self.clear_match_entities();
        let starting_economy = Economy {
            gold: self.balance.starting_gold,
            income: self.balance.base_income,
        };
        self.economies = [starting_economy.clone(), starting_economy];
        let left_health = self
            .players
            .iter()
            .find(|p| p.team == Team::Left)
            .and_then(|p| p.race)
            .map(|race| self.balance.race(race).castle_health)
            .unwrap_or_else(|| self.balance.race(RaceKind::Vanguard).castle_health);
        let right_health = self
            .players
            .iter()
            .find(|p| p.team == Team::Right)
            .and_then(|p| p.race)
            .map(|race| self.balance.race(race).castle_health)
            .unwrap_or_else(|| self.balance.race(RaceKind::Vanguard).castle_health);
        let left_armor = self
            .players
            .iter()
            .find(|p| p.team == Team::Left)
            .and_then(|p| p.race)
            .map(|race| self.balance.race(race).castle_armor)
            .unwrap_or_else(default_castle_armor);
        let right_armor = self
            .players
            .iter()
            .find(|p| p.team == Team::Right)
            .and_then(|p| p.race)
            .map(|race| self.balance.race(race).castle_armor)
            .unwrap_or_else(default_castle_armor);
        self.castles = [
            Castle {
                team: Team::Left,
                health: left_health,
                max_health: left_health,
                armor_type: left_armor,
            },
            Castle {
                team: Team::Right,
                health: right_health,
                max_health: right_health,
                armor_type: right_armor,
            },
        ];
        self.winner = None;
        self.income_timer = self.balance.income_interval;
        self.elapsed_secs = 0.0;
        self.rng_state = mix_seed(self.seed);
        self.castle_regen_accum = [0.0, 0.0];
        self.castle_regen_delay_timer = [0.0, 0.0];
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
                    spawns.push((
                        building.owner,
                        building.lane,
                        building.zone,
                        building.cell,
                        kind,
                    ));
                }
            }
        }

        for (owner, lane, zone, cell, kind) in spawns {
            let unit_config = self.balance.unit(kind);
            let max_health = unit_config.max_health;
            let radius = unit_config.radius;
            let pos = self.free_spawn_position(owner, lane, zone, cell, radius);
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
            });
        }
    }

    fn tick_units(&mut self, dt: f32) {
        let mut rng_state = self.rng_state;
        // Ordered by the units Vec (spawn order), never a hash map: equal-distance
        // target ties below must break by id so the sim stays deterministic.
        let positions: Vec<(u64, Team, Lane, f32, WorldPos, i32, ArmorType)> = self
            .units
            .iter()
            .map(|u| {
                (
                    u.id,
                    u.owner,
                    u.lane,
                    u.lane_pos,
                    u.pos,
                    u.health,
                    self.balance.unit(u.kind).armor_type,
                )
            })
            .collect();
        let mut unit_damage: HashMap<u64, (i32, Team)> = HashMap::new();
        let mut building_damage: HashMap<u64, i32> = HashMap::new();
        let mut castle_damage = [0, 0];

        for unit in &mut self.units {
            let unit_config = self.balance.unit(unit.kind);
            unit.attack_timer = (unit.attack_timer - dt).max(0.0);
            let unit_target = positions
                .iter()
                .filter(|(_, team, lane, lane_pos, _, health, _)| {
                    *team != unit.owner
                        && *health > 0
                        && unit_lanes_connected(unit.lane, unit.lane_pos, *lane, *lane_pos)
                })
                .map(|(id, _, lane, lane_pos, pos, _, armor)| {
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
                    building.owner != unit.owner
                        && building.lane == unit.lane
                        && building.zone == BuildZone::Front
                        && building.health > 0
                })
                .map(|building| {
                    let pos = building_position(
                        building.owner,
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
                .min_by(|a, b| a.1.total_cmp(&b.1));

            let enemy_castle_pos = lane_position(unit.lane, unit.owner.opponent().castle_pos());
            let enemy_castle_distance =
                footprint_distance(unit.pos, enemy_castle_pos, CASTLE_FOOTPRINT_RADIUS);
            let can_attack_castle = enemy_castle_distance <= unit_config.attack_range;

            if unit.attack_timer <= 0.0 {
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
                    unit.attack_timer = unit_config.attack_interval;
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
                    unit.attack_timer = unit_config.attack_interval;
                    continue;
                }
                if can_attack_castle {
                    let target_slot = unit.owner.opponent().slot();
                    let rolled_damage = roll_base_damage(
                        &mut rng_state,
                        unit_config.damage,
                        unit_config.damage_variance,
                    );
                    castle_damage[target_slot] += typed_damage(
                        rolled_damage,
                        unit_config.attack_type,
                        self.castles[target_slot].armor_type,
                    );
                    unit.attack_timer = unit_config.attack_interval;
                    continue;
                }
            }

            if unit_target.is_none() && building_target.is_none() && !can_attack_castle {
                let old_pos = unit.pos;
                let max_step = unit_config.speed * dt;
                unit.pos.x += unit.owner.direction() * max_step;
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
        self.separate_units();
        self.rng_state = rng_state;

        let mut bounty_awards = [0, 0];
        let mut pending_bounty_events = Vec::new();
        for unit in &mut self.units {
            if let Some((damage, killer)) = unit_damage.get(&unit.id) {
                unit.health -= *damage;
                if unit.health <= 0 {
                    let bounty = self.balance.unit(unit.kind).bounty.max(0);
                    bounty_awards[killer.slot()] += bounty;
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
        let mut destroyed_income = [0, 0];
        for building in &mut self.buildings {
            if let Some(damage) = building_damage.get(&building.id) {
                building.health -= *damage;
                if building.health <= 0 {
                    destroyed_income[building.owner.slot()] +=
                        self.balance.building(building.kind).income_bonus;
                }
            }
        }
        self.buildings.retain(|building| building.health > 0);
        for (idx, lost_income) in destroyed_income.into_iter().enumerate() {
            if lost_income > 0 {
                self.economies[idx].income -= lost_income;
            }
        }
        for (idx, bounty) in bounty_awards.into_iter().enumerate() {
            self.economies[idx].gold += bounty;
        }
        for (team, amount, lane, lane_pos, pos, unit_kind) in pending_bounty_events {
            let id = self.take_id();
            self.bounty_events.push(BountyEvent {
                id,
                team,
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

    fn apply_castle_regen_and_damage(&mut self, dt: f32, castle_damage: [i32; 2]) {
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

        // Pressure ramps up the longer sudden death runs so stalemates still
        // resolve instead of both castles grinding down at a fixed 40 dps.
        let ramp = self.balance.sudden_death_ramp_per_minute.max(0.0);
        let minutes_into_sudden_death =
            ((self.elapsed_secs - self.balance.sudden_death_start) / 60.0).max(0.0);
        let pressure_scale = 1.0 + ramp * minutes_into_sudden_death;

        for team in [Team::Left, Team::Right] {
            let pressure = self.board_pressure(team.opponent());
            let slot = team.slot();
            self.overtime_damage_accum[slot] += pressure * pressure_scale * dt;
            let damage = self.overtime_damage_accum[slot].floor() as i32;
            if damage > 0 {
                self.overtime_damage_accum[slot] -= damage as f32;
                self.castles[slot].health = (self.castles[slot].health - damage).max(0);
            }
        }

        if self.tick % 30 == 0 {
            self.message = "Sudden death: castles take pressure damage.".to_string();
        }
    }

    fn board_pressure(&self, team: Team) -> f32 {
        let buildings = self
            .buildings
            .iter()
            .filter(|building| building.owner == team)
            .count() as f32;
        let units = self.units.iter().filter(|unit| unit.owner == team).count() as f32;
        1.0 + buildings * 2.0 + units
    }

    fn check_victory(&mut self) {
        let left_dead = self.castles[Team::Left.slot()].health <= 0;
        let right_dead = self.castles[Team::Right.slot()].health <= 0;
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
        let castle = self.castles[team.slot()].health.max(0) * 10;
        let economy = self.economies[team.slot()].gold + self.economies[team.slot()].income * 5;
        let buildings = self
            .buildings
            .iter()
            .filter(|building| building.owner == team)
            .map(|building| self.balance.building(building.kind).cost)
            .sum::<i32>();
        let units = self
            .units
            .iter()
            .filter(|unit| unit.owner == team)
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
            owner,
            kind,
            lane,
            health,
            lane_pos,
            pos: lane_position(lane, lane_pos),
            velocity: WorldPos::new(0.0, 0.0),
            radius: BalanceConfig::default().unit(kind).radius,
            attack_timer,
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
