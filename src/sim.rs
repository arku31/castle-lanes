use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub const CASTLE_HEALTH: i32 = 1000;
pub const STARTING_GOLD: i32 = 150;
pub const BASE_INCOME: i32 = 10;
pub const INCOME_INTERVAL: f32 = 5.0;
pub const GRID_W: i32 = 4;
pub const GRID_H: i32 = 3;
pub const LANE_LENGTH: f32 = 100.0;
pub const SUDDEN_DEATH_START: f32 = 180.0;
pub const DEFAULT_BALANCE_PATH: &str = "config/balance.json";

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
    GroveRootDen,
    GroveThornSpire,
    GroveBloomWell,
    EmberCinderPit,
    EmberFlameSpire,
    EmberAshMine,
}

impl BuildingKind {
    pub fn race(self) -> RaceKind {
        match self {
            BuildingKind::VanguardBarracks
            | BuildingKind::VanguardRangeTower
            | BuildingKind::VanguardForge => RaceKind::Vanguard,
            BuildingKind::GroveRootDen
            | BuildingKind::GroveThornSpire
            | BuildingKind::GroveBloomWell => RaceKind::Grove,
            BuildingKind::EmberCinderPit
            | BuildingKind::EmberFlameSpire
            | BuildingKind::EmberAshMine => RaceKind::Ember,
        }
    }

