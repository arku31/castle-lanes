use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
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
mod render3d;
mod scene;
mod ui;
mod vfx;

use audio::*;
use input::*;
use net::*;
use render3d::*;
use scene::*;
use ui::*;
use vfx::*;

const WORLD_W: f32 = 2400.0;

const SIM_Y_TO_WORLD: f32 = 16.0;

const LANE_Y: f32 = 0.0;

const MAP_W: f32 = 3600.0;

const MAP_H: f32 = 560.0;

const CELL: f32 = 38.0;

const MINIMAP_SIZE: Vec2 = Vec2::new(230.0, 165.0);

/// Half-extents of the window in UI units (logical pixels). UI elements are
/// positioned relative to the screen center by `pin_ui_to_camera`, so every
/// edge-anchored cluster (top bar, bottom console, minimap, command card)
/// derives its offset from this instead of constants sized for one resolution.
#[derive(Resource, Clone, Copy)]
struct UiLayout {
    half: Vec2,
}

impl Default for UiLayout {
    fn default() -> Self {
        Self {
            half: Vec2::new(800.0, 450.0),
        }
    }
}

impl UiLayout {
    fn from_window(window: &Window) -> Self {
        Self {
            half: Vec2::new(window.width() * 0.5, window.height() * 0.5),
        }
    }

    /// Center Y of the top bar panel.
    fn top_bar_y(&self) -> f32 {
        self.half.y - 38.0
    }

    /// Center Y of the hint strip tucked under the top edge.
    fn hint_y(&self) -> f32 {
        self.half.y - 12.0
    }

    /// Center Y of the bottom console panel.
    fn panel_y(&self) -> f32 {
        -(self.half.y - 128.0)
    }

    /// Clicks with screen Y above this hit the world, not the console.
    fn panel_top_y(&self) -> f32 {
        self.panel_y() + 117.0
    }

    /// Half width of the top bar / bottom console; clamps on narrow windows.
    fn bar_half_width(&self) -> f32 {
        524.0_f32.min(self.half.x - 12.0)
    }

    /// Center of the first command card button; the grid grows right/down.
    fn command_grid_origin(&self) -> Vec2 {
        Vec2::new(self.half.x - 450.0, self.panel_y() + 60.0)
    }

    /// Center of the minimap panel in the bottom-left corner.
    fn minimap_center(&self) -> Vec2 {
        Vec2::new(-(self.half.x - 280.0), self.panel_y() + 5.0)
    }
}

fn update_ui_layout(windows: Query<&Window, With<PrimaryWindow>>, mut layout: ResMut<UiLayout>) {
    if let Ok(window) = windows.single() {
        *layout = UiLayout::from_window(window);
    }
}

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
    is_replay: bool,
    lobby_team_size: usize,
    lobby_random_factions: bool,
    queued_for_match: bool,
    profile_wins: u32,
    profile_losses: u32,
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

/// Local deterministic playback of a recorded match (plan.md Phase 3): the
/// client re-runs the recorded seed + intents through the sim and renders
/// the result; no server connection is used.
#[derive(Resource)]
struct ReplayPlayer {
    sim: castle_lanes::sim::GameSim,
    commands: Vec<castle_lanes::record::RecordedCommand>,
    next_command: usize,
    paused: bool,
    /// Accumulated sub-tick time scaled by playback speed.
    accumulator: f32,
}

#[derive(Resource, Default)]
struct ReplayControls {
    paused: bool,
    speed: f32,
}

