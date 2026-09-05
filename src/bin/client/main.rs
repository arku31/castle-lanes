use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::window::{PrimaryWindow, WindowResolution};
use castle_lanes::net::{
    ClientPacket, DEFAULT_SERVER_ADDR, GameId, GameInfo, PROTOCOL_VERSION, ServerPacket,
    apply_snapshot_delta, decode_server, encode,
};
use castle_lanes::sim::{
    ArmorType, AttackType, BalanceConfig, BuildZone, Building, BuildingKind, Castle as SimCastle,
    GRID_H, GRID_W, GridCell, LANE_LENGTH, Lane, MatchPhase, MatchSnapshot, PlayerId, RaceKind,
    Team, Unit, UnitKind, building_lane_pos,
};
use std::collections::{HashMap, HashSet};
use std::env;
use std::io::ErrorKind;
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

mod audio;
mod input;
mod net;
mod scene;
mod ui;
mod vfx;

use audio::*;
use input::*;
use net::*;
use scene::*;
use ui::*;
use vfx::*;

const WORLD_W: f32 = 2400.0;

const SIM_Y_TO_WORLD: f32 = 16.0;

const LANE_Y: f32 = 0.0;

const MAP_W: f32 = 3600.0;

const MAP_H: f32 = 560.0;

const CELL: f32 = 38.0;

const UI_PANEL_TOP_Y: f32 = -164.0;

const UI_PANEL_Y: f32 = -258.0;

const TOP_BAR_Y: f32 = 330.0;

const COMMAND_GRID_ORIGIN: Vec2 = Vec2::new(270.0, UI_PANEL_Y + 48.0);

const MINIMAP_CENTER: Vec2 = Vec2::new(-412.0, UI_PANEL_Y + 4.0);

const MINIMAP_SIZE: Vec2 = Vec2::new(184.0, 132.0);

const REVEAL_CASTLE_RADIUS: f32 = 430.0;

const REVEAL_BUILDING_RADIUS: f32 = 150.0;

const REVEAL_UNIT_RADIUS: f32 = 185.0;

const FOG_COLUMNS: usize = 96;

const FOG_ROWS: usize = 34;

const FOG_SOFT_EDGE: f32 = 82.0;

const TEAM_COLOR_LEFT: Color = Color::srgb(0.30, 0.58, 0.98);

const TEAM_COLOR_RIGHT: Color = Color::srgb(0.98, 0.36, 0.28);

const PANEL_BG: Color = Color::srgba(0.045, 0.040, 0.035, 0.92);

const PANEL_EDGE: Color = Color::srgba(0.58, 0.43, 0.22, 0.95);

const TEXT_GOLD: Color = Color::srgb(0.95, 0.78, 0.42);

const TEXT_PARCHMENT: Color = Color::srgb(0.93, 0.88, 0.73);

const VFX_Z: f32 = 32.0;

#[derive(Resource)]
struct ClientNet {
    socket: UdpSocket,
    server_addr: SocketAddr,
    connected: bool,
    game_id: Option<GameId>,
    player_id: Option<PlayerId>,
    team: Option<Team>,
    player_name: String,
    auto_ready: bool,
    auto_build_demo: bool,
    auto_race: RaceKind,
    sent_auto_game: bool,
    sent_auto_race: bool,
    sent_auto_ready: bool,
    sent_auto_build: bool,
    last_join: Instant,
    last_keepalive: Instant,
    status: String,
    next_seq: u32,
    pending_placement: Option<PendingPlacement>,
}

/// An unacknowledged placement intent, retried until acked or expired
/// (plan.md Phase 1 command acks).
#[derive(Clone, Copy)]
struct PendingPlacement {
    seq: u32,
    kind: BuildingKind,
    lane: Lane,
    zone: BuildZone,
    cell: GridCell,
    first_sent: Instant,
    last_sent: Instant,
}

#[derive(Resource, Default)]
struct SnapshotState {
    snapshot: Option<MatchSnapshot>,
    balance: BalanceConfig,
    games: Vec<GameInfo>,
}

#[derive(Resource)]
struct BuildSelection {
    kind: Option<BuildingKind>,
}