    pub fn fallback_name(self) -> &'static str {
        match self {
            BuildingKind::VanguardBarracks => "Barracks",
            BuildingKind::VanguardRangeTower => "Range Tower",
            BuildingKind::VanguardForge => "Forge",
            BuildingKind::GroveRootDen => "Root Den",
            BuildingKind::GroveThornSpire => "Thorn Spire",
            BuildingKind::GroveBloomWell => "Bloom Well",
            BuildingKind::EmberCinderPit => "Cinder Pit",
            BuildingKind::EmberFlameSpire => "Flame Spire",
            BuildingKind::EmberAshMine => "Ash Mine",
        }
    }

    pub fn from_cli(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "barracks" | "vanguard-barracks" | "b" => Some(BuildingKind::VanguardBarracks),
            "range" | "range-tower" | "rangetower" | "tower" | "r" => {
                Some(BuildingKind::VanguardRangeTower)
            }
            "forge" | "f" => Some(BuildingKind::VanguardForge),
            "root" | "root-den" | "rootden" | "g1" => Some(BuildingKind::GroveRootDen),
            "thorn" | "thorn-spire" | "thornspire" | "g2" => Some(BuildingKind::GroveThornSpire),
            "bloom" | "bloom-well" | "bloomwell" | "g3" => Some(BuildingKind::GroveBloomWell),
            "cinder" | "cinder-pit" | "cinderpit" | "e1" => Some(BuildingKind::EmberCinderPit),
            "flame" | "flame-spire" | "flamespire" | "e2" => Some(BuildingKind::EmberFlameSpire),
            "ash" | "ash-mine" | "ashmine" | "e3" => Some(BuildingKind::EmberAshMine),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnitKind {
    VanguardGuard,
    VanguardArcher,
    GroveBruiser,
    GroveNeedler,
    EmberRunner,
    EmberCaster,
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

impl UnitKind {
    pub fn race_like(self) -> RaceKind {
        match self {
            UnitKind::VanguardGuard | UnitKind::VanguardArcher => RaceKind::Vanguard,
            UnitKind::GroveBruiser | UnitKind::GroveNeedler => RaceKind::Grove,
            UnitKind::EmberRunner | UnitKind::EmberCaster => RaceKind::Ember,
        }
    }

    pub fn fallback_name(self) -> &'static str {
        match self {
            UnitKind::VanguardGuard => "Guard",
            UnitKind::VanguardArcher => "Archer",
            UnitKind::GroveBruiser => "Bruiser",
            UnitKind::GroveNeedler => "Needler",
            UnitKind::EmberRunner => "Runner",
            UnitKind::EmberCaster => "Caster",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceConfig {
    pub starting_gold: i32,
    pub base_income: i32,
    pub income_interval: f32,
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
    pub buildings: [BuildingKind; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildingConfig {
    pub kind: BuildingKind,
    pub name: String,
    pub label: String,
    pub cost: i32,
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
    #[serde(default = "default_attack_type")]
    pub attack_type: AttackType,
    #[serde(default = "default_attack_mode")]
    pub attack_mode: AttackMode,
    #[serde(default = "default_armor_type")]
    pub armor_type: ArmorType,
    #[serde(alias = "range")]
    pub attack_range: f32,
    pub speed: f32,
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

fn default_armor_type() -> ArmorType {
    ArmorType::Normal
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GridCell {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerInfo {
    pub id: PlayerId,
    pub name: String,
    pub team: Team,
    pub race: Option<RaceKind>,
    pub ready: bool,
    pub connected: bool,
    pub rematch_vote: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Castle {
    pub team: Team,
    pub health: i32,
    pub max_health: i32,
    pub armor_type: ArmorType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Building {
    pub id: u64,
    pub owner: Team,
    pub kind: BuildingKind,
    pub cell: GridCell,
    pub spawn_timer: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Unit {
    pub id: u64,
    pub owner: Team,
    pub kind: UnitKind,
    pub health: i32,
    pub lane_pos: f32,
    pub attack_timer: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchSnapshot {
    pub phase: MatchPhase,
    pub tick: u64,
    pub elapsed_secs: f32,
    pub sudden_death: bool,
    pub balance: BalanceConfig,
    pub players: Vec<PlayerInfo>,
    pub economies: [Economy; 2],
    pub castles: [Castle; 2],
    pub buildings: Vec<Building>,
    pub units: Vec<Unit>,
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
    pub winner: Option<Team>,
    pub message: String,
    pub balance: BalanceConfig,
    next_id: u64,
    rng_state: u64,
    income_timer: f32,
    elapsed_secs: f32,
    overtime_damage_accum: [f32; 2],
}

impl Default for GameSim {
    fn default() -> Self {
        Self::new(BalanceConfig::default())
    }
}

impl GameSim {
    pub fn new(balance: BalanceConfig) -> Self {
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
            winner: None,
            message: "Waiting for two players.".to_string(),
            balance,
            next_id: 1,
            rng_state: 0xC057_1A4E_5EED,
            income_timer,
            elapsed_secs: 0.0,
            overtime_damage_accum: [0.0, 0.0],
        }
    }

    pub fn snapshot(&self) -> MatchSnapshot {
        MatchSnapshot {
            phase: self.phase,
            tick: self.tick,
            elapsed_secs: self.elapsed_secs,
            sudden_death: self.sudden_death(),
            balance: self.balance.clone(),
            players: self.players.clone(),
            economies: self.economies.clone(),
            castles: self.castles.clone(),
            buildings: self.buildings.clone(),
            units: self.units.clone(),
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
            .any(|b| b.owner == team && b.cell == cell)
        {
            return Err("That cell is already occupied.".to_string());
        }
        let building_name = self.balance.building(kind).name.clone();
        let building_cost = self.balance.building(kind).cost;
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
            cell,
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
        self.overtime_damage_accum = [0.0, 0.0];
    }

    fn clear_match_entities(&mut self) {
        self.buildings.clear();
        self.units.clear();
    }

    fn tick_income(&mut self, dt: f32) {
        self.income_timer -= dt;
        if self.income_timer > 0.0 {
            return;
        }
        self.income_timer += self.balance.income_interval;
        for economy in &mut self.economies {
            economy.gold += economy.income;
        }
    }

    fn tick_buildings(&mut self, dt: f32) {
        let mut spawns = Vec::new();
        for building in &mut self.buildings {
            let building_config = self.balance.building(building.kind);
            let Some(interval) = building_config.spawn_interval else {
                continue;
            };
            building.spawn_timer -= dt;
            if building.spawn_timer <= 0.0 {
                building.spawn_timer += interval;
                if let Some(kind) = building_config.spawned_unit {
                    spawns.push((building.owner, kind));
                }
            }
        }

        for (owner, kind) in spawns {
            let max_health = self.balance.unit(kind).max_health;
            let id = self.take_id();
            self.units.push(Unit {
                id,
                owner,
                kind,
                health: max_health,
                lane_pos: owner.spawn_pos(),
                attack_timer: 0.25,
            });
        }
    }

    fn tick_units(&mut self, dt: f32) {
        let mut rng_state = self.rng_state;
        let positions: HashMap<u64, (Team, f32, i32, ArmorType)> = self
            .units
            .iter()
            .map(|u| {
                (
                    u.id,
                    (
                        u.owner,
                        u.lane_pos,
                        u.health,
                        self.balance.unit(u.kind).armor_type,
                    ),
                )
            })
            .collect();
        let mut unit_damage: HashMap<u64, i32> = HashMap::new();
        let mut castle_damage = [0, 0];

        for unit in &mut self.units {
            let unit_config = self.balance.unit(unit.kind);
            unit.attack_timer = (unit.attack_timer - dt).max(0.0);
            let target = positions
                .iter()
                .filter(|(_, (team, _, health, _))| *team != unit.owner && *health > 0)
                .map(|(id, (_, pos, _, armor))| (*id, (unit.lane_pos - *pos).abs(), *armor))
                .filter(|(_, distance, _)| *distance <= unit_config.attack_range)
                .min_by(|a, b| a.1.total_cmp(&b.1));

            let enemy_castle_distance = (unit.lane_pos - unit.owner.opponent().castle_pos()).abs();
            let can_attack_castle = enemy_castle_distance <= unit_config.attack_range;

            if unit.attack_timer <= 0.0 {
                if let Some((target_id, _, target_armor)) = target {
                    let rolled_damage = roll_base_damage(
                        &mut rng_state,
                        unit_config.damage,
                        unit_config.damage_variance,
                    );
                    let damage = typed_damage(rolled_damage, unit_config.attack_type, target_armor);
                    *unit_damage.entry(target_id).or_insert(0) += damage;
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

            if target.is_none() && !can_attack_castle {
                unit.lane_pos += unit.owner.direction() * unit_config.speed * dt;
                unit.lane_pos = unit.lane_pos.clamp(0.0, LANE_LENGTH);
            }
        }
        self.rng_state = rng_state;

        for unit in &mut self.units {
            if let Some(damage) = unit_damage.get(&unit.id) {
                unit.health -= *damage;
            }
        }
        self.units.retain(|u| u.health > 0);
        for (idx, damage) in castle_damage.into_iter().enumerate() {
            self.castles[idx].health = (self.castles[idx].health - damage).max(0);
        }
    }

    fn tick_sudden_death(&mut self, dt: f32) {
        if !self.sudden_death() {
            return;
        }

        for team in [Team::Left, Team::Right] {
            let pressure = self.board_pressure(team.opponent());
            let slot = team.slot();
            self.overtime_damage_accum[slot] += pressure * dt;
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
        sim.place_building(
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
            sim.place_building(
                player,
                BuildingKind::VanguardBarracks,
                GridCell { x: 0, y: 0 }
            )
            .is_err()
        );
        assert!(
            sim.place_building(
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
            .place_building(player, BuildingKind::GroveRootDen, GridCell { x: 0, y: 0 })
            .unwrap_err();

        assert!(err.contains("not available"));
    }

    #[test]
    fn forge_increases_income_tick() {
        let mut sim = ready_two_players();
        let player = sim.players[0].id;
        sim.place_building(player, BuildingKind::VanguardForge, GridCell { x: 1, y: 1 })
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
            gold_after_buy + sim.economies[0].income
        );
    }

    #[test]
    fn buildings_spawn_units() {
        let mut sim = ready_two_players();
        let player = sim.players[0].id;
        sim.place_building(
            player,
            BuildingKind::VanguardBarracks,
            GridCell { x: 0, y: 0 },
        )
        .unwrap();
        sim.tick(6.0);
        assert_eq!(sim.units.len(), 1);
        assert_eq!(sim.units[0].kind, UnitKind::VanguardGuard);
    }

    #[test]
    fn units_can_destroy_castle_and_end_match() {
        let mut sim = ready_two_players();
        let unit_config = sim.balance.unit(UnitKind::VanguardGuard);
        let castle_armor = sim.castles[Team::Right.slot()].armor_type;
        sim.units.push(Unit {
            id: 500,
            owner: Team::Left,
            kind: UnitKind::VanguardGuard,
            health: 999,
            lane_pos: Team::Right.castle_pos() - 1.0,
            attack_timer: 0.0,
        });
        let (min_damage, _) = damage_range(unit_config.damage, unit_config.damage_variance);
        sim.castles[Team::Right.slot()].health =
            typed_damage(min_damage, unit_config.attack_type, castle_armor);
        sim.tick(0.1);
        assert_eq!(sim.phase, MatchPhase::GameOver);
        assert_eq!(sim.winner, Some(Team::Left));
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
        sim.place_building(
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
        sim.place_building(
            player,
            BuildingKind::VanguardBarracks,
            GridCell { x: 0, y: 0 },
        )
        .unwrap();
        sim.units.push(Unit {
            id: 900,
            owner: Team::Left,
            kind: UnitKind::VanguardGuard,
            health: sim.balance.unit(UnitKind::VanguardGuard).max_health,
            lane_pos: Team::Left.spawn_pos(),
            attack_timer: 0.0,
        });
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