fn run_replay(path: std::path::PathBuf) {
    // Replay mode: no server, no networking, the local sim drives everything.
    let record = match castle_lanes::record::load_record(&path) {
        Ok(record) => record,
        Err(err) => {
            eprintln!("failed to load replay {path:?}: {err}");
            std::process::exit(1);
        }
    };
    println!(
        "Replaying {}: {} players, {} commands, {} kills, winner {:?}",
        path.display(),
        record.players.len(),
        record.commands.len(),
        record.kills.len(),
        record.winner
    );

    let mut sim = castle_lanes::sim::GameSim::with_seed(
        castle_lanes::sim::BalanceConfig::load_or_default(castle_lanes::sim::DEFAULT_BALANCE_PATH),
        record.seed,
    );
    // Rebuild seats in recorded order so PlayerIds match the command log.
    for player in &record.players {
        let id = sim.join_or_update_player(player.name.clone()).unwrap().id;
        if let Some(race) = player.race {
            sim.set_race(id, race).unwrap();
        }
        sim.set_ready(id, true).unwrap();
    }
    sim.phase = MatchPhase::Playing;
    sim.reset_match_state();
    sim.phase = MatchPhase::Playing;

    let player = ReplayPlayer {
        sim,
        commands: record.commands.iter().cloned().collect(),
        next_command: 0,
        paused: false,
        accumulator: 0.0,
    };

    let socket = UdpSocket::bind("0.0.0.0:0").expect("bind replay socket");
    let client_net = ClientNet {
        socket,
        server_addr: "127.0.0.1:0".parse().unwrap(),
        connected: false,
        game_id: None,
        player_id: None,
        team: None,
        player_name: String::new(),
        auto_ready: false,
        auto_build_demo: false,
        auto_race: RaceKind::Vanguard,
        sent_auto_game: false,
        sent_auto_race: false,
        sent_auto_ready: false,
        sent_auto_build: false,
        last_join: Instant::now(),
        last_keepalive: Instant::now(),
        status: String::new(),
        next_seq: 0,
        pending_placement: None,
        is_replay: true,
        lobby_team_size: 1,
        lobby_random_factions: false,
        queued_for_match: false,
        profile_wins: 0,
        profile_losses: 0,
    };
    // replay client: same window/UI/render stack, driven by the local sim
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: format!("Castle Lanes v{} - REPLAY", castle_lanes::VERSION),
                        resolution: WindowResolution::new(1600, 900),
                        resizable: true,
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    file_path: std::env::current_dir()
                        .map(|cwd| cwd.join("assets").to_string_lossy().to_string())
                        .unwrap_or_else(|_| "assets".to_string()),
                    ..default()
                }),
        )
        .insert_resource(ClearColor(Color::srgb(0.07, 0.08, 0.09)))
        .register_type::<Transform>()
        .register_type::<GlobalTransform>()
        .register_type::<Visibility>()
        .register_type::<bevy::transform::components::TransformTreeChanged>()
        .register_type::<bevy::camera::visibility::InheritedVisibility>()
        .register_type::<bevy::camera::visibility::ViewVisibility>()
        .register_type::<bevy::mesh::Mesh3d>()
        .register_type::<bevy::camera::primitives::Aabb>()
        .register_type::<bevy::gltf::GltfMeshName>()
        .register_type::<bevy::gltf::GltfMaterialName>()
        .register_type::<ChildOf>()
        .register_type::<Children>()
        .register_type::<Name>()
        .insert_resource(ReplayControls::default())
        .insert_resource(DemoShots {
            enabled: false,
            counter: 0,
            next_at: 0.0,
        })
        .init_resource::<World3dAssets>()
        .init_resource::<CameraRig>()
        .init_resource::<BuildFx>()
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
        .insert_resource(client_net)
        .insert_resource(player)
        .init_resource::<UiLayout>()
        .add_systems(Startup, setup)
        .add_systems(Startup, setup_3d_world)
        .add_systems(
            Update,
            (
                update_ui_layout,
                auto_screenshots,
                replay_advance,
                replay_input,
                camera_controls,
                camera_rig_input,
                apply_camera_rig,
                detect_combat_vfx,
                update_world_hover,
                sync_static_scene,
                sync_units,
                animate_units,
                update_object_highlight,
                update_fog_tiles,
                orient_billboards,
                redraw_game_ui,
                pin_ui_to_camera,
                animate_grass,
                update_combat_vfx,
            )
                .chain(),
        )
        .run();
}

#[derive(Resource, Default)]
struct MatchHints {
    step: usize,
}

/// In-engine screenshot capture (plan-0.2.md M0 gate): F12 saves a manual
/// frame; `--demo-shots` records bursts for autonomous verification because
/// the desktop screenshot tooling on this box is unreliable.
#[derive(Resource)]
struct DemoShots {
    enabled: bool,
    counter: u64,
    next_at: f32,
}