impl Default for BuildSelection {
    fn default() -> Self {
        Self { kind: None }
    }
}

#[derive(Resource, Default)]
struct BuildHover {
    kind: Option<BuildingKind>,
}

#[derive(Resource, Default)]
struct WorldSelection {
    selected: Option<SelectedObject>,
}

#[derive(Resource, Default)]
struct WorldHover {
    hovered: Option<SelectedObject>,
}

#[derive(Resource, Default)]
struct UiOverlays {
    help: bool,
    settings: bool,
}

#[derive(Resource, Default)]
struct MatchHints {
    step: usize,
}

#[derive(Resource, Default)]
struct CameraHome {
    initialized_for: Option<Team>,
}

#[derive(Resource)]
struct FogMemory {
    team: Option<Team>,
    explored: Vec<bool>,
    revision: u64,
}

impl Default for FogMemory {
    fn default() -> Self {
        Self {
            team: None,
            explored: vec![false; FOG_COLUMNS * FOG_ROWS],
            revision: 0,
        }
    }
}

/// Marker for board/building/castle entities rebuilt only when match state
/// structure changes (not per snapshot).
#[derive(Component)]
struct StaticScene;

/// Marker for fog overlay tiles; colors mutate in place per snapshot.
#[derive(Component)]
struct FogTile {
    col: usize,
    row: usize,
}

/// The two most recent snapshot ticks with receive timestamps, used to render
/// unit positions between server snapshots instead of snapping at 10 Hz.
#[derive(Resource, Default)]
struct RenderInterp {
    prev: Option<InterpSnapshot>,
    latest: Option<InterpSnapshot>,
}

#[derive(Clone)]
struct InterpSnapshot {
    at: Instant,
    tick: u64,
    unit_pos: HashMap<u64, castle_lanes::sim::WorldPos>,
}

#[derive(Clone, Copy)]
struct UnitVisual {
    root: Entity,
    sprite: Entity,
    badge_attack: Entity,
    badge_armor: Entity,
    health_fill: Entity,
    health: i32,
    sprite_base_y: f32,
    badge_base_y: f32,
    kind: UnitKind,
    last_pos: Vec2,
}

#[derive(Resource, Default)]
struct SceneRegistry {
    units: HashMap<u64, UnitVisual>,
    fog_tiles: Vec<Entity>,
    highlight: Option<Entity>,
    highlight_source: Option<HighlightSource>,
    statics_key: u64,
}

#[derive(Clone, Copy, PartialEq)]
enum HighlightSource {
    Unit(u64),
    Building(u64),
    Castle(Team),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SelectedObject {
    Unit(u64),
    Building(u64),
    Castle(Team),
    Cell(Team, Lane, BuildZone, GridCell),
}

#[derive(Resource, Default)]
struct CombatTracker {
    initialized: bool,
    units: HashMap<u64, TrackedUnit>,
    buildings: HashMap<u64, TrackedBuilding>,
    castle_health: Vec<i32>,
    seen_bounty_events: HashSet<u64>,
}

#[derive(Clone, Copy)]
struct TrackedUnit {
    health: i32,
    pos: Vec2,
}

#[derive(Clone, Copy)]
struct TrackedBuilding {
    health: i32,
}

#[derive(Resource)]
struct UnitSpriteAssets {
    vanguard_guard: Handle<Image>,
    vanguard_archer: Handle<Image>,
    vanguard_pikeman: Handle<Image>,
    vanguard_shieldbearer: Handle<Image>,
    vanguard_battle_cleric: Handle<Image>,
    vanguard_lancer: Handle<Image>,
    vanguard_ballista: Handle<Image>,
    grove_bruiser: Handle<Image>,
    grove_needler: Handle<Image>,
    grove_sproutling: Handle<Image>,
    grove_barkguard: Handle<Image>,
    grove_mire_shaman: Handle<Image>,
    grove_vine_stalker: Handle<Image>,
    grove_treant_colossus: Handle<Image>,
    ember_runner: Handle<Image>,
    ember_caster: Handle<Image>,
    ember_spark_imp: Handle<Image>,
    ember_obsidian_guard: Handle<Image>,
    ember_fire_lancer: Handle<Image>,
    ember_smoke_witch: Handle<Image>,
    ember_cinder_engine: Handle<Image>,
}

/// Multi-frame animation sets (plan.md Phase 2 item 5). Only units with a
/// generated frame atlas are present; everything else keeps its static
/// sprite. Frames: 0 idle, 1 walk, 2 wind-up, 3 strike, 4 death.
#[derive(Resource, Default)]
struct UnitFrameSets {
    frames: HashMap<UnitKind, [Handle<Image>; 5]>,
}

#[derive(Resource)]
struct FontAssets {
    /// Fantasy display face for headers and titles (OFL-licensed).
    display: Handle<Font>,
}

#[derive(Resource)]
struct BuildingIconAssets {
    vanguard_barracks: Handle<Image>,
    vanguard_range_tower: Handle<Image>,
    vanguard_forge: Handle<Image>,
    vanguard_pike_yard: Handle<Image>,
    vanguard_bulwark_hall: Handle<Image>,
    vanguard_chapel: Handle<Image>,
    vanguard_stables: Handle<Image>,
    vanguard_siege_workshop: Handle<Image>,
    grove_root_den: Handle<Image>,
    grove_thorn_spire: Handle<Image>,
    grove_bloom_well: Handle<Image>,
    grove_moss_nursery: Handle<Image>,
    grove_bark_bastion: Handle<Image>,
    grove_mire_pool: Handle<Image>,
    grove_vine_warren: Handle<Image>,
    grove_ancient_seed: Handle<Image>,
    ember_cinder_pit: Handle<Image>,
    ember_flame_spire: Handle<Image>,
    ember_ash_mine: Handle<Image>,
    ember_spark_kennel: Handle<Image>,
    ember_obsidian_gate: Handle<Image>,
    ember_blaze_stable: Handle<Image>,
    ember_smoke_altar: Handle<Image>,
    ember_inferno_engine: Handle<Image>,
}

#[derive(Component)]
struct SceneEntity;

#[derive(Component)]
struct UiEntity;

#[derive(Component)]
struct GrassBlade {
    base_pos: Vec2,
    base_rotation: f32,
    phase: f32,
    sway: f32,
}

#[derive(Component)]
struct UiPinned {
    pos: Vec2,
    size: Option<Vec2>,
    font_size: Option<f32>,
    z: f32,
}

#[derive(Component)]
struct PreviewEntity;

#[derive(Component)]
struct CombatVfx {
    lifetime: f32,
    max_lifetime: f32,
    velocity: Vec2,
}

/// Resolve an entity's owning player to its side; presentation logic is
/// side-based while entities carry PlayerId owners (plan.md Phase 1 item 3).
/// Position of a player in the snapshot's per-player vectors.
fn player_index_of(snapshot: &MatchSnapshot, player_id: PlayerId) -> usize {
    snapshot
        .players
        .iter()
        .position(|player| player.id == player_id)
        .unwrap_or(0)
}

fn side_of_player(snapshot: &MatchSnapshot, owner: PlayerId) -> Team {
    snapshot
        .players
        .iter()
        .find(|player| player.id == owner)
        .map(|player| player.team)
        .unwrap_or(Team::Left)
}

/// True when the owner's side matches the viewer (no viewer = everything).
fn side_matches_viewer(
    snapshot: &MatchSnapshot,
    owner: PlayerId,
    viewer_team: Option<Team>,
) -> bool {
    match viewer_team {
        None => true,
        Some(team) => side_of_player(snapshot, owner) == team,
    }
}

fn main() {
    let options = parse_args();
    let socket = UdpSocket::bind("0.0.0.0:0").expect("bind client udp socket");
    socket.set_nonblocking(true).expect("set nonblocking");

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: format!("Castle Lanes v{}", castle_lanes::VERSION),
                resolution: WindowResolution::new(1100, 720),
                resizable: true,
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb(0.07, 0.08, 0.09)))
        .insert_resource(ClientNet {
            socket,
            server_addr: options.server_addr,
            connected: false,
            game_id: None,
            player_id: None,
            team: None,
            player_name: options.player_name,
            auto_ready: options.auto_ready,
            auto_build_demo: options.auto_build_demo,
            auto_race: options.auto_race,
            sent_auto_game: false,
            sent_auto_race: false,
            sent_auto_ready: false,
            sent_auto_build: false,
            last_join: Instant::now() - Duration::from_secs(3),
            last_keepalive: Instant::now(),
            status: "Press Enter to connect to the lobby server.".to_string(),
            next_seq: 1,
            pending_placement: None,
        })
        .init_resource::<SnapshotState>()
        .init_resource::<BuildSelection>()
        .init_resource::<BuildHover>()
        .init_resource::<WorldSelection>()
        .init_resource::<WorldHover>()
        .init_resource::<CombatTracker>()
        .init_resource::<CameraHome>()
        .init_resource::<FogMemory>()
        .init_resource::<RenderInterp>()
        .init_resource::<SceneRegistry>()
        .init_resource::<SfxQueue>()
        .init_resource::<UiOverlays>()
        .init_resource::<MatchHints>()
        .insert_resource(ClientSettings::load())
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                (
                    receive_packets,
                    demo_automation,
                    menu_and_lobby_input,
                    build_selection_input,
                    ui_mouse_input,
                    placement_input,
                    camera_controls,
                    detect_combat_vfx,
                    update_build_hover,
                    update_world_hover,
                    update_fog_memory,
                ),
                (
                    play_sfx_queue,
                    volume_toggle_input,
                    announce_phase_sfx,
                    sync_static_scene,
                    sync_units,
                    animate_units,
                    update_object_highlight,
                    update_fog_tiles,
                    update_placement_preview,
                    redraw_game_ui,
                    pin_ui_to_camera,
                    animate_grass,
                    update_combat_vfx,
                ),
            )
                .chain(),
        )
        .run();
}