fn auto_screenshots(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut demo: ResMut<DemoShots>,
) {
    if keys.just_pressed(KeyCode::F12) {
        let path = format!(
            "shots/manual_{}.png",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0)
        );
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path));
    }
    if demo.enabled {
        if demo.next_at == 0.0 {
            demo.next_at = time.elapsed_secs() + demo.interval_secs();
        } else if time.elapsed_secs() >= demo.next_at {
            demo.next_at = time.elapsed_secs() + demo.interval_secs();
            let path = format!("shots/auto_{:04}.png", demo.counter);
            demo.counter += 1;
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(path));
        }
    }
}

impl DemoShots {
    fn interval_secs(&self) -> f32 {
        2.5
    }
}

#[derive(Resource, Default)]
struct CameraHome {
    initialized_for: Option<Team>,
    /// Start-p camera fraction of the battlefield (capture/spectate aid).
    home_override: Option<f32>,
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
    side: Team,
    last_pos: Vec2,
    /// True when the sprite slot holds a Blender mesh (glTF scene) instead of
    /// a camera-facing quad; drives facing + bob behavior.
    is_model: bool,
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

/// Per-snapshot VFX budget: with fast/huge battles the combat inference can
/// try to spawn hundreds of streaks per snapshot, which turns the screen into
/// bar soup. Damage text always spawns; streaks/bursts consume budget.
#[derive(Resource)]
struct VfxBudget {
    remaining: u32,
}

impl VfxBudget {
    fn take(&mut self) -> bool {
        if self.remaining == 0 {
            return false;
        }
        self.remaining -= 1;
        true
    }
}

const VFX_BUDGET_PER_SNAPSHOT: u32 = 14;

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

/// Phase 4 crash reporting: panics write a timestamped log with the message,
/// backtrace, and system info to `crashes/` for post-mortem debugging.
fn install_crash_reporter() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let dir = std::path::Path::new("crashes");
        let _ = std::fs::create_dir_all(dir);
        let path = dir.join(format!("crash_{}.log", timestamp));
        let location = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown".to_string());
        let message = if let Some(msg) = info.payload().downcast_ref::<&str>() {
            msg.to_string()
        } else if let Some(msg) = info.payload().downcast_ref::<String>() {
            msg.clone()
        } else {
            "unknown panic payload".to_string()
        };
        let report = format!(
            "Castle Lanes v{} crash report\n\
             timestamp: {} (unix)\n\
             location: {}\n\
             message: {}\n\
             \nBacktrace:\n{:?}\n",
            castle_lanes::VERSION,
            timestamp,
            location,
            message,
            std::backtrace::Backtrace::force_capture()
        );
        let _ = std::fs::write(&path, report);
        eprintln!("Crash log written to {}", path.display());
        default_hook(info);
    }));
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

/// Advance the local replay sim and publish its snapshot to the render
/// pipeline. Space pauses, [ and ] step playback speed down/up.
fn replay_advance(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut controls: ResMut<ReplayControls>,
    mut player: ResMut<ReplayPlayer>,
    mut state: ResMut<SnapshotState>,
) {
    if keys.just_pressed(KeyCode::KeyP) {
        controls.paused = !controls.paused;
    }
    if keys.just_pressed(KeyCode::BracketLeft) {
        controls.speed = (controls.speed - 0.5).max(0.25);
    }
    if keys.just_pressed(KeyCode::BracketRight) {
        controls.speed = (controls.speed + 0.5).min(4.0);
    }
    if keys.just_pressed(KeyCode::Escape) {
        std::process::exit(0);
    }

    if !controls.paused {
        player.accumulator += time.delta_secs() * controls.speed;
    }
    let dt = 1.0 / 30.0;
    let mut ticks = 0;
    while player.accumulator >= dt && ticks < 8 * 30 {
        player.accumulator -= dt;
        player.sim.tick(dt);
        ticks += 1;
        loop {
            let next = player.next_command;
            let Some(command) = player.commands.get(next) else {
                break;
            };
            if command.tick > player.sim.tick {
                break;
            }
            let command = command.clone();
            apply_recorded_intent(&mut player.sim, &command);
            player.next_command += 1;
        }
    }

    let mut snapshot = player.sim.snapshot();
    snapshot.message = if controls.paused {
        format!("[PAUSED {:.1}x] {}", controls.speed, snapshot.message)
    } else {
        format!("[{:.1}x] {}", controls.speed, snapshot.message)
    };
    state.snapshot = Some(snapshot);
}