struct ClientOptions {
    server_addr: SocketAddr,
    player_name: String,
    auto_ready: bool,
    auto_build_demo: bool,
    auto_race: RaceKind,
}

fn parse_args() -> ClientOptions {
    let mut server = DEFAULT_SERVER_ADDR.to_string();
    let mut name = format!("Player{}", std::process::id() % 1000);
    let mut auto_ready = false;
    let mut auto_build_demo = false;
    let mut auto_race = RaceKind::Vanguard;
    let args: Vec<String> = env::args().collect();
    let mut legacy_json = false;
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
            "--auto-ready" => {
                auto_ready = true;
            }
            "--auto-build-demo" => {
                auto_build_demo = true;
            }
            "--race" if idx + 1 < args.len() => {
                auto_race = parse_race(&args[idx + 1]);
                idx += 1;
            }
            "--legacy-json" => {
                legacy_json = true;
            }
            _ => {}
        }
        idx += 1;
    }
    if legacy_json {
        castle_lanes::net::set_outgoing_format(castle_lanes::net::PacketFormat::Json);
    }
    let server_addr = server.parse().expect("--server must be host:port");
    ClientOptions {
        server_addr,
        player_name: name,
        auto_ready,
        auto_build_demo,
        auto_race,
    }
}

fn parse_race(value: &str) -> RaceKind {
    match value.to_ascii_lowercase().as_str() {
        "vanguard" | "human" | "humans" | "v" | "1" => RaceKind::Vanguard,
        "grove" | "forest" | "g" | "2" => RaceKind::Grove,
        "ember" | "fire" | "e" | "3" => RaceKind::Ember,
        _ => panic!("--race must be vanguard, grove, or ember"),
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);
    commands.insert_resource(UnitSpriteAssets {
        vanguard_guard: asset_server.load("art/units/vanguard_guard.png"),
        vanguard_archer: asset_server.load("art/units/vanguard_archer.png"),
        vanguard_pikeman: asset_server.load("art/units/vanguard_pikeman.png"),
        vanguard_shieldbearer: asset_server.load("art/units/vanguard_shieldbearer.png"),
        vanguard_battle_cleric: asset_server.load("art/units/vanguard_battle_cleric.png"),
        vanguard_lancer: asset_server.load("art/units/vanguard_lancer.png"),
        vanguard_ballista: asset_server.load("art/units/vanguard_ballista.png"),
        grove_bruiser: asset_server.load("art/units/grove_bruiser.png"),
        grove_needler: asset_server.load("art/units/grove_needler.png"),
        grove_sproutling: asset_server.load("art/units/grove_sproutling.png"),
        grove_barkguard: asset_server.load("art/units/grove_barkguard.png"),
        grove_mire_shaman: asset_server.load("art/units/grove_mire_shaman.png"),
        grove_vine_stalker: asset_server.load("art/units/grove_vine_stalker.png"),
        grove_treant_colossus: asset_server.load("art/units/grove_treant_colossus.png"),
        ember_runner: asset_server.load("art/units/ember_runner.png"),
        ember_caster: asset_server.load("art/units/ember_caster.png"),
        ember_spark_imp: asset_server.load("art/units/ember_spark_imp.png"),
        ember_obsidian_guard: asset_server.load("art/units/ember_obsidian_guard.png"),
        ember_fire_lancer: asset_server.load("art/units/ember_fire_lancer.png"),
        ember_smoke_witch: asset_server.load("art/units/ember_smoke_witch.png"),
        ember_cinder_engine: asset_server.load("art/units/ember_cinder_engine.png"),
    });
    commands.insert_resource(BuildingIconAssets {
        vanguard_barracks: asset_server.load("art/buildings/vanguard_barracks.png"),
        vanguard_range_tower: asset_server.load("art/buildings/vanguard_range_tower.png"),
        vanguard_forge: asset_server.load("art/buildings/vanguard_forge.png"),
        vanguard_pike_yard: asset_server.load("art/buildings/vanguard_pike_yard.png"),
        vanguard_bulwark_hall: asset_server.load("art/buildings/vanguard_bulwark_hall.png"),
        vanguard_chapel: asset_server.load("art/buildings/vanguard_chapel.png"),
        vanguard_stables: asset_server.load("art/buildings/vanguard_stables.png"),
        vanguard_siege_workshop: asset_server.load("art/buildings/vanguard_siege_workshop.png"),
        grove_root_den: asset_server.load("art/buildings/grove_root_den.png"),
        grove_thorn_spire: asset_server.load("art/buildings/grove_thorn_spire.png"),
        grove_bloom_well: asset_server.load("art/buildings/grove_bloom_well.png"),
        grove_moss_nursery: asset_server.load("art/buildings/grove_moss_nursery.png"),
        grove_bark_bastion: asset_server.load("art/buildings/grove_bark_bastion.png"),
        grove_mire_pool: asset_server.load("art/buildings/grove_mire_pool.png"),
        grove_vine_warren: asset_server.load("art/buildings/grove_vine_warren.png"),
        grove_ancient_seed: asset_server.load("art/buildings/grove_ancient_seed.png"),
        ember_cinder_pit: asset_server.load("art/buildings/ember_cinder_pit.png"),
        ember_flame_spire: asset_server.load("art/buildings/ember_flame_spire.png"),
        ember_ash_mine: asset_server.load("art/buildings/ember_ash_mine.png"),
        ember_spark_kennel: asset_server.load("art/buildings/ember_spark_kennel.png"),
        ember_obsidian_gate: asset_server.load("art/buildings/ember_obsidian_gate.png"),
        ember_blaze_stable: asset_server.load("art/buildings/ember_blaze_stable.png"),
        ember_smoke_altar: asset_server.load("art/buildings/ember_smoke_altar.png"),
        ember_inferno_engine: asset_server.load("art/buildings/ember_inferno_engine.png"),
    });
    spawn_grass_background(&mut commands);
    spawn_static_board(&mut commands, None);
    let fonts = FontAssets {
        display: asset_server.load("fonts/medievalsharp.ttf"),
    };
    commands.insert_resource(fonts);

    let mut frame_sets = UnitFrameSets::default();
    let vanguard_frames: [(&str, UnitKind); 8] = [
        ("vanguard_guard", UnitKind::VanguardGuard),
        ("vanguard_archer", UnitKind::VanguardArcher),
        ("vanguard_pikeman", UnitKind::VanguardPikeman),
        ("vanguard_shieldbearer", UnitKind::VanguardShieldbearer),
        ("vanguard_battle_cleric", UnitKind::VanguardBattleCleric),
        ("vanguard_lancer", UnitKind::VanguardLancer),
        ("vanguard_ballista", UnitKind::VanguardBallista),
        ("vanguard_arbalester", UnitKind::VanguardArbalester),
    ];
    for (name, kind) in vanguard_frames {
        let load = |index: usize| {
            asset_server.load(format!("art/units/frames/{name}_frame{index}.png"))
        };
        frame_sets.frames.insert(
            kind,
            [load(0), load(1), load(2), load(3), load(4)],
        );
    }
    commands.insert_resource(frame_sets);
    let ambient = AudioAssets {
        ui_click: asset_server.load("audio/ui_click.wav"),
        build_place: asset_server.load("audio/build_place.wav"),
        build_error: asset_server.load("audio/build_error.wav"),
        unit_spawn: asset_server.load("audio/unit_spawn.wav"),
        melee_hit: asset_server.load("audio/melee_hit.wav"),
        ranged_shot: asset_server.load("audio/ranged_shot.wav"),
        unit_death: asset_server.load("audio/unit_death.wav"),
        bounty_coin: asset_server.load("audio/bounty_coin.wav"),
        castle_alarm: asset_server.load("audio/castle_alarm.wav"),
        victory: asset_server.load("audio/victory.wav"),
        defeat: asset_server.load("audio/defeat.wav"),
    };
    commands.insert_resource(ambient);
    commands.spawn((
        AudioPlayer::new(asset_server.load("audio/ambient_loop.wav")),
        PlaybackSettings::LOOP.with_volume(bevy::audio::Volume::Linear(0.05)),
    ));
}

const PLACEMENT_RETRY_INTERVAL: Duration = Duration::from_millis(350);

const PLACEMENT_RETRY_WINDOW: Duration = Duration::from_millis(1500);

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Sfx {
    UiClick,
    BuildPlace,
    BuildError,
    UnitSpawn,
    MeleeHit,
    RangedShot,
    UnitDeath,
    BountyCoin,
    CastleAlarm,
    Victory,
    Defeat,
}

impl Sfx {
    fn handle<'a>(&self, assets: &'a AudioAssets) -> &'a Handle<AudioSource> {
        match self {
            Self::UiClick => &assets.ui_click,
            Self::BuildPlace => &assets.build_place,
            Self::BuildError => &assets.build_error,
            Self::UnitSpawn => &assets.unit_spawn,
            Self::MeleeHit => &assets.melee_hit,
            Self::RangedShot => &assets.ranged_shot,
            Self::UnitDeath => &assets.unit_death,
            Self::BountyCoin => &assets.bounty_coin,
            Self::CastleAlarm => &assets.castle_alarm,
            Self::Victory => &assets.victory,
            Self::Defeat => &assets.defeat,
        }
    }

    fn gain(&self) -> f32 {
        match self {
            Self::UiClick => 0.55,
            Self::BuildPlace => 0.7,
            Self::BuildError => 0.5,
            Self::UnitSpawn => 0.35,
            Self::MeleeHit => 0.28,
            Self::RangedShot => 0.3,
            Self::UnitDeath => 0.4,
            Self::BountyCoin => 0.6,
            Self::CastleAlarm => 0.65,
            Self::Victory => 0.85,
            Self::Defeat => 0.85,
        }
    }

    /// Minimum spacing between plays of the same sound, so 30 simultaneous
    /// melee hits stay a battle rumble instead of a wall of noise.
    fn cooldown(&self) -> Duration {
        match self {
            Self::MeleeHit | Self::RangedShot => Duration::from_millis(70),
            Self::UnitDeath => Duration::from_millis(90),
            Self::UnitSpawn => Duration::from_millis(110),
            Self::CastleAlarm => Duration::from_secs(4),
            Self::Victory | Self::Defeat => Duration::from_secs(2),
            _ => Duration::from_millis(40),
        }
    }
}