fn apply_recorded_intent(
    sim: &mut castle_lanes::sim::GameSim,
    command: &castle_lanes::record::RecordedCommand,
) {
    use castle_lanes::record::RecordedIntent;
    let player_id = PlayerId(command.player);
    let result = match &command.intent {
        RecordedIntent::SetRace { race } => sim.set_race(player_id, *race).map(|_| ()),
        RecordedIntent::SetReady { ready } => sim.set_ready(player_id, *ready),
        RecordedIntent::PlaceBuilding {
            kind,
            lane,
            zone,
            cell,
            ..
        } => sim.place_building(player_id, *kind, *lane, *zone, *cell),
        RecordedIntent::SellBuilding { building_id, .. } => {
            sim.sell_building(player_id, *building_id)
        }
        RecordedIntent::UpgradeBuilding {
            building_id, to, ..
        } => sim.upgrade_building(player_id, *building_id, *to),
        RecordedIntent::Surrender => sim.surrender(player_id),
        RecordedIntent::VoteRematch => {
            sim.vote_rematch(player_id);
            Ok(())
        }
    };
    if let Err(err) = result {
        eprintln!("replay command failed: {err}");
    }
}

fn replay_input() {}

fn main() {
    install_crash_reporter();
    let options = parse_args();
    if let Some(path) = options.replay.clone() {
        run_replay(path);
        return;
    }
    let socket = UdpSocket::bind("0.0.0.0:0").expect("bind client udp socket");
    socket.set_nonblocking(true).expect("set nonblocking");

    // Bevy 0.18 resolves the asset folder relative to the executable, which
    // breaks `target/debug` layouts: prefer the repo-relative assets/ dir.
    // Bevy 0.18 resolves RELATIVE asset paths against the executable dir
    // (target/debug), so the override must be absolute.
    let asset_file_path = std::env::current_dir()
        .map(|cwd| cwd.join("assets").to_string_lossy().to_string())
        .unwrap_or_else(|_| "assets".to_string());

    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: format!("Castle Lanes v{}", castle_lanes::VERSION),
                        resolution: WindowResolution::new(1600, 900),
                        resizable: true,
                        // Dev/demo aid: the capture pipeline on this box
                        // depends on the window actually being composited.
                        window_level: bevy::window::WindowLevel::AlwaysOnTop,
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    file_path: asset_file_path,
                    ..default()
                }),
        )
        .insert_resource(ClearColor(if is_3d() {
            Color::srgb(0.52, 0.65, 0.78)
        } else {
            Color::srgb(0.07, 0.08, 0.09)
        }))
        // glTF scenes spawn via reflection; these types must be registered.
        .register_type::<Transform>()
        .register_type::<GlobalTransform>()
        .register_type::<Visibility>()
        .register_type::<bevy::transform::components::TransformTreeChanged>()
        .register_type::<bevy::camera::visibility::InheritedVisibility>()
        .register_type::<bevy::camera::visibility::ViewVisibility>()
        .register_type::<bevy::mesh::Mesh3d>()
        .register_type::<bevy::camera::primitives::Aabb>()
        .register_type::<bevy::gltf::GltfMeshName>()
        .register_type::<bevy::gltf::GltfMaterialName>()
        .register_type::<ChildOf>()
        .register_type::<Children>()
        .register_type::<Name>()
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
            is_replay: false,
            lobby_team_size: 1,
            lobby_random_factions: false,
            queued_for_match: false,
            profile_wins: 0,
            profile_losses: 0,
        })
        .init_resource::<SnapshotState>()
        .init_resource::<BuildSelection>()
        .init_resource::<BuildHover>()
        .init_resource::<WorldSelection>()
        .init_resource::<WorldHover>()
        .init_resource::<CombatTracker>()
        .insert_resource(VfxBudget {
            remaining: VFX_BUDGET_PER_SNAPSHOT,
        })
        .init_resource::<CameraHome>()
        .init_resource::<FogMemory>()
        .init_resource::<RenderInterp>()
        .insert_resource(CameraHome {
            initialized_for: None,
            home_override: options.camera_x,
        })
        .init_resource::<SceneRegistry>()
        .init_resource::<SfxQueue>()
        .insert_resource(UiOverlays {
            help: options.show_help,
            settings: options.show_settings,
        })
        .init_resource::<MatchHints>()
        .insert_resource(ClientSettings::load())
        .init_resource::<UiLayout>()
        .init_resource::<World3dAssets>()
        .init_resource::<CameraRig>()
        .init_resource::<BuildFx>()
        .insert_resource(DemoShots {
            enabled: options.demo_shots,
            counter: 0,
            next_at: 0.0,
        })
        .add_systems(Startup, setup)
        .add_systems(Startup, setup_3d_world)
        .add_systems(
            Update,
            (
                update_ui_layout,
                auto_screenshots,
                (
                    (
                        receive_packets,
                        demo_automation,
                        menu_and_lobby_input,
                        build_selection_input,
                        ui_mouse_input,
                        placement_input,
                        camera_controls,
                        camera_rig_input,
                        apply_camera_rig,
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
                        update_build_fx,
                        orient_billboards,
                        redraw_game_ui,
                        pin_ui_to_camera,
                        animate_grass,
                        update_combat_vfx,
                    )
                        .chain(),
                )
                    .chain(),
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
    /// Start the camera at this fraction of the battlefield (0 = left home,
    /// 1 = right home) instead of the own base; capture/spectate aid.
    camera_x: Option<f32>,
    replay: Option<std::path::PathBuf>,
    /// Open with the help overlay visible (capture/demo aid).
    show_help: bool,
    /// Open with the settings overlay visible (capture/demo aid).
    show_settings: bool,
    /// Periodically save in-engine screenshots into shots/ (capture aid).
    demo_shots: bool,
}

fn parse_args() -> ClientOptions {
    let mut server = DEFAULT_SERVER_ADDR.to_string();
    let mut name = format!("Player{}", std::process::id() % 1000);
    let mut auto_ready = false;
    let mut auto_build_demo = false;
    let mut auto_race = RaceKind::Vanguard;
    let mut camera_x = None;
    let mut replay = None;
    let mut show_help = false;
    let mut show_settings = false;
    let mut demo_shots = false;
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
            // plan-0.2.md: v0.1 2D renderer stays available as escape hatch.
            "--renderer2d" => {
                set_renderer_3d(false);
            }
            // In-engine capture (spectacle is broken on this box).
            "--demo-shots" => {
                demo_shots = true;
            }
            "--show-help" => {
                show_help = true;
            }
            "--show-settings" => {
                show_settings = true;
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
        camera_x,
        replay,
        show_help,
        show_settings,
        demo_shots,
    }
}

fn argv_parse_fraction(value: &str) -> f32 {
    value.parse::<f32>().unwrap_or(0.5).clamp(0.0, 1.0)
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
    if is_3d() {
        // Dual-camera layout (plan-0.2.md §3.1): the UI Camera2d draws after
        // the 3D world camera and never clears it; it is the default UI
        // target so the whole v0.1 HUD keeps working unchanged.
        commands.spawn((
            Camera2d,
            Camera {
                order: 1,
                clear_color: ClearColorConfig::None,
                ..default()
            },
            // No MSAA on either camera: both render straight to the swapchain
            // so the HUD composes over the 3D pass deterministically (and the
            // Deck APU keeps the fill rate).
            Msaa::Off,
            bevy::ui::IsDefaultUiCamera,
        ));
    } else {
        commands.spawn(Camera2d);
    }
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
    if !is_3d() {
        spawn_grass_background(&mut commands);
        spawn_static_board(&mut commands, None, 2, None);
    } else {
        setup_models(commands.reborrow(), asset_server.clone());
    }
    let fonts = FontAssets {
        display: asset_server.load("fonts/MedievalSharp.ttf"),
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
        let load =
            |index: usize| asset_server.load(format!("art/units/frames/{name}_frame{index}.png"));
        frame_sets
            .frames
            .insert(kind, [load(0), load(1), load(2), load(3), load(4)]);
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
    wins: u32,
    losses: u32,
}

impl Default for ClientSettings {
    fn default() -> Self {
        Self {
            master_volume: 0.8,
            muted: false,
            fullscreen: false,
            resolution_index: 2,
            wins: 0,
            losses: 0,
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
            "wins": self.wins,
            "losses": self.losses,
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