#[derive(Resource)]
struct AudioAssets {
    ui_click: Handle<AudioSource>,
    build_place: Handle<AudioSource>,
    build_error: Handle<AudioSource>,
    unit_spawn: Handle<AudioSource>,
    melee_hit: Handle<AudioSource>,
    ranged_shot: Handle<AudioSource>,
    unit_death: Handle<AudioSource>,
    bounty_coin: Handle<AudioSource>,
    castle_alarm: Handle<AudioSource>,
    victory: Handle<AudioSource>,
    defeat: Handle<AudioSource>,
}

#[derive(Resource, Default)]
struct SfxQueue {
    queue: Vec<Sfx>,
    last_played: HashMap<Sfx, Instant>,
}

impl SfxQueue {
    fn push(&mut self, sfx: Sfx) {
        if let Some(last) = self.last_played.get(&sfx) {
            if last.elapsed() < sfx.cooldown() {
                return;
            }
        }
        self.last_played.insert(sfx, Instant::now());
        self.queue.push(sfx);
    }
}

/// Resolution presets for the settings panel (plan.md Phase 1 item 10).
const RESOLUTION_PRESETS: [(f32, f32); 4] = [
    (1280.0, 720.0),
    (1600.0, 900.0),
    (1920.0, 1080.0),
    (2560.0, 1440.0),
];

#[derive(Resource, Clone)]
struct ClientSettings {
    master_volume: f32,
    muted: bool,
    fullscreen: bool,
    resolution_index: usize,
}

impl Default for ClientSettings {
    fn default() -> Self {
        Self {
            master_volume: 0.8,
            muted: false,
            fullscreen: false,
            resolution_index: 2,
        }
    }
}

impl ClientSettings {
    fn settings_path() -> std::path::PathBuf {
        std::path::PathBuf::from("config/client_settings.json")
    }

    fn load() -> Self {
        let mut settings = Self::default();
        if let Ok(raw) = std::fs::read_to_string(Self::settings_path()) {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&raw) {
                if let Some(value) = parsed.get("master_volume").and_then(|v| v.as_f64()) {
                    settings.master_volume = (value as f32).clamp(0.0, 1.0);
                }
                if let Some(muted) = parsed.get("muted").and_then(|v| v.as_bool()) {
                    settings.muted = muted;
                }
                if let Some(fullscreen) = parsed.get("fullscreen").and_then(|v| v.as_bool()) {
                    settings.fullscreen = fullscreen;
                }
                if let Some(index) = parsed.get("resolution_index").and_then(|v| v.as_u64()) {
                    settings.resolution_index = (index as usize).min(RESOLUTION_PRESETS.len() - 1);
                }
            }
        }
        settings
    }

    fn save(&self) {
        let payload = serde_json::json!({
            "master_volume": self.master_volume,
            "muted": self.muted,
            "fullscreen": self.fullscreen,
            "resolution_index": self.resolution_index,
        });
        let _ = std::fs::write(Self::settings_path(), payload.to_string());
    }

    fn effective_volume(&self) -> f32 {
        if self.muted { 0.0 } else { self.master_volume }
    }

    fn resolution(&self) -> (f32, f32) {
        RESOLUTION_PRESETS[self.resolution_index.min(RESOLUTION_PRESETS.len() - 1)]
    }
}

const UNIT_HEALTH_BAR_W: f32 = 34.0;

const UNIT_ROOT_Z: f32 = 8.4;

fn default_lane_for_team(team: Team) -> Lane {
    match team {
        Team::Left => Lane::Top,
        Team::Right => Lane::Bottom,
    }
}

#[cfg(test)]
mod settings_tests {
    use super::*;

    #[test]
    fn client_settings_round_trip_through_disk() {
        let original = std::fs::read_to_string(ClientSettings::settings_path()).ok();

        let mut settings = ClientSettings::default();
        settings.master_volume = 0.55;
        settings.muted = true;
        settings.fullscreen = true;
        settings.resolution_index = 1;
        settings.save();

        let loaded = ClientSettings::load();
        assert!((loaded.master_volume - 0.55).abs() < 1e-6);
        assert!(loaded.muted);
        assert!(loaded.fullscreen);
        assert_eq!(loaded.resolution_index, 1);

        // restore whatever the user had (or clear our defaults)
        match original {
            Some(raw) => std::fs::write(ClientSettings::settings_path(), raw).unwrap(),
            None => {
                let _ = std::fs::remove_file(ClientSettings::settings_path());
            }
        }
    }
}
