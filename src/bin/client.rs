use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::window::{PrimaryWindow, WindowResolution};
use castle_lanes::net::{
    ClientPacket, DEFAULT_SERVER_ADDR, PROTOCOL_VERSION, ServerPacket, decode_server, encode,
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
const PANEL_BG: Color = Color::srgba(0.045, 0.040, 0.035, 0.92);
const PANEL_EDGE: Color = Color::srgba(0.58, 0.43, 0.22, 0.95);
const TEXT_GOLD: Color = Color::srgb(0.95, 0.78, 0.42);
const TEXT_PARCHMENT: Color = Color::srgb(0.93, 0.88, 0.73);
const VFX_Z: f32 = 32.0;

#[derive(Resource)]
struct ClientNet {
    socket: UdpSocket,
    server_addr: SocketAddr,
    player_id: Option<PlayerId>,
    team: Option<Team>,
    player_name: String,
    auto_ready: bool,
    auto_build_demo: bool,
    auto_race: RaceKind,
    sent_auto_race: bool,
    sent_auto_ready: bool,
    sent_auto_build: bool,
    last_join: Instant,
    last_keepalive: Instant,
    status: String,
}

#[derive(Resource, Default)]
struct SnapshotState {
    snapshot: Option<MatchSnapshot>,
    balance: BalanceConfig,
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
struct CameraHome {
    initialized_for: Option<Team>,
}

#[derive(Resource)]
struct FogMemory {
    team: Option<Team>,
    explored: Vec<bool>,
}

impl Default for FogMemory {
    fn default() -> Self {
        Self {
            team: None,
            explored: vec![false; FOG_COLUMNS * FOG_ROWS],
        }
    }
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
    castle_health: [i32; 2],
    seen_bounty_events: HashSet<u64>,
}

#[derive(Clone, Copy)]
struct TrackedUnit {
    health: i32,
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

fn main() {
    let options = parse_args();
    let socket = UdpSocket::bind("0.0.0.0:0").expect("bind client udp socket");
    socket.set_nonblocking(true).expect("set nonblocking");

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Castle Lanes MVP".to_string(),
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
            player_id: None,
            team: None,
            player_name: options.player_name,
            auto_ready: options.auto_ready,
            auto_build_demo: options.auto_build_demo,
            auto_race: options.auto_race,
            sent_auto_race: false,
            sent_auto_ready: false,
            sent_auto_build: false,
            last_join: Instant::now() - Duration::from_secs(3),
            last_keepalive: Instant::now(),
            status: "Press Enter to join the dedicated server.".to_string(),
        })
        .init_resource::<SnapshotState>()
        .init_resource::<BuildSelection>()
        .init_resource::<BuildHover>()
        .init_resource::<WorldSelection>()
        .init_resource::<WorldHover>()
        .init_resource::<CombatTracker>()
        .init_resource::<CameraHome>()
        .init_resource::<FogMemory>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
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
                redraw_scene,
                update_placement_preview,
                redraw_game_ui,
                pin_ui_to_camera,
                animate_grass,
                update_combat_vfx,
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
            _ => {}
        }
        idx += 1;
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
}

fn receive_packets(mut net: ResMut<ClientNet>, mut state: ResMut<SnapshotState>) {
    let mut buf = [0_u8; 65_535];
    loop {
        match net.socket.recv_from(&mut buf) {
            Ok((len, _)) => match decode_server(&buf[..len]) {
                Ok(ServerPacket::Welcome { player }) => {
                    net.player_id = Some(player.id);
                    net.team = Some(player.team);
                    net.status = format!("Joined as {} ({:?}).", player.name, player.team);
                }
                Ok(ServerPacket::BalanceRaces {
                    starting_gold,
                    base_income,
                    income_interval,
                    interest_rate,
                    sudden_death_start,
                    races,
                }) => {
                    state.balance.starting_gold = starting_gold;
                    state.balance.base_income = base_income;
                    state.balance.income_interval = income_interval;
                    state.balance.interest_rate = interest_rate;
                    state.balance.sudden_death_start = sudden_death_start;
                    state.balance.races = races;
                }
                Ok(ServerPacket::BalanceBuildings { buildings }) => {
                    state.balance.buildings = buildings;
                }
                Ok(ServerPacket::BalanceUnits { units }) => {
                    state.balance.units = units;
                }
                Ok(ServerPacket::Snapshot(snapshot)) => {
                    state.snapshot = Some(snapshot);
                }
                Ok(ServerPacket::Error { message }) => {
                    net.status = message;
                }
                Err(err) => {
                    net.status = format!("Bad server packet: {err}");
                }
            },
            Err(err) if err.kind() == ErrorKind::WouldBlock => break,
            Err(err) => {
                net.status = format!("Network error: {err}");
                break;
            }
        }
    }

    if net.player_id.is_none() && net.last_join.elapsed() > Duration::from_secs(2) {
        send_join(&mut net);
    } else if net.player_id.is_some() && net.last_keepalive.elapsed() > Duration::from_secs(2) {
        send_join(&mut net);
        net.last_keepalive = Instant::now();
    }
}

fn demo_automation(mut net: ResMut<ClientNet>, state: Res<SnapshotState>) {
    let Some(player_id) = net.player_id else {
        return;
    };
    if !net.sent_auto_race {
        send_client(
            &net,
            &ClientPacket::SetRace {
                player_id,
                race: net.auto_race,
            },
        );
        net.sent_auto_race = true;
    }
    if net.auto_ready && !net.sent_auto_ready {
        let Some(snapshot) = &state.snapshot else {
            return;
        };
        if current_player(snapshot, player_id)
            .and_then(|player| player.race)
            .is_none()
        {
            return;
        }
        send_client(
            &net,
            &ClientPacket::SetReady {
                player_id,
                ready: true,
            },
        );
        net.sent_auto_ready = true;
    }

    if !net.auto_build_demo || net.sent_auto_build {
        return;
    }
    let Some(snapshot) = &state.snapshot else {
        return;
    };
    if snapshot.phase != MatchPhase::Playing {
        return;
    }
    let race = current_player(snapshot, player_id)
        .and_then(|player| player.race)
        .unwrap_or(net.auto_race);
    let balance = active_balance(&state);
    let kind = balance.race(race).buildings[0];
    send_client(
        &net,
        &ClientPacket::PlaceBuilding {
            player_id,
            kind,
            lane: default_lane_for_team(net.team.unwrap_or(Team::Left)),
            zone: BuildZone::Front,
            cell: GridCell { x: 0, y: 0 },
        },
    );
    net.sent_auto_build = true;
}

fn menu_and_lobby_input(keys: Res<ButtonInput<KeyCode>>, mut net: ResMut<ClientNet>) {
    if keys.just_pressed(KeyCode::Enter) {
        send_join(&mut net);
    }
    if keys.just_pressed(KeyCode::KeyR) {
        if let Some(player_id) = net.player_id {
            send_client(&net, &ClientPacket::VoteRematch { player_id });
        }
    }
}

fn build_selection_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut selection: ResMut<BuildSelection>,
    net: Res<ClientNet>,
    state: Res<SnapshotState>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        selection.kind = None;
        return;
    }
    let Some(race) = selected_race(&state, &net) else {
        return;
    };
    let balance = active_balance(&state);
    let buildings = &balance.race(race).buildings;
    if selection.kind.is_some_and(|kind| kind.race() != race) {
        selection.kind = None;
    }
    for (key, idx) in [
        (KeyCode::Digit1, 0),
        (KeyCode::Digit2, 1),
        (KeyCode::Digit3, 2),
        (KeyCode::Digit4, 3),
        (KeyCode::Digit5, 4),
        (KeyCode::Digit6, 5),
        (KeyCode::Digit7, 6),
        (KeyCode::Digit8, 7),
    ] {
        if keys.just_pressed(key) {
            if let Some(kind) = buildings.get(idx) {
                selection.kind = Some(*kind);
            }
        }
    }
}

fn ui_mouse_input(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut selection: ResMut<BuildSelection>,
    net: Res<ClientNet>,
    state: Res<SnapshotState>,
) {
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let Some(screen) = cursor_screen_pos(&windows) else {
        return;
    };

    if race_selection_popup_open(&state, &net) {
        if let (Some(snapshot), Some(player_id)) = (&state.snapshot, net.player_id) {
            let selected_race = current_player(snapshot, player_id).and_then(|player| player.race);
            if selected_race.is_some()
                && point_in_rect(screen, ready_button_center(), ready_button_size())
            {
                send_client(
                    &net,
                    &ClientPacket::SetReady {
                        player_id,
                        ready: true,
                    },
                );
                return;
            }

            for (idx, race) in RaceKind::ALL.iter().enumerate() {
                let center = race_button_center(idx);
                if point_in_rect(screen, center, race_button_size()) {
                    send_client(
                        &net,
                        &ClientPacket::SetRace {
                            player_id,
                            race: *race,
                        },
                    );
                    selection.kind = None;
                    return;
                }
            }
        }
        return;
    }

    if screen.y > UI_PANEL_TOP_Y {
        return;
    }

    let Some(race) = selected_race(&state, &net) else {
        return;
    };
    let balance = active_balance(&state);
    for (idx, kind) in balance.race(race).buildings.iter().enumerate() {
        let center = command_button_center(idx);
        if point_in_rect(screen, center, Vec2::splat(54.0)) {
            selection.kind = Some(*kind);
            return;
        }
    }
    if point_in_rect(screen, command_button_center(8), Vec2::splat(54.0)) {
        selection.kind = None;
    }
}

fn placement_input(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    net: Res<ClientNet>,
    mut selection: ResMut<BuildSelection>,
    mut world_selection: ResMut<WorldSelection>,
    state: Res<SnapshotState>,
) {
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let Some(snapshot) = &state.snapshot else {
        return;
    };
    if snapshot.phase != MatchPhase::Playing {
        return;
    }
    let (Some(player_id), Some(team)) = (net.player_id, net.team) else {
        return;
    };
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(screen) = cursor_screen_pos(&windows) else {
        return;
    };
    let Ok((camera, transform)) = camera_query.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Ok(world) = camera.viewport_to_world_2d(transform, cursor) else {
        return;
    };
    if screen.y <= UI_PANEL_TOP_Y {
        return;
    }
    let Some(kind) = selection.kind else {
        if let Some(picked) = pick_world_object(snapshot, world, net.team) {
            world_selection.selected = Some(picked);
        } else {
            world_selection.selected = None;
        }
        return;
    };

    let Some((lane, zone, cell)) = world_to_build_slot(team, world) else {
        return;
    };
    let balance = active_balance(&state);
    if !can_place_building(&balance, snapshot, player_id, team, kind, lane, zone, cell) {
        return;
    }
    send_client(
        &net,
        &ClientPacket::PlaceBuilding {
            player_id,
            kind,
            lane,
            zone,
            cell,
        },
    );
    world_selection.selected = Some(SelectedObject::Cell(team, lane, zone, cell));
    selection.kind = None;
}

fn can_place_building(
    balance: &BalanceConfig,
    snapshot: &MatchSnapshot,
    player_id: PlayerId,
    team: Team,
    kind: BuildingKind,
    lane: Lane,
    zone: BuildZone,
    cell: GridCell,
) -> bool {
    let occupied = snapshot.buildings.iter().any(|building| {
        building.owner == team
            && building.lane == lane
            && building.zone == zone
            && building.cell == cell
    });
    if occupied {
        return false;
    }
    let gold = snapshot
        .players
        .iter()
        .find(|player| player.id == player_id)
        .map(|player| snapshot.economies[player.team.slot()].gold)
        .unwrap_or_default();
    kind.race() == team_race(snapshot, team).unwrap_or(kind.race())
        && gold >= balance.building(kind).cost
}

fn update_placement_preview(
    mut commands: Commands,
    preview_query: Query<Entity, With<PreviewEntity>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    net: Res<ClientNet>,
    selection: Res<BuildSelection>,
    state: Res<SnapshotState>,
    building_icons: Res<BuildingIconAssets>,
) {
    for entity in &preview_query {
        commands.entity(entity).despawn();
    }

    let Some(snapshot) = &state.snapshot else {
        return;
    };
    if snapshot.phase != MatchPhase::Playing {
        return;
    }
    let (Some(player_id), Some(team)) = (net.player_id, net.team) else {
        return;
    };
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(screen) = cursor_screen_pos(&windows) else {
        return;
    };
    let Ok((camera, transform)) = camera_query.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Ok(world) = camera.viewport_to_world_2d(transform, cursor) else {
        return;
    };
    if screen.y <= UI_PANEL_TOP_Y {
        return;
    }
    let Some(kind) = selection.kind else {
        return;
    };

    let balance = active_balance(&state);
    let slot = world_to_build_slot(team, world);
    let valid = slot.is_some_and(|(lane, zone, cell)| {
        can_place_building(&balance, snapshot, player_id, team, kind, lane, zone, cell)
    });
    let color = if valid {
        Color::srgb(0.30, 0.85, 0.46).with_alpha(0.45)
    } else {
        Color::srgb(0.95, 0.20, 0.18).with_alpha(0.45)
    };
    let pos = slot
        .map(|(lane, zone, cell)| cell_to_world(team, lane, zone, cell))
        .unwrap_or(world);
    commands.spawn((
        Sprite::from_color(color, Vec2::splat(CELL - 1.0)),
        Transform::from_xyz(pos.x, pos.y, 12.0),
        PreviewEntity,
    ));
    let mut sprite = Sprite::from_image(building_icon_handle(&building_icons, kind));
    sprite.custom_size = Some(Vec2::splat(CELL + 8.0));
    sprite.color = Color::srgba(1.0, 1.0, 1.0, 0.44);
    commands.spawn((
        sprite,
        Transform::from_xyz(pos.x, pos.y + 4.0, 13.0),
        PreviewEntity,
    ));
}

fn update_build_hover(
    windows: Query<&Window, With<PrimaryWindow>>,
    net: Res<ClientNet>,
    state: Res<SnapshotState>,
    mut hover: ResMut<BuildHover>,
) {
    let Some(screen) = cursor_screen_pos(&windows) else {
        if hover.kind.is_some() {
            hover.kind = None;
        }
        return;
    };
    if screen.y > UI_PANEL_TOP_Y {
        if hover.kind.is_some() {
            hover.kind = None;
        }
        return;
    }

    let Some(race) = selected_race(&state, &net) else {
        if hover.kind.is_some() {
            hover.kind = None;
        }
        return;
    };
    let balance = active_balance(&state);
    let hovered = balance
        .race(race)
        .buildings
        .iter()
        .enumerate()
        .find_map(|(idx, kind)| {
            let center = command_button_center(idx);
            point_in_rect(screen, center, Vec2::splat(54.0)).then_some(*kind)
        });

    if hover.kind != hovered {
        hover.kind = hovered;
    }
}

fn update_world_hover(
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    net: Res<ClientNet>,
    state: Res<SnapshotState>,
    mut hover: ResMut<WorldHover>,
) {
    let hovered = cursor_world(&windows, &camera_query).and_then(|world| {
        let screen = cursor_screen_pos(&windows)?;
        if screen.y <= UI_PANEL_TOP_Y {
            return None;
        }
        state
            .snapshot
            .as_ref()
            .and_then(|snapshot| pick_world_object(snapshot, world, net.team))
    });

    if hover.hovered != hovered {
        hover.hovered = hovered;
    }
}

fn detect_combat_vfx(
    mut commands: Commands,
    net: Res<ClientNet>,
    state: Res<SnapshotState>,
    mut tracker: ResMut<CombatTracker>,
) {
    if !state.is_changed() {
        return;
    }
    let Some(snapshot) = &state.snapshot else {
        tracker.initialized = false;
        tracker.units.clear();
        tracker.buildings.clear();
        tracker.seen_bounty_events.clear();
        return;
    };
    if snapshot.phase != MatchPhase::Playing {
        tracker.initialized = false;
        tracker.units.clear();
        tracker.buildings.clear();
        tracker.seen_bounty_events.clear();
        tracker.castle_health = [0, 0];
        return;
    }
    let balance = active_balance(&state);

    if tracker.initialized {
        tracker
            .seen_bounty_events
            .retain(|id| snapshot.bounty_events.iter().any(|event| event.id == *id));

        for unit in &snapshot.units {
            if !is_unit_visible(snapshot, net.team, unit) {
                continue;
            }
            if let Some(previous) = tracker.units.get(&unit.id) {
                if unit.health < previous.health {
                    let damage = previous.health - unit.health;
                    let pos = unit_world_pos(unit);
                    let config = balance.unit(unit.kind);
                    let attacker = infer_attacker(snapshot, &balance, pos, unit.owner)
                        .map(|attacker| {
                            (
                                unit_world_pos(attacker),
                                balance.unit(attacker.kind).attack_type,
                            )
                        })
                        .unwrap_or((
                            Vec2::new(pos.x - unit.owner.direction() * 38.0, pos.y),
                            config.attack_type,
                        ));
                    spawn_combat_impact(&mut commands, pos, damage, attacker.0, attacker.1, false);
                }
            }
        }

        for building in &snapshot.buildings {
            if !is_building_visible(snapshot, net.team, building) {
                continue;
            }
            if let Some(previous) = tracker.buildings.get(&building.id) {
                if building.health < previous.health {
                    let damage = previous.health - building.health;
                    let pos = building_hit_pos(building);
                    let attacker = infer_attacker(snapshot, &balance, pos, building.owner)
                        .map(|attacker| {
                            (
                                unit_world_pos(attacker),
                                balance.unit(attacker.kind).attack_type,
                            )
                        })
                        .unwrap_or((
                            Vec2::new(pos.x - building.owner.opponent().direction() * 58.0, pos.y),
                            AttackType::Siege,
                        ));
                    spawn_structure_impact(&mut commands, pos, damage, attacker.0, attacker.1);
                }
            }
        }

        for castle in &snapshot.castles {
            if !is_castle_visible(snapshot, net.team, castle.team) {
                continue;
            }
            let slot = castle.team.slot();
            let previous = tracker.castle_health[slot];
            if previous > 0 && castle.health < previous {
                let damage = previous - castle.health;
                let pos = castle_world_pos(castle.team) + Vec2::new(0.0, 52.0);
                let attacker = infer_attacker(snapshot, &balance, pos, castle.team)
                    .map(|attacker| {
                        (
                            unit_world_pos(attacker),
                            balance.unit(attacker.kind).attack_type,
                        )
                    })
                    .unwrap_or((
                        Vec2::new(
                            pos.x - castle.team.opponent().direction() * 52.0,
                            pos.y - 28.0,
                        ),
                        AttackType::Siege,
                    ));
                spawn_combat_impact(&mut commands, pos, damage, attacker.0, attacker.1, true);
            }
        }

        if let Some(team) = net.team {
            for event in &snapshot.bounty_events {
                if event.team == team && !tracker.seen_bounty_events.contains(&event.id) {
                    let pos = sim_pos_to_world(event.pos) + Vec2::new(0.0, 34.0);
                    spawn_bounty_text(&mut commands, pos, event.amount);
                    tracker.seen_bounty_events.insert(event.id);
                }
            }
        }
    }

    tracker.units = snapshot
        .units
        .iter()
        .map(|unit| {
            (
                unit.id,
                TrackedUnit {
                    health: unit.health,
                },
            )
        })
        .collect();
    tracker.buildings = snapshot
        .buildings
        .iter()
        .map(|building| {
            (
                building.id,
                TrackedBuilding {
                    health: building.health,
                },
            )
        })
        .collect();
    tracker.castle_health = [
        snapshot.castles[Team::Left.slot()].health,
        snapshot.castles[Team::Right.slot()].health,
    ];
    tracker.initialized = true;
}

fn update_combat_vfx(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(
        Entity,
        &mut CombatVfx,
        &mut Transform,
        Option<&mut TextFont>,
        Option<&mut TextColor>,
        Option<&mut Sprite>,
    )>,
) {
    for (entity, mut vfx, mut transform, font, text_color, sprite) in &mut query {
        vfx.lifetime -= time.delta_secs();
        if vfx.lifetime <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        let age = 1.0 - (vfx.lifetime / vfx.max_lifetime).clamp(0.0, 1.0);
        transform.translation.x += vfx.velocity.x * time.delta_secs();
        transform.translation.y += vfx.velocity.y * time.delta_secs();
        transform.scale = Vec3::splat(1.0 + age * 0.18);

        if let Some(mut font) = font {
            font.font_size *= 1.0 + time.delta_secs() * 0.24;
        }
        let alpha = (vfx.lifetime / vfx.max_lifetime).clamp(0.0, 1.0);
        if let Some(mut text_color) = text_color {
            text_color.0 = text_color.0.with_alpha(alpha);
        }
        if let Some(mut sprite) = sprite {
            sprite.color = sprite.color.with_alpha(alpha * 0.82);
        }
    }
}

fn animate_grass(time: Res<Time>, mut query: Query<(&GrassBlade, &mut Transform)>) {
    let t = time.elapsed_secs();
    for (blade, mut transform) in &mut query {
        let wave = (t * 1.35 + blade.phase).sin();
        transform.translation.x = blade.base_pos.x + wave * blade.sway;
        transform.translation.y = blade.base_pos.y;
        transform.rotation = Quat::from_rotation_z(blade.base_rotation + wave * 0.055);
    }
}

fn camera_controls(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    windows: Query<&Window, With<PrimaryWindow>>,
    net: Res<ClientNet>,
    mut camera_home: ResMut<CameraHome>,
    mut camera_query: Query<(&mut Transform, &mut Projection), With<Camera2d>>,
) {
    let Ok((mut transform, mut projection)) = camera_query.single_mut() else {
        return;
    };
    let scale = if let Projection::Orthographic(orthographic) = projection.as_ref() {
        orthographic.scale
    } else {
        1.0
    };
    if let Some(team) = net.team {
        if camera_home.initialized_for != Some(team) {
            transform.translation.x = home_camera_x(team, scale, windows.single().ok());
            transform.translation.y = 0.0;
            camera_home.initialized_for = Some(team);
        }
    }

    let mut direction = Vec2::ZERO;
    if keys.pressed(KeyCode::ArrowLeft) || keys.pressed(KeyCode::KeyA) {
        direction.x -= 1.0;
    }
    if keys.pressed(KeyCode::ArrowRight) || keys.pressed(KeyCode::KeyD) {
        direction.x += 1.0;
    }
    if keys.pressed(KeyCode::ArrowUp) || keys.pressed(KeyCode::KeyW) {
        direction.y += 1.0;
    }
    if keys.pressed(KeyCode::ArrowDown) || keys.pressed(KeyCode::KeyS) {
        direction.y -= 1.0;
    }
    if let Ok(window) = windows.single() {
        if let Some(cursor) = window.cursor_position() {
            let edge = 18.0;
            if cursor.x <= edge {
                direction.x -= 1.0;
            } else if cursor.x >= window.width() - edge {
                direction.x += 1.0;
            }
            if cursor.y <= edge {
                direction.y += 1.0;
            } else if cursor.y >= window.height() - edge
                && cursor_screen_pos(&windows).is_some_and(|screen| screen.y > UI_PANEL_TOP_Y)
            {
                direction.y -= 1.0;
            }
        }
    }
    if direction != Vec2::ZERO {
        let speed = 650.0 * scale * time.delta_secs();
        transform.translation.x += direction.normalize().x * speed;
        transform.translation.y += direction.normalize().y * speed;
        clamp_camera(&mut transform, scale, windows.single().ok());
    }
    if let Projection::Orthographic(orthographic) = projection.as_mut() {
        if keys.just_pressed(KeyCode::Equal) || keys.just_pressed(KeyCode::NumpadAdd) {
            orthographic.scale = (orthographic.scale * 0.88).clamp(0.55, 1.25);
        }
        if keys.just_pressed(KeyCode::Minus) || keys.just_pressed(KeyCode::NumpadSubtract) {
            orthographic.scale = (orthographic.scale * 1.12).clamp(0.55, 1.25);
        }
        if keys.just_pressed(KeyCode::Home) {
            orthographic.scale = 1.0;
            transform.translation.x = net
                .team
                .map(|team| home_camera_x(team, orthographic.scale, windows.single().ok()))
                .unwrap_or(0.0);
            transform.translation.y = 0.0;
        }
        clamp_camera(&mut transform, orthographic.scale, windows.single().ok());
    }
}

fn home_camera_x(team: Team, scale: f32, window: Option<&Window>) -> f32 {
    let castle_x = lane_to_world(team.castle_pos());
    let desired = castle_x + team.direction() * 290.0 * scale;
    camera_x_bounds(scale, window)
        .map(|(min_x, max_x)| desired.clamp(min_x, max_x))
        .unwrap_or(desired)
}

fn clamp_camera(transform: &mut Transform, scale: f32, window: Option<&Window>) {
    if let Some((min_x, max_x)) = camera_x_bounds(scale, window) {
        transform.translation.x = transform.translation.x.clamp(min_x, max_x);
    }
    let y_bound =
        (MAP_H * 0.5 - window.map(|w| w.height() * scale * 0.5).unwrap_or(360.0)).max(0.0) + 36.0;
    transform.translation.y = transform.translation.y.clamp(-y_bound, y_bound);
}

fn camera_x_bounds(scale: f32, window: Option<&Window>) -> Option<(f32, f32)> {
    let half_map = MAP_W * 0.5;
    let half_view = window.map(|w| w.width() * scale * 0.5)?;
    let bound = (half_map - half_view).max(0.0);
    Some((-bound, bound))
}

fn update_fog_memory(mut fog: ResMut<FogMemory>, state: Res<SnapshotState>, net: Res<ClientNet>) {
    if fog.team != net.team {
        fog.team = net.team;
        fog.explored.fill(false);
    }
    if net.team.is_none() {
        fog.explored.fill(true);
        return;
    }
    if !state.is_changed() {
        return;
    }
    let (Some(snapshot), Some(team)) = (&state.snapshot, net.team) else {
        return;
    };

    for row in 0..FOG_ROWS {
        for col in 0..FOG_COLUMNS {
            let pos = fog_cell_center(col, row);
            if world_reveal_strength(snapshot, team, pos) > 0.05 {
                fog.explored[fog_index(col, row)] = true;
            }
        }
    }
}

fn redraw_scene(
    mut commands: Commands,
    scene_query: Query<Entity, With<SceneEntity>>,
    state: Res<SnapshotState>,
    net: Res<ClientNet>,
    fog: Res<FogMemory>,
    world_selection: Res<WorldSelection>,
    world_hover: Res<WorldHover>,
    unit_assets: Res<UnitSpriteAssets>,
    building_icons: Res<BuildingIconAssets>,
) {
    if !state.is_changed()
        && !fog.is_changed()
        && !world_selection.is_changed()
        && !world_hover.is_changed()
    {
        return;
    }
    for entity in &scene_query {
        commands.entity(entity).despawn();
    }

    spawn_static_board(&mut commands, net.team);

    let Some(snapshot) = &state.snapshot else {
        return;
    };
    let balance = active_balance(&state);

    for building in &snapshot.buildings {
        if !is_building_visible(snapshot, net.team, building) {
            if is_enemy_building_scouted(snapshot, &fog, net.team, building) {
                spawn_building_silhouette(&mut commands, building);
            }
            continue;
        }
        let highlighted = world_selection.selected == Some(SelectedObject::Building(building.id))
            || world_hover.hovered == Some(SelectedObject::Building(building.id));
        spawn_building(
            &mut commands,
            building,
            &balance,
            &building_icons,
            highlighted,
        );
    }
    for unit in &snapshot.units {
        if !is_unit_visible(snapshot, net.team, unit) {
            continue;
        }
        let selected = world_selection.selected == Some(SelectedObject::Unit(unit.id));
        spawn_unit(
            &mut commands,
            unit,
            &balance,
            &unit_assets,
            selected,
            snapshot.tick,
        );
    }
    for castle in &snapshot.castles {
        if !is_castle_visible(snapshot, net.team, castle.team) {
            continue;
        }
        spawn_castle(
            &mut commands,
            castle,
            team_race(snapshot, castle.team).unwrap_or(RaceKind::Vanguard),
            &building_icons,
            world_selection.selected == Some(SelectedObject::Castle(castle.team)),
        );
    }
    spawn_fog_of_war(&mut commands, snapshot, &fog, net.team);
}

fn redraw_game_ui(
    mut commands: Commands,
    ui_query: Query<Entity, With<UiEntity>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Transform, &Projection), With<Camera2d>>,
    state: Res<SnapshotState>,
    net: Res<ClientNet>,
    fog: Res<FogMemory>,
    selection: Res<BuildSelection>,
    hover: Res<BuildHover>,
    world_selection: Res<WorldSelection>,
    _world_hover: Res<WorldHover>,
    building_icons: Res<BuildingIconAssets>,
) {
    for entity in &ui_query {
        commands.entity(entity).despawn();
    }

    let balance = active_balance(&state);
    let (phase_line, _economy_line, _message_line) = ui_summary(&state, &net);
    let selected_race = selected_race(&state, &net);
    let race_popup_open = race_selection_popup_open(&state, &net);
    spawn_top_hud(&mut commands, &state, &net, &balance, &phase_line);

    spawn_bottom_console(&mut commands);

    if let Some(snapshot) = &state.snapshot {
        spawn_minimap(
            &mut commands,
            snapshot,
            &fog,
            net.team,
            camera_query.single().ok(),
            windows.single().ok(),
        );
    }

    let selected_details =
        selected_object_details(&state, &world_selection, selection.kind, net.team).or_else(|| {
            selection
                .kind
                .map(|kind| selected_build_details(&balance, kind))
        });
    spawn_context_bay(
        &mut commands,
        selected_details.as_deref(),
        race_popup_open,
        world_selection.selected.is_some() || selection.kind.is_some(),
    );

    if !race_popup_open {
        if let Some(race) = selected_race {
            spawn_command_card(
                &mut commands,
                &balance,
                &building_icons,
                &state,
                &net,
                race,
                selection.kind,
                hover.kind,
            );
        }
    }
    if !race_popup_open {
        if let Some(kind) = hover.kind {
            spawn_build_tooltip(&mut commands, &balance, kind);
        }
    }

    if let Some(snapshot) = &state.snapshot {
        if let Some(winner) = snapshot.winner {
            let result = if Some(winner) == net.team {
                "VICTORY"
            } else {
                "DEFEAT"
            };
            spawn_ui_panel(
                &mut commands,
                Vec2::new(0.0, 92.0),
                Vec2::new(320.0, 112.0),
                48.0,
            );
            spawn_ui_label(
                &mut commands,
                result,
                Vec2::new(0.0, 118.0),
                32.0,
                TEXT_GOLD,
                51.0,
                Anchor::CENTER,
                Justify::Center,
            );
            spawn_ui_label(
                &mut commands,
                "Press R on both clients for rematch",
                Vec2::new(0.0, 76.0),
                13.0,
                TEXT_PARCHMENT,
                51.0,
                Anchor::CENTER,
                Justify::Center,
            );
        }
    }

    if race_popup_open {
        spawn_race_selection_popup(&mut commands, &balance, selected_race);
    }
}

fn spawn_top_hud(
    commands: &mut Commands,
    state: &SnapshotState,
    net: &ClientNet,
    balance: &BalanceConfig,
    phase_line: &str,
) {
    spawn_ui_panel(
        commands,
        Vec2::new(0.0, TOP_BAR_Y + 5.0),
        Vec2::new(1048.0, 36.0),
        40.0,
    );
    let race = selected_race(state, net);
    let left_label = race
        .map(|race| format!("{}  {}", balance.race(race).name, net.player_name))
        .unwrap_or_else(|| format!("Commander  {}", net.player_name));
    spawn_ui_label(
        commands,
        &left_label,
        Vec2::new(-502.0, TOP_BAR_Y + 10.0),
        13.0,
        TEXT_PARCHMENT,
        45.0,
        Anchor::TOP_LEFT,
        Justify::Left,
    );
    spawn_ui_label(
        commands,
        phase_line,
        Vec2::new(0.0, TOP_BAR_Y + 10.0),
        14.0,
        TEXT_GOLD,
        45.0,
        Anchor::CENTER,
        Justify::Center,
    );

    if let (Some(snapshot), Some(player_id)) = (&state.snapshot, net.player_id) {
        if let Some(player) = current_player(snapshot, player_id) {
            let econ = &snapshot.economies[player.team.slot()];
            let castle = &snapshot.castles[player.team.slot()];
            spawn_resource_chip(
                commands,
                Vec2::new(206.0, TOP_BAR_Y + 6.0),
                "GOLD",
                &econ.gold.to_string(),
                TEXT_GOLD,
            );
            spawn_resource_chip(
                commands,
                Vec2::new(298.0, TOP_BAR_Y + 6.0),
                "INC",
                &econ.income.to_string(),
                Color::srgb(0.58, 0.92, 0.62),
            );
            spawn_ui_label(
                commands,
                "CASTLE",
                Vec2::new(376.0, TOP_BAR_Y + 12.0),
                8.5,
                Color::srgb(0.62, 0.58, 0.48),
                45.0,
                Anchor::TOP_LEFT,
                Justify::Left,
            );
            spawn_value_bar(
                commands,
                Vec2::new(438.0, TOP_BAR_Y + 7.0),
                Vec2::new(104.0, 8.0),
                castle.health.max(0) as f32 / castle.max_health.max(1) as f32,
                Color::srgb(0.18, 0.82, 0.36),
                45.0,
            );
            spawn_ui_label(
                commands,
                &format!("{}/{}", castle.health.max(0), castle.max_health),
                Vec2::new(500.0, TOP_BAR_Y + 6.0),
                10.0,
                TEXT_PARCHMENT,
                46.0,
                Anchor::CENTER_RIGHT,
                Justify::Right,
            );
        }
    } else {
        spawn_ui_label(
            commands,
            "Press Enter to join",
            Vec2::new(500.0, TOP_BAR_Y + 7.0),
            11.0,
            TEXT_GOLD,
            45.0,
            Anchor::CENTER_RIGHT,
            Justify::Right,
        );
    }
}

fn spawn_resource_chip(commands: &mut Commands, pos: Vec2, label: &str, value: &str, color: Color) {
    spawn_ui_rect(
        commands,
        pos,
        Vec2::new(82.0, 20.0),
        Color::srgba(0.02, 0.018, 0.014, 0.82),
        44.0,
    );
    spawn_ui_label(
        commands,
        label,
        Vec2::new(pos.x - 34.0, pos.y + 4.0),
        8.0,
        Color::srgb(0.57, 0.53, 0.43),
        46.0,
        Anchor::TOP_LEFT,
        Justify::Left,
    );
    spawn_ui_label(
        commands,
        value,
        Vec2::new(pos.x + 33.0, pos.y + 4.0),
        12.0,
        color,
        46.0,
        Anchor::TOP_RIGHT,
        Justify::Right,
    );
}

fn spawn_bottom_console(commands: &mut Commands) {
    spawn_ui_panel(
        commands,
        Vec2::new(0.0, UI_PANEL_Y),
        Vec2::new(1048.0, 188.0),
        40.0,
    );
    spawn_ui_rect(
        commands,
        Vec2::new(-300.0, UI_PANEL_Y + 4.0),
        Vec2::new(4.0, 164.0),
        PANEL_EDGE,
        44.0,
    );
    spawn_ui_rect(
        commands,
        Vec2::new(178.0, UI_PANEL_Y + 4.0),
        Vec2::new(4.0, 164.0),
        PANEL_EDGE,
        44.0,
    );
}

fn spawn_context_bay(
    commands: &mut Commands,
    details: Option<&str>,
    race_popup_open: bool,
    has_selection: bool,
) {
    if race_popup_open {
        return;
    }
    let Some(details) = details.filter(|_| has_selection) else {
        return;
    };
    spawn_ui_label(
        commands,
        "Selection",
        Vec2::new(-274.0, UI_PANEL_Y + 58.0),
        13.0,
        TEXT_GOLD,
        46.0,
        Anchor::TOP_LEFT,
        Justify::Left,
    );
    spawn_ui_label(
        commands,
        details,
        Vec2::new(-274.0, UI_PANEL_Y + 32.0),
        11.2,
        TEXT_PARCHMENT,
        46.0,
        Anchor::TOP_LEFT,
        Justify::Left,
    );
}

fn spawn_command_card(
    commands: &mut Commands,
    balance: &BalanceConfig,
    building_icons: &BuildingIconAssets,
    state: &SnapshotState,
    net: &ClientNet,
    race: RaceKind,
    selected_kind: Option<BuildingKind>,
    hovered_kind: Option<BuildingKind>,
) {
    let local_gold = local_gold(state, net);
    for (idx, kind) in balance.race(race).buildings.iter().enumerate() {
        let config = balance.building(*kind);
        let center = command_button_center(idx);
        let selected = selected_kind == Some(*kind);
        let hovered = hovered_kind == Some(*kind);
        let affordable = local_gold.is_none_or(|gold| gold >= config.cost);
        let color = match (affordable, hovered) {
            (false, _) => Color::srgba(0.045, 0.040, 0.038, 0.96),
            (true, true) => Color::srgba(0.24, 0.19, 0.10, 0.98),
            (true, false) => Color::srgba(0.08, 0.07, 0.055, 0.96),
        };
        spawn_ui_button(commands, center, Vec2::splat(54.0), color, selected);
        spawn_ui_image(
            commands,
            building_icon_handle(building_icons, *kind),
            center,
            Vec2::splat(if hovered { 50.0 } else { 46.0 }),
            if affordable { 46.2 } else { 45.8 },
        );
        if !affordable {
            spawn_ui_rect(
                commands,
                center,
                Vec2::splat(48.0),
                Color::srgba(0.28, 0.02, 0.02, 0.36),
                47.0,
            );
        }
        if let Some(unit_kind) = config.spawned_unit {
            let unit = balance.unit(unit_kind);
            spawn_ui_rect(
                commands,
                Vec2::new(center.x - 14.0, center.y + 15.0),
                Vec2::new(18.0, 6.0),
                attack_type_color(unit.attack_type),
                47.0,
            );
            spawn_ui_rect(
                commands,
                Vec2::new(center.x + 14.0, center.y + 15.0),
                Vec2::new(18.0, 6.0),
                armor_type_color(unit.armor_type),
                47.0,
            );
        }
        spawn_ui_label(
            commands,
            &command_button_badge(balance, *kind),
            Vec2::new(center.x, center.y - 18.0),
            8.2,
            if affordable {
                Color::srgb(0.98, 0.90, 0.66)
            } else {
                Color::srgb(0.82, 0.34, 0.28)
            },
            48.0,
            Anchor::CENTER,
            Justify::Center,
        );
    }
    let cancel_center = command_button_center(8);
    spawn_ui_button(
        commands,
        cancel_center,
        Vec2::splat(54.0),
        Color::srgba(0.055, 0.050, 0.045, 0.92),
        selected_kind.is_none(),
    );
    spawn_ui_label(
        commands,
        "X",
        Vec2::new(cancel_center.x, cancel_center.y + 4.0),
        22.0,
        Color::srgb(0.55, 0.48, 0.38),
        47.0,
        Anchor::CENTER,
        Justify::Center,
    );
    spawn_ui_label(
        commands,
        "ESC",
        Vec2::new(cancel_center.x, cancel_center.y - 18.0),
        8.0,
        Color::srgb(0.58, 0.54, 0.46),
        47.0,
        Anchor::CENTER,
        Justify::Center,
    );
}

fn format_match_time(secs: f32) -> String {
    let secs = secs.max(0.0).floor() as u32;
    format!("{}:{:02}", secs / 60, secs % 60)
}

fn cursor_world(
    windows: &Query<&Window, With<PrimaryWindow>>,
    camera_query: &Query<(&Camera, &GlobalTransform), With<Camera2d>>,
) -> Option<Vec2> {
    let window = windows.single().ok()?;
    let cursor = window.cursor_position()?;
    let (camera, transform) = camera_query.single().ok()?;
    camera.viewport_to_world_2d(transform, cursor).ok()
}

fn cursor_screen_pos(windows: &Query<&Window, With<PrimaryWindow>>) -> Option<Vec2> {
    let window = windows.single().ok()?;
    let cursor = window.cursor_position()?;
    Some(Vec2::new(
        cursor.x - window.width() * 0.5,
        window.height() * 0.5 - cursor.y,
    ))
}

fn point_in_rect(point: Vec2, center: Vec2, size: Vec2) -> bool {
    let half = size * 0.5;
    point.x >= center.x - half.x
        && point.x <= center.x + half.x
        && point.y >= center.y - half.y
        && point.y <= center.y + half.y
}

fn command_button_center(idx: usize) -> Vec2 {
    let col = idx % 3;
    let row = idx / 3;
    Vec2::new(
        COMMAND_GRID_ORIGIN.x + col as f32 * 58.0,
        COMMAND_GRID_ORIGIN.y - row as f32 * 50.0,
    )
}

fn race_button_center(idx: usize) -> Vec2 {
    Vec2::new(-184.0 + idx as f32 * 184.0, 42.0)
}

fn race_button_size() -> Vec2 {
    Vec2::new(156.0, 132.0)
}

fn ready_button_center() -> Vec2 {
    Vec2::new(0.0, -96.0)
}

fn ready_button_size() -> Vec2 {
    Vec2::new(178.0, 44.0)
}

fn pick_world_object(
    snapshot: &MatchSnapshot,
    world: Vec2,
    viewer_team: Option<Team>,
) -> Option<SelectedObject> {
    let unit_pick = snapshot
        .units
        .iter()
        .filter(|unit| is_unit_visible(snapshot, viewer_team, unit))
        .map(|unit| {
            let pos = unit_world_pos(unit);
            (unit.id, pos.distance(world))
        })
        .filter(|(_, distance)| *distance <= 22.0)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(id, _)| SelectedObject::Unit(id));
    if unit_pick.is_some() {
        return unit_pick;
    }

    for building in &snapshot.buildings {
        if !is_building_visible(snapshot, viewer_team, building) {
            continue;
        }
        let center = cell_to_world(building.owner, building.lane, building.zone, building.cell);
        if point_in_rect(world, center, Vec2::splat(CELL)) {
            return Some(SelectedObject::Building(building.id));
        }
    }

    for castle in &snapshot.castles {
        if !is_castle_visible(snapshot, viewer_team, castle.team) {
            continue;
        }
        let center = castle_world_pos(castle.team) + Vec2::new(0.0, 24.0);
        if point_in_rect(world, center, Vec2::new(116.0, 140.0)) {
            return Some(SelectedObject::Castle(castle.team));
        }
    }

    None
}

fn selected_object_details(
    state: &SnapshotState,
    selection: &WorldSelection,
    build_selection: Option<BuildingKind>,
    viewer_team: Option<Team>,
) -> Option<String> {
    let snapshot = state.snapshot.as_ref()?;
    let balance = &state.balance;
    match selection.selected? {
        SelectedObject::Unit(id) => {
            let unit = snapshot.units.iter().find(|unit| unit.id == id)?;
            if !is_unit_visible(snapshot, viewer_team, unit) {
                return None;
            }
            let config = balance.unit(unit.kind);
            Some(format!(
                "{} {:?} {}\nHP {}/{}   {} {} dmg\nMv {:.1}   AS {:.2}/s   AtkR {:.1}\nArmor {}   Bounty {}g",
                config.name,
                unit.owner,
                config.attack_mode.label(),
                unit.health.max(0),
                config.max_health,
                config.attack_type.label(),
                damage_range_text(config.damage, config.damage_variance),
                config.speed,
                attacks_per_second(config.attack_interval),
                config.attack_range,
                config.armor_type.label(),
                config.bounty
            ))
        }
        SelectedObject::Building(id) => {
            let building = snapshot
                .buildings
                .iter()
                .find(|building| building.id == id)?;
            if !is_building_visible(snapshot, viewer_team, building) {
                return None;
            }
            let config = balance.building(building.kind);
            if let Some(unit_kind) = config.spawned_unit {
                let unit = balance.unit(unit_kind);
                Some(format!(
                    "{} {:?} {:?} {:?}\nHP {}/{}   Next {} in {:.1}s\nProduces: {}  HP {}\nDamage {} {}   Armor {}\nMv {:.1}   AtkR {:.1}   AS {:.2}/s",
                    config.name,
                    building.owner,
                    building.lane,
                    building.zone,
                    building.health.max(0),
                    building.max_health,
                    unit.name,
                    building.spawn_timer.max(0.0),
                    unit.name,
                    unit.max_health,
                    damage_range_text(unit.damage, unit.damage_variance),
                    unit.attack_type.label(),
                    unit.armor_type.label(),
                    unit.speed,
                    unit.attack_range,
                    attacks_per_second(unit.attack_interval),
                ))
            } else {
                Some(format!(
                    "{} {:?} {:?} {:?}\nHP {}/{}   Economy building\nIncome +{} every income tick\nProduces no unit wave.",
                    config.name,
                    building.owner,
                    building.lane,
                    building.zone,
                    building.health.max(0),
                    building.max_health,
                    config.income_bonus
                ))
            }
        }
        SelectedObject::Castle(team) => {
            if !is_castle_visible(snapshot, viewer_team, team) {
                return None;
            }
            let castle = &snapshot.castles[team.slot()];
            let race = team_race(snapshot, team)
                .map(|race| balance.race(race).name.as_str())
                .unwrap_or("Unknown");
            Some(format!(
                "{team:?} Castle\nRace: {}   Vision {:.0}\nHP {}/{}   Armor {}",
                race,
                REVEAL_CASTLE_RADIUS,
                castle.health.max(0),
                castle.max_health,
                castle.armor_type.label()
            ))
        }
        SelectedObject::Cell(team, lane, zone, cell) => {
            if let Some(build_selection) = build_selection {
                let config = balance.building(build_selection);
                Some(format!(
                    "{team:?} {lane:?} {zone:?} cell {},{}\nSelected: {}\n{}",
                    cell.x,
                    cell.y,
                    config.name,
                    selected_build_details(balance, build_selection)
                ))
            } else {
                Some(format!(
                    "{team:?} {lane:?} {zone:?} cell {},{}",
                    cell.x, cell.y
                ))
            }
        }
    }
}

fn selected_build_details(balance: &BalanceConfig, kind: BuildingKind) -> String {
    let config = balance.building(kind);
    if let Some(unit_kind) = config.spawned_unit {
        let unit = balance.unit(unit_kind);
        format!(
            "{}\n{}g\n{} {}  {}g bounty\nR {:.1} AS {:.1}",
            config.name,
            config.cost,
            unit.attack_mode.short_label(),
            unit.attack_type.short_label(),
            unit.bounty,
            unit.attack_range,
            attacks_per_second(unit.attack_interval)
        )
    } else {
        format!(
            "{}\n{}g\nIncome +{}",
            config.name, config.cost, config.income_bonus
        )
    }
}

fn command_button_badge(balance: &BalanceConfig, kind: BuildingKind) -> String {
    let config = balance.building(kind);
    if let Some(unit_kind) = config.spawned_unit {
        let unit = balance.unit(unit_kind);
        format!("{}  {}g", unit.attack_mode.short_label(), config.cost)
    } else {
        format!("+{}  {}g", config.income_bonus, config.cost)
    }
}

fn spawn_build_tooltip(commands: &mut Commands, balance: &BalanceConfig, kind: BuildingKind) {
    let pos = Vec2::new(354.0, UI_PANEL_Y + 116.0);
    spawn_ui_panel(commands, pos, Vec2::new(326.0, 120.0), 49.0);
    spawn_ui_label(
        commands,
        &building_tooltip_text(balance, kind),
        Vec2::new(pos.x - 148.0, pos.y + 46.0),
        11.0,
        TEXT_PARCHMENT,
        53.0,
        Anchor::TOP_LEFT,
        Justify::Left,
    );
}

fn building_tooltip_text(balance: &BalanceConfig, kind: BuildingKind) -> String {
    let config = balance.building(kind);
    if let Some(unit_kind) = config.spawned_unit {
        let unit = balance.unit(unit_kind);
        format!(
            "{}\nCost: {} gold   Spawns every {:.1}s\nProduces: {} ({})   Bounty: {}g\nDamage: {} {}   Armor: {}\nMove: {:.1}   AtkR: {:.1}   AS: {:.2}/s",
            config.name,
            config.cost,
            config.spawn_interval.unwrap_or_default(),
            unit.name,
            unit.attack_mode.label(),
            unit.bounty,
            damage_range_text(unit.damage, unit.damage_variance),
            unit.attack_type.label(),
            unit.armor_type.label(),
            unit.speed,
            unit.attack_range,
            attacks_per_second(unit.attack_interval),
        )
    } else {
        format!(
            "{}\nCost: {} gold\nEconomy building\nIncome: +{} every income tick\nProduces no units.",
            config.name, config.cost, config.income_bonus
        )
    }
}

fn spawn_race_selection_popup(
    commands: &mut Commands,
    balance: &BalanceConfig,
    selected_race: Option<RaceKind>,
) {
    spawn_ui_rect(
        commands,
        Vec2::ZERO,
        Vec2::new(1100.0, 720.0),
        Color::srgba(0.01, 0.008, 0.006, 0.58),
        54.0,
    );
    spawn_ui_panel(
        commands,
        Vec2::new(0.0, 42.0),
        Vec2::new(650.0, 320.0),
        55.0,
    );
    spawn_ui_label(
        commands,
        "Choose Your Race",
        Vec2::new(0.0, 172.0),
        24.0,
        TEXT_GOLD,
        61.0,
        Anchor::CENTER,
        Justify::Center,
    );
    spawn_ui_label(
        commands,
        "Pick a race, then confirm when you are ready.",
        Vec2::new(0.0, 142.0),
        12.0,
        Color::srgb(0.74, 0.69, 0.56),
        61.0,
        Anchor::CENTER,
        Justify::Center,
    );

    for (idx, race) in RaceKind::ALL.iter().enumerate() {
        let center = race_button_center(idx);
        let size = race_button_size();
        let selected = selected_race == Some(*race);
        spawn_ui_button_layer(
            commands,
            center,
            size,
            race_card_color(*race),
            selected,
            58.0,
        );
        spawn_ui_rect(
            commands,
            Vec2::new(center.x, center.y + 34.0),
            Vec2::new(size.x - 28.0, 38.0),
            race_accent_color(*race).with_alpha(0.72),
            60.0,
        );
        spawn_ui_label(
            commands,
            balance.race(*race).name.as_str(),
            Vec2::new(center.x, center.y + 42.0),
            17.0,
            TEXT_PARCHMENT,
            62.0,
            Anchor::CENTER,
            Justify::Center,
        );
        spawn_ui_label(
            commands,
            &race_card_summary(balance, *race),
            Vec2::new(center.x - 60.0, center.y + 6.0),
            11.0,
            Color::srgb(0.91, 0.84, 0.66),
            62.0,
            Anchor::TOP_LEFT,
            Justify::Left,
        );
    }

    let ready_enabled = selected_race.is_some();
    let ready_color = if ready_enabled {
        Color::srgba(0.25, 0.17, 0.06, 0.98)
    } else {
        Color::srgba(0.10, 0.09, 0.08, 0.92)
    };
    spawn_ui_button_layer(
        commands,
        ready_button_center(),
        ready_button_size(),
        ready_color,
        ready_enabled,
        58.0,
    );
    spawn_ui_label(
        commands,
        if ready_enabled {
            "Ready"
        } else {
            "Select a Race"
        },
        Vec2::new(ready_button_center().x, ready_button_center().y - 1.0),
        16.0,
        if ready_enabled {
            TEXT_GOLD
        } else {
            Color::srgb(0.55, 0.52, 0.44)
        },
        62.0,
        Anchor::CENTER,
        Justify::Center,
    );
}

fn race_card_summary(balance: &BalanceConfig, race: RaceKind) -> String {
    let config = balance.race(race);
    format!(
        "Castle HP: {}\nArmor: {}\nBuildings: {}\nUnit halls, economy, siege",
        config.castle_health,
        config.castle_armor.label(),
        config.buildings.len()
    )
}

fn race_card_color(race: RaceKind) -> Color {
    match race {
        RaceKind::Vanguard => Color::srgba(0.08, 0.10, 0.13, 0.98),
        RaceKind::Grove => Color::srgba(0.07, 0.13, 0.08, 0.98),
        RaceKind::Ember => Color::srgba(0.15, 0.07, 0.045, 0.98),
    }
}

fn race_accent_color(race: RaceKind) -> Color {
    match race {
        RaceKind::Vanguard => Color::srgb(0.36, 0.54, 0.78),
        RaceKind::Grove => Color::srgb(0.36, 0.68, 0.32),
        RaceKind::Ember => Color::srgb(0.88, 0.34, 0.17),
    }
}

fn spawn_minimap(
    commands: &mut Commands,
    snapshot: &MatchSnapshot,
    fog: &FogMemory,
    viewer_team: Option<Team>,
    camera: Option<(&Transform, &Projection)>,
    window: Option<&Window>,
) {
    spawn_ui_panel(
        commands,
        MINIMAP_CENTER,
        MINIMAP_SIZE + Vec2::new(12.0, 12.0),
        48.0,
    );
    spawn_ui_rect(
        commands,
        MINIMAP_CENTER,
        MINIMAP_SIZE,
        Color::srgba(0.025, 0.028, 0.022, 0.96),
        50.0,
    );
    spawn_ui_rect(
        commands,
        MINIMAP_CENTER,
        MINIMAP_SIZE - Vec2::new(10.0, 10.0),
        Color::srgba(0.16, 0.30, 0.12, 0.35),
        50.5,
    );
    for lane in Lane::ALL {
        let lane_pos = minimap_world_to_ui(Vec2::new(0.0, lane_world_y(lane)));
        spawn_ui_rect(
            commands,
            Vec2::new(MINIMAP_CENTER.x, lane_pos.y),
            Vec2::new(MINIMAP_SIZE.x - 14.0, 3.0),
            Color::srgba(0.62, 0.50, 0.28, 0.58),
            51.0,
        );
    }
    spawn_minimap_fog(commands, snapshot, fog, viewer_team);

    for castle in &snapshot.castles {
        if !is_castle_visible(snapshot, viewer_team, castle.team) {
            continue;
        }
        let pos = minimap_world_to_ui(castle_world_pos(castle.team));
        spawn_ui_rect(
            commands,
            pos,
            Vec2::splat(9.0),
            team_minimap_color(castle.team, viewer_team),
            53.0,
        );
    }
    for building in &snapshot.buildings {
        if !is_building_visible(snapshot, viewer_team, building) {
            if is_enemy_building_scouted(snapshot, fog, viewer_team, building) {
                let pos = minimap_world_to_ui(cell_to_world(
                    building.owner,
                    building.lane,
                    building.zone,
                    building.cell,
                ));
                spawn_ui_rect(
                    commands,
                    pos,
                    Vec2::splat(4.0),
                    Color::srgba(0.63, 0.59, 0.48, 0.46),
                    53.0,
                );
            }
            continue;
        }
        let pos = minimap_world_to_ui(cell_to_world(
            building.owner,
            building.lane,
            building.zone,
            building.cell,
        ));
        spawn_ui_rect(
            commands,
            pos,
            Vec2::splat(4.0),
            team_minimap_color(building.owner, viewer_team),
            54.0,
        );
    }
    for unit in &snapshot.units {
        if !is_unit_visible(snapshot, viewer_team, unit) {
            continue;
        }
        let pos = minimap_world_to_ui(unit_world_pos(unit));
        spawn_ui_rect(
            commands,
            pos,
            Vec2::splat(3.0),
            team_minimap_color(unit.owner, viewer_team),
            55.0,
        );
    }

    if let (Some((camera_transform, projection)), Some(window)) = (camera, window) {
        let scale = if let Projection::Orthographic(orthographic) = projection {
            orthographic.scale
        } else {
            1.0
        };
        let view_size = Vec2::new(window.width() * scale, window.height() * scale);
        let center = minimap_world_to_ui(camera_transform.translation.truncate());
        let size = Vec2::new(
            view_size.x / MAP_W * MINIMAP_SIZE.x,
            view_size.y / MAP_H * MINIMAP_SIZE.y,
        )
        .clamp(Vec2::new(10.0, 8.0), MINIMAP_SIZE);
        spawn_ui_rect(
            commands,
            center,
            size,
            Color::srgba(0.95, 0.82, 0.38, 0.22),
            56.0,
        );
        spawn_ui_rect(
            commands,
            Vec2::new(center.x, center.y + size.y * 0.5),
            Vec2::new(size.x, 2.0),
            TEXT_GOLD,
            57.0,
        );
        spawn_ui_rect(
            commands,
            Vec2::new(center.x, center.y - size.y * 0.5),
            Vec2::new(size.x, 2.0),
            TEXT_GOLD,
            57.0,
        );
        spawn_ui_rect(
            commands,
            Vec2::new(center.x - size.x * 0.5, center.y),
            Vec2::new(2.0, size.y),
            TEXT_GOLD,
            57.0,
        );
        spawn_ui_rect(
            commands,
            Vec2::new(center.x + size.x * 0.5, center.y),
            Vec2::new(2.0, size.y),
            TEXT_GOLD,
            57.0,
        );
    }
}

fn spawn_minimap_fog(
    commands: &mut Commands,
    snapshot: &MatchSnapshot,
    fog: &FogMemory,
    viewer_team: Option<Team>,
) {
    let Some(team) = viewer_team else {
        return;
    };
    let tile_size = Vec2::new(
        MINIMAP_SIZE.x / FOG_COLUMNS as f32 + 0.5,
        MINIMAP_SIZE.y / FOG_ROWS as f32 + 0.5,
    );
    for row in 0..FOG_ROWS {
        for col in 0..FOG_COLUMNS {
            let world = fog_cell_center(col, row);
            let reveal = world_reveal_strength(snapshot, team, world);
            let explored = fog.explored[fog_index(col, row)];
            let alpha = if reveal > 0.0 {
                0.28 * (1.0 - reveal)
            } else if explored {
                0.42
            } else {
                0.82
            };
            if alpha <= 0.02 {
                continue;
            }
            spawn_ui_rect(
                commands,
                minimap_world_to_ui(world),
                tile_size,
                Color::srgba(0.0, 0.0, 0.0, alpha),
                52.0,
            );
        }
    }
}

fn minimap_world_to_ui(pos: Vec2) -> Vec2 {
    Vec2::new(
        MINIMAP_CENTER.x + (pos.x / MAP_W).clamp(-0.5, 0.5) * MINIMAP_SIZE.x,
        MINIMAP_CENTER.y + (pos.y / MAP_H).clamp(-0.5, 0.5) * MINIMAP_SIZE.y,
    )
}

fn team_minimap_color(team: Team, viewer_team: Option<Team>) -> Color {
    if Some(team) == viewer_team {
        Color::srgb(0.24, 0.95, 0.38)
    } else {
        Color::srgb(0.96, 0.22, 0.18)
    }
}

fn attacks_per_second(attack_interval: f32) -> f32 {
    if attack_interval <= f32::EPSILON {
        return 0.0;
    }
    1.0 / attack_interval
}

fn damage_range_text(midpoint: i32, variance: f32) -> String {
    let (min_damage, max_damage) = castle_lanes::sim::damage_range(midpoint, variance);
    if min_damage == max_damage {
        min_damage.to_string()
    } else {
        format!("{min_damage}-{max_damage}")
    }
}

fn infer_attacker<'a>(
    snapshot: &'a MatchSnapshot,
    balance: &BalanceConfig,
    target_pos: Vec2,
    target_team: Team,
) -> Option<&'a Unit> {
    snapshot
        .units
        .iter()
        .filter(|unit| unit.owner != target_team)
        .map(|unit| {
            let unit_config = balance.unit(unit.kind);
            let pos = unit_world_pos(unit);
            let score = pos.distance(target_pos);
            (unit, score, unit_config.attack_range + 3.5)
        })
        .filter(|(_, score, range)| *score <= range * 9.0)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(unit, _, _)| unit)
}

fn building_hit_pos(building: &Building) -> Vec2 {
    cell_to_world(building.owner, building.lane, building.zone, building.cell)
        + Vec2::new(0.0, 14.0)
}

fn spawn_structure_impact(
    commands: &mut Commands,
    target: Vec2,
    damage: i32,
    source: Vec2,
    attack_type: AttackType,
) {
    spawn_combat_impact(commands, target, damage, source, attack_type, false);
    let debris_color = Color::srgb(0.74, 0.56, 0.34);
    for (idx, offset) in [
        Vec2::new(-13.0, -5.0),
        Vec2::new(15.0, -2.0),
        Vec2::new(-3.0, 10.0),
        Vec2::new(8.0, 6.0),
    ]
    .into_iter()
    .enumerate()
    {
        commands.spawn((
            Sprite::from_color(debris_color.with_alpha(0.82), Vec2::new(8.0, 3.0)),
            Transform::from_xyz(target.x + offset.x, target.y + offset.y, VFX_Z + 2.0)
                .with_rotation(Quat::from_rotation_z(idx as f32 * 0.78)),
            CombatVfx {
                lifetime: 0.42,
                max_lifetime: 0.42,
                velocity: Vec2::new(offset.x * 0.55, 22.0 + offset.y.abs()),
            },
        ));
    }
}

fn spawn_combat_impact(
    commands: &mut Commands,
    target: Vec2,
    damage: i32,
    source: Vec2,
    attack_type: AttackType,
    castle_hit: bool,
) {
    let color = damage_number_color(attack_type, castle_hit);
    let big_hit = damage >= if castle_hit { 12 } else { 8 };
    let text = if big_hit {
        format!("{damage}!")
    } else {
        damage.to_string()
    };
    spawn_damage_text(
        commands,
        &text,
        target + Vec2::new(0.0, 29.0),
        Color::srgb(0.05, 0.02, 0.01),
        if castle_hit { 32.0 } else { 26.0 },
        VFX_Z + 4.0,
    );
    spawn_damage_text(
        commands,
        &text,
        target + Vec2::new(2.0, 27.0),
        color,
        if castle_hit { 31.0 } else { 25.0 },
        VFX_Z + 5.0,
    );

    spawn_hit_burst(commands, target, attack_type, castle_hit);
    spawn_attack_streak(commands, source, target, attack_type);
}

fn spawn_damage_text(
    commands: &mut Commands,
    text: &str,
    pos: Vec2,
    color: Color,
    size: f32,
    z: f32,
) {
    commands.spawn((
        Text2d::new(text),
        TextFont::from_font_size(size),
        TextColor(color),
        TextLayout::new_with_justify(Justify::Center),
        Anchor::CENTER,
        Transform::from_xyz(pos.x, pos.y, z),
        CombatVfx {
            lifetime: 0.95,
            max_lifetime: 0.95,
            velocity: Vec2::new(16.0, 58.0),
        },
    ));
}

fn spawn_bounty_text(commands: &mut Commands, pos: Vec2, amount: i32) {
    let text = format!("+{amount}g");
    spawn_floating_text(
        commands,
        &text,
        pos + Vec2::new(0.0, 4.0),
        Color::srgb(0.08, 0.04, 0.00),
        30.0,
        VFX_Z + 8.0,
        Vec2::new(-7.0, 72.0),
        1.12,
    );
    spawn_floating_text(
        commands,
        &text,
        pos + Vec2::new(2.0, 6.0),
        Color::srgb(1.0, 0.86, 0.24),
        29.0,
        VFX_Z + 9.0,
        Vec2::new(-7.0, 72.0),
        1.12,
    );

    for offset in [
        Vec2::new(-16.0, -2.0),
        Vec2::new(19.0, 3.0),
        Vec2::new(4.0, 15.0),
    ] {
        commands.spawn((
            Sprite::from_color(Color::srgb(1.0, 0.73, 0.16), Vec2::splat(7.0)),
            Transform::from_xyz(pos.x + offset.x, pos.y + offset.y, VFX_Z + 7.0)
                .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_4)),
            CombatVfx {
                lifetime: 0.72,
                max_lifetime: 0.72,
                velocity: Vec2::new(offset.x * 0.42, 48.0 + offset.y.max(0.0)),
            },
        ));
    }
}

fn spawn_floating_text(
    commands: &mut Commands,
    text: &str,
    pos: Vec2,
    color: Color,
    size: f32,
    z: f32,
    velocity: Vec2,
    lifetime: f32,
) {
    commands.spawn((
        Text2d::new(text),
        TextFont::from_font_size(size),
        TextColor(color),
        TextLayout::new_with_justify(Justify::Center),
        Anchor::CENTER,
        Transform::from_xyz(pos.x, pos.y, z),
        CombatVfx {
            lifetime,
            max_lifetime: lifetime,
            velocity,
        },
    ));
}

fn spawn_hit_burst(
    commands: &mut Commands,
    target: Vec2,
    attack_type: AttackType,
    castle_hit: bool,
) {
    let color = attack_type_color(attack_type);
    let size = if castle_hit { 42.0 } else { 28.0 };
    commands.spawn((
        Sprite::from_color(color.with_alpha(0.82), Vec2::splat(size)),
        Transform::from_xyz(target.x, target.y + 4.0, VFX_Z)
            .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_4)),
        CombatVfx {
            lifetime: 0.32,
            max_lifetime: 0.32,
            velocity: Vec2::ZERO,
        },
    ));
    commands.spawn((
        Sprite::from_color(
            Color::srgb(1.0, 0.93, 0.63).with_alpha(0.76),
            Vec2::new(size, 5.0),
        ),
        Transform::from_xyz(target.x, target.y + 4.0, VFX_Z + 1.0),
        CombatVfx {
            lifetime: 0.24,
            max_lifetime: 0.24,
            velocity: Vec2::ZERO,
        },
    ));
}

fn spawn_attack_streak(
    commands: &mut Commands,
    source: Vec2,
    target: Vec2,
    attack_type: AttackType,
) {
    let delta = target - source;
    let length = delta.length().clamp(18.0, 120.0);
    if length <= 1.0 {
        return;
    }
    let angle = delta.y.atan2(delta.x);
    let center = source + delta * 0.55;
    commands.spawn((
        Sprite::from_color(
            attack_type_color(attack_type).with_alpha(0.72),
            Vec2::new(
                length,
                if attack_type == AttackType::Magic {
                    5.0
                } else {
                    3.0
                },
            ),
        ),
        Transform::from_xyz(center.x, center.y + 10.0, VFX_Z - 1.0)
            .with_rotation(Quat::from_rotation_z(angle)),
        CombatVfx {
            lifetime: 0.20,
            max_lifetime: 0.20,
            velocity: Vec2::ZERO,
        },
    ));
}

fn damage_number_color(attack_type: AttackType, castle_hit: bool) -> Color {
    if castle_hit {
        return Color::srgb(1.0, 0.42, 0.20);
    }
    match attack_type {
        AttackType::Normal => Color::srgb(1.0, 0.86, 0.24),
        AttackType::Pierce => Color::srgb(0.70, 0.94, 1.0),
        AttackType::Magic => Color::srgb(0.95, 0.64, 1.0),
        AttackType::Siege => Color::srgb(1.0, 0.56, 0.18),
        AttackType::Chaos => Color::srgb(1.0, 0.18, 0.14),
    }
}

fn building_icon_handle(assets: &BuildingIconAssets, kind: BuildingKind) -> Handle<Image> {
    match kind {
        BuildingKind::VanguardBarracks => assets.vanguard_barracks.clone(),
        BuildingKind::VanguardRangeTower => assets.vanguard_range_tower.clone(),
        BuildingKind::VanguardForge => assets.vanguard_forge.clone(),
        BuildingKind::VanguardPikeYard => assets.vanguard_pike_yard.clone(),
        BuildingKind::VanguardBulwarkHall => assets.vanguard_bulwark_hall.clone(),
        BuildingKind::VanguardChapel => assets.vanguard_chapel.clone(),
        BuildingKind::VanguardStables => assets.vanguard_stables.clone(),
        BuildingKind::VanguardSiegeWorkshop => assets.vanguard_siege_workshop.clone(),
        BuildingKind::GroveRootDen => assets.grove_root_den.clone(),
        BuildingKind::GroveThornSpire => assets.grove_thorn_spire.clone(),
        BuildingKind::GroveBloomWell => assets.grove_bloom_well.clone(),
        BuildingKind::GroveMossNursery => assets.grove_moss_nursery.clone(),
        BuildingKind::GroveBarkBastion => assets.grove_bark_bastion.clone(),
        BuildingKind::GroveMirePool => assets.grove_mire_pool.clone(),
        BuildingKind::GroveVineWarren => assets.grove_vine_warren.clone(),
        BuildingKind::GroveAncientSeed => assets.grove_ancient_seed.clone(),
        BuildingKind::EmberCinderPit => assets.ember_cinder_pit.clone(),
        BuildingKind::EmberFlameSpire => assets.ember_flame_spire.clone(),
        BuildingKind::EmberAshMine => assets.ember_ash_mine.clone(),
        BuildingKind::EmberSparkKennel => assets.ember_spark_kennel.clone(),
        BuildingKind::EmberObsidianGate => assets.ember_obsidian_gate.clone(),
        BuildingKind::EmberBlazeStable => assets.ember_blaze_stable.clone(),
        BuildingKind::EmberSmokeAltar => assets.ember_smoke_altar.clone(),
        BuildingKind::EmberInfernoEngine => assets.ember_inferno_engine.clone(),
    }
}

fn castle_icon_handle(assets: &BuildingIconAssets, race: RaceKind) -> Handle<Image> {
    match race {
        RaceKind::Vanguard => assets.vanguard_barracks.clone(),
        RaceKind::Grove => assets.grove_root_den.clone(),
        RaceKind::Ember => assets.ember_cinder_pit.clone(),
    }
}

fn unit_sprite_handle(assets: &UnitSpriteAssets, kind: UnitKind) -> Handle<Image> {
    match kind {
        UnitKind::VanguardGuard => assets.vanguard_guard.clone(),
        UnitKind::VanguardArcher => assets.vanguard_archer.clone(),
        UnitKind::VanguardPikeman => assets.vanguard_pikeman.clone(),
        UnitKind::VanguardShieldbearer => assets.vanguard_shieldbearer.clone(),
        UnitKind::VanguardBattleCleric => assets.vanguard_battle_cleric.clone(),
        UnitKind::VanguardLancer => assets.vanguard_lancer.clone(),
        UnitKind::VanguardBallista => assets.vanguard_ballista.clone(),
        UnitKind::GroveBruiser => assets.grove_bruiser.clone(),
        UnitKind::GroveNeedler => assets.grove_needler.clone(),
        UnitKind::GroveSproutling => assets.grove_sproutling.clone(),
        UnitKind::GroveBarkguard => assets.grove_barkguard.clone(),
        UnitKind::GroveMireShaman => assets.grove_mire_shaman.clone(),
        UnitKind::GroveVineStalker => assets.grove_vine_stalker.clone(),
        UnitKind::GroveTreantColossus => assets.grove_treant_colossus.clone(),
        UnitKind::EmberRunner => assets.ember_runner.clone(),
        UnitKind::EmberCaster => assets.ember_caster.clone(),
        UnitKind::EmberSparkImp => assets.ember_spark_imp.clone(),
        UnitKind::EmberObsidianGuard => assets.ember_obsidian_guard.clone(),
        UnitKind::EmberFireLancer => assets.ember_fire_lancer.clone(),
        UnitKind::EmberSmokeWitch => assets.ember_smoke_witch.clone(),
        UnitKind::EmberCinderEngine => assets.ember_cinder_engine.clone(),
    }
}

fn unit_sprite_size(kind: UnitKind) -> Vec2 {
    match kind {
        UnitKind::GroveBruiser => Vec2::splat(72.0),
        UnitKind::VanguardGuard => Vec2::splat(66.0),
        UnitKind::VanguardArcher => Vec2::splat(62.0),
        UnitKind::VanguardPikeman => Vec2::splat(64.0),
        UnitKind::VanguardShieldbearer => Vec2::splat(70.0),
        UnitKind::VanguardBattleCleric => Vec2::splat(63.0),
        UnitKind::VanguardLancer => Vec2::splat(70.0),
        UnitKind::VanguardBallista => Vec2::splat(74.0),
        UnitKind::GroveNeedler => Vec2::splat(64.0),
        UnitKind::GroveSproutling => Vec2::splat(52.0),
        UnitKind::GroveBarkguard => Vec2::splat(74.0),
        UnitKind::GroveMireShaman => Vec2::splat(64.0),
        UnitKind::GroveVineStalker => Vec2::splat(64.0),
        UnitKind::GroveTreantColossus => Vec2::splat(82.0),
        UnitKind::EmberRunner => Vec2::splat(61.0),
        UnitKind::EmberCaster => Vec2::splat(64.0),
        UnitKind::EmberSparkImp => Vec2::splat(54.0),
        UnitKind::EmberObsidianGuard => Vec2::splat(70.0),
        UnitKind::EmberFireLancer => Vec2::splat(67.0),
        UnitKind::EmberSmokeWitch => Vec2::splat(64.0),
        UnitKind::EmberCinderEngine => Vec2::splat(76.0),
    }
}

fn unit_world_pos(unit: &Unit) -> Vec2 {
    sim_pos_to_world(unit.pos)
}

fn castle_world_pos(team: Team) -> Vec2 {
    Vec2::new(lane_to_world(team.castle_pos()), LANE_Y + 8.0)
}

fn sim_pos_to_world(pos: castle_lanes::sim::WorldPos) -> Vec2 {
    Vec2::new(lane_to_world(pos.x), pos.y * SIM_Y_TO_WORLD)
}

fn is_unit_visible(snapshot: &MatchSnapshot, viewer_team: Option<Team>, unit: &Unit) -> bool {
    if Some(unit.owner) == viewer_team || viewer_team.is_none() {
        return true;
    }
    is_world_revealed(snapshot, viewer_team, unit_world_pos(unit))
}

fn is_building_visible(
    snapshot: &MatchSnapshot,
    viewer_team: Option<Team>,
    building: &Building,
) -> bool {
    if Some(building.owner) == viewer_team || viewer_team.is_none() {
        return true;
    }
    is_world_revealed(
        snapshot,
        viewer_team,
        cell_to_world(building.owner, building.lane, building.zone, building.cell),
    )
}

fn is_castle_visible(snapshot: &MatchSnapshot, viewer_team: Option<Team>, team: Team) -> bool {
    if Some(team) == viewer_team || viewer_team.is_none() {
        return true;
    }
    is_world_revealed(snapshot, viewer_team, castle_world_pos(team))
}

fn is_world_revealed(snapshot: &MatchSnapshot, viewer_team: Option<Team>, pos: Vec2) -> bool {
    let Some(team) = viewer_team else {
        return true;
    };
    world_reveal_strength(snapshot, team, pos) > 0.08
}

fn world_reveal_strength(snapshot: &MatchSnapshot, team: Team, pos: Vec2) -> f32 {
    let castle_strength =
        reveal_strength(pos.distance(castle_world_pos(team)), REVEAL_CASTLE_RADIUS);
    let building_strength = snapshot
        .buildings
        .iter()
        .filter(|building| building.owner == team)
        .map(|building| {
            reveal_strength(
                pos.distance(cell_to_world(
                    building.owner,
                    building.lane,
                    building.zone,
                    building.cell,
                )),
                REVEAL_BUILDING_RADIUS,
            )
        })
        .fold(0.0, f32::max);
    let unit_strength = snapshot
        .units
        .iter()
        .filter(|unit| unit.owner == team)
        .map(|unit| reveal_strength(pos.distance(unit_world_pos(unit)), REVEAL_UNIT_RADIUS))
        .fold(0.0, f32::max);
    castle_strength.max(building_strength).max(unit_strength)
}

fn reveal_strength(distance: f32, radius: f32) -> f32 {
    if distance >= radius {
        0.0
    } else if distance <= radius - FOG_SOFT_EDGE {
        1.0
    } else {
        ((radius - distance) / FOG_SOFT_EDGE).clamp(0.0, 1.0)
    }
}

fn is_enemy_building_scouted(
    snapshot: &MatchSnapshot,
    fog: &FogMemory,
    viewer_team: Option<Team>,
    building: &Building,
) -> bool {
    let Some(team) = viewer_team else {
        return false;
    };
    if building.owner == team {
        return false;
    }
    let pos = cell_to_world(building.owner, building.lane, building.zone, building.cell);
    !is_world_revealed(snapshot, viewer_team, pos) && fog_is_explored(fog, viewer_team, pos)
}

fn fog_is_explored(fog: &FogMemory, viewer_team: Option<Team>, pos: Vec2) -> bool {
    let Some(team) = viewer_team else {
        return true;
    };
    if fog.team != Some(team) {
        return false;
    }
    let Some((col, row)) = fog_cell(pos) else {
        return false;
    };
    fog.explored[fog_index(col, row)]
}

fn fog_index(col: usize, row: usize) -> usize {
    row * FOG_COLUMNS + col
}

fn fog_cell(pos: Vec2) -> Option<(usize, usize)> {
    let x = ((pos.x + MAP_W * 0.5) / MAP_W * FOG_COLUMNS as f32).floor() as isize;
    let y = ((pos.y + MAP_H * 0.5) / MAP_H * FOG_ROWS as f32).floor() as isize;
    if x < 0 || y < 0 || x >= FOG_COLUMNS as isize || y >= FOG_ROWS as isize {
        return None;
    }
    Some((x as usize, y as usize))
}

fn fog_cell_center(col: usize, row: usize) -> Vec2 {
    let tile_w = MAP_W / FOG_COLUMNS as f32;
    let tile_h = MAP_H / FOG_ROWS as f32;
    Vec2::new(
        -MAP_W * 0.5 + tile_w * (col as f32 + 0.5),
        -MAP_H * 0.5 + tile_h * (row as f32 + 0.5),
    )
}

fn spawn_fog_of_war(
    commands: &mut Commands,
    snapshot: &MatchSnapshot,
    fog: &FogMemory,
    viewer_team: Option<Team>,
) {
    if viewer_team.is_none() {
        return;
    }
    let Some(team) = viewer_team else {
        return;
    };
    let tile_w = MAP_W / FOG_COLUMNS as f32;
    let tile_h = MAP_H / FOG_ROWS as f32;
    for col in 0..FOG_COLUMNS {
        for row in 0..FOG_ROWS {
            let pos = fog_cell_center(col, row);
            let reveal = world_reveal_strength(snapshot, team, pos);
            let explored = fog.explored[fog_index(col, row)];
            let alpha = if reveal > 0.0 {
                0.20 * (1.0 - reveal)
            } else if explored {
                0.53
            } else {
                0.88
            };
            if alpha <= 0.015 {
                continue;
            }
            let tint = if explored {
                Color::srgba(0.006, 0.009, 0.012, alpha)
            } else {
                Color::srgba(0.002, 0.002, 0.003, alpha)
            };
            spawn_rect(
                commands,
                pos,
                Vec2::new(tile_w + 1.0, tile_h + 1.0),
                tint,
                18.0,
            );
        }
    }
}

fn lane_world_y(lane: Lane) -> f32 {
    match lane {
        Lane::Top => 128.0,
        Lane::Bottom => -128.0,
    }
}

fn ui_summary(state: &SnapshotState, net: &ClientNet) -> (String, String, String) {
    let Some(snapshot) = &state.snapshot else {
        return (
            "No match snapshot".to_string(),
            "Press Enter to join".to_string(),
            format!("{}\nStart the dedicated server if needed.", net.status),
        );
    };

    let phase_line = match snapshot.phase {
        MatchPhase::Lobby => "Lobby: choose race, then ready".to_string(),
        MatchPhase::Playing => {
            let pressure = if snapshot.sudden_death {
                "Sudden death active".to_string()
            } else {
                format!(
                    "Sudden death in {}",
                    format_match_time(
                        (state.balance.sudden_death_start - snapshot.elapsed_secs).max(0.0)
                    )
                )
            };
            format!(
                "Battle {}  |  {}",
                format_match_time(snapshot.elapsed_secs),
                pressure
            )
        }
        MatchPhase::GameOver => "Game over".to_string(),
    };

    let economy_line = if let Some(player_id) = net.player_id {
        if let Some(player) = current_player(snapshot, player_id) {
            let econ = &snapshot.economies[player.team.slot()];
            let castle = &snapshot.castles[player.team.slot()];
            format!(
                "Gold {}   Income {}   Castle {}/{}",
                econ.gold, econ.income, castle.health, castle.max_health
            )
        } else {
            "Waiting for roster".to_string()
        }
    } else {
        "Not joined".to_string()
    };

    let message = if snapshot.message.is_empty() {
        net.status.clone()
    } else {
        format!("{}\n{}", net.status, snapshot.message)
    };
    (phase_line, economy_line, truncate_text(&message, 94))
}

fn local_gold(state: &SnapshotState, net: &ClientNet) -> Option<i32> {
    let snapshot = state.snapshot.as_ref()?;
    let player_id = net.player_id?;
    let player = current_player(snapshot, player_id)?;
    Some(snapshot.economies[player.team.slot()].gold)
}

fn spawn_value_bar(commands: &mut Commands, pos: Vec2, size: Vec2, pct: f32, color: Color, z: f32) {
    let pct = pct.clamp(0.0, 1.0);
    spawn_ui_rect(
        commands,
        pos,
        size,
        Color::srgba(0.035, 0.028, 0.020, 0.92),
        z,
    );
    spawn_ui_rect(
        commands,
        Vec2::new(pos.x - size.x * (1.0 - pct) * 0.5, pos.y),
        Vec2::new(size.x * pct, size.y),
        color,
        z + 1.0,
    );
}

fn truncate_text(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let mut out: String = value.chars().take(max_chars.saturating_sub(3)).collect();
    out.push_str("...");
    out
}

fn active_balance(state: &SnapshotState) -> BalanceConfig {
    state.balance.clone()
}

fn current_player(
    snapshot: &MatchSnapshot,
    player_id: PlayerId,
) -> Option<&castle_lanes::sim::PlayerInfo> {
    snapshot
        .players
        .iter()
        .find(|player| player.id == player_id)
}

fn selected_race(state: &SnapshotState, net: &ClientNet) -> Option<RaceKind> {
    let snapshot = state.snapshot.as_ref()?;
    let player_id = net.player_id?;
    current_player(snapshot, player_id).and_then(|player| player.race)
}

fn race_selection_popup_open(state: &SnapshotState, net: &ClientNet) -> bool {
    let Some(snapshot) = &state.snapshot else {
        return false;
    };
    if snapshot.phase != MatchPhase::Lobby {
        return false;
    }
    let Some(player_id) = net.player_id else {
        return false;
    };
    current_player(snapshot, player_id).is_some_and(|player| !player.ready)
}

fn team_race(snapshot: &MatchSnapshot, team: Team) -> Option<RaceKind> {
    snapshot
        .players
        .iter()
        .find(|player| player.team == team)
        .and_then(|player| player.race)
}

fn send_join(net: &mut ClientNet) {
    send_client(
        net,
        &ClientPacket::Join {
            version: PROTOCOL_VERSION,
            name: net.player_name.clone(),
        },
    );
    net.last_join = Instant::now();
}

fn send_client(net: &ClientNet, packet: &ClientPacket) {
    if let Ok(bytes) = encode(packet) {
        let _ = net.socket.send_to(&bytes, net.server_addr);
    }
}

fn visible_sides(team: Option<Team>) -> Vec<Team> {
    match team {
        Some(team) => vec![team],
        None => vec![Team::Left, Team::Right],
    }
}

fn spawn_grass_background(commands: &mut Commands) {
    commands.spawn((
        Sprite::from_color(
            Color::srgb(0.22, 0.38, 0.18),
            Vec2::new(MAP_W + 260.0, MAP_H + 180.0),
        ),
        Transform::from_xyz(0.0, 0.0, -40.0),
    ));

    for idx in 0..140 {
        let x = hash_range(idx, 1, -MAP_W * 0.5, MAP_W * 0.5);
        let y = hash_range(idx, 2, -MAP_H * 0.5, MAP_H * 0.5);
        let w = hash_range(idx, 3, 34.0, 120.0);
        let h = hash_range(idx, 4, 16.0, 46.0);
        let tint = hash_range(idx, 5, -0.035, 0.045);
        let color = Color::srgba(
            (0.20 + tint).clamp(0.0, 1.0),
            (0.36 + tint).clamp(0.0, 1.0),
            (0.16 + tint * 0.5).clamp(0.0, 1.0),
            0.28,
        );
        commands.spawn((
            Sprite::from_color(color, Vec2::new(w, h)),
            Transform::from_xyz(x, y, -38.0)
                .with_rotation(Quat::from_rotation_z(hash_range(idx, 6, -0.25, 0.25))),
        ));
    }

    for idx in 0..520 {
        let x = hash_range(idx, 11, -MAP_W * 0.5, MAP_W * 0.5);
        let y = hash_range(idx, 12, -MAP_H * 0.5, MAP_H * 0.5);
        let height = hash_range(idx, 13, 7.0, 15.0);
        let width = hash_range(idx, 14, 1.2, 2.6);
        let phase = hash_range(idx, 15, 0.0, std::f32::consts::TAU);
        let rotation = hash_range(idx, 16, -0.15, 0.15);
        let color = Color::srgba(
            hash_range(idx, 17, 0.24, 0.34),
            hash_range(idx, 18, 0.43, 0.58),
            hash_range(idx, 19, 0.18, 0.26),
            0.46,
        );
        commands.spawn((
            Sprite::from_color(color, Vec2::new(width, height)),
            Transform::from_xyz(x, y, hash_range(idx, 20, -33.0, -28.0))
                .with_rotation(Quat::from_rotation_z(rotation)),
            GrassBlade {
                base_pos: Vec2::new(x, y),
                base_rotation: rotation,
                phase,
                sway: hash_range(idx, 21, 0.7, 2.1),
            },
        ));
    }
}

fn hash_range(index: u32, salt: u32, min: f32, max: f32) -> f32 {
    min + (max - min) * hash_unit(index, salt)
}

fn hash_unit(index: u32, salt: u32) -> f32 {
    let mut value = index
        .wrapping_mul(747_796_405)
        .wrapping_add(salt.wrapping_mul(2_891_336_453));
    value ^= value >> 16;
    value = value.wrapping_mul(2_246_822_519);
    value ^= value >> 13;
    (value as f32) / (u32::MAX as f32)
}

fn spawn_static_board(commands: &mut Commands, team: Option<Team>) {
    for lane in Lane::ALL {
        let y = lane_world_y(lane);
        spawn_rect(
            commands,
            Vec2::new(0.0, y),
            Vec2::new(WORLD_W - 120.0, 30.0),
            Color::srgba(0.12, 0.10, 0.07, 0.34),
            -1.0,
        );
        spawn_rect(
            commands,
            Vec2::new(0.0, y),
            Vec2::new(WORLD_W - 160.0, 3.0),
            Color::srgba(0.80, 0.64, 0.34, 0.42),
            0.0,
        );
    }

    for side in visible_sides(team) {
        for lane in Lane::ALL {
            for zone in BuildZone::ALL {
                for x in 0..GRID_W {
                    for y in 0..GRID_H {
                        let pos = cell_to_world(side, lane, zone, GridCell { x, y });
                        let color = if Some(side) == team {
                            match zone {
                                BuildZone::Front => Color::srgba(0.26, 0.56, 0.72, 0.32),
                                BuildZone::Back => Color::srgba(0.30, 0.42, 0.68, 0.22),
                            }
                        } else {
                            Color::srgba(0.07, 0.06, 0.05, 0.20)
                        };
                        spawn_rect(commands, pos, Vec2::splat(CELL - 3.0), color, 0.5);
                        spawn_rect(
                            commands,
                            pos,
                            Vec2::new(CELL - 3.0, 2.0),
                            Color::srgba(0.67, 0.54, 0.31, 0.18),
                            0.7,
                        );
                    }
                }
            }
        }
    }
}

fn spawn_building(
    commands: &mut Commands,
    building: &Building,
    balance: &BalanceConfig,
    building_icons: &BuildingIconAssets,
    selected: bool,
) {
    let pos = cell_to_world(building.owner, building.lane, building.zone, building.cell);
    let config = balance.building(building.kind);
    let color = Color::srgb(config.color[0], config.color[1], config.color[2]);
    if selected {
        spawn_rect(
            commands,
            Vec2::new(pos.x, pos.y - 2.0),
            Vec2::new(CELL + 4.0, CELL + 4.0),
            Color::srgba(0.95, 0.76, 0.24, 0.42),
            2.5,
        );
    }
    spawn_rect(
        commands,
        Vec2::new(pos.x, pos.y - 8.0),
        Vec2::new(CELL - 8.0, CELL - 8.0),
        Color::srgba(0.04, 0.035, 0.025, 0.58),
        2.7,
    );
    spawn_rect(
        commands,
        Vec2::new(pos.x, pos.y + 3.0),
        Vec2::new(CELL - 12.0, CELL - 12.0),
        color.with_alpha(0.50),
        3.0,
    );
    let mut sprite = Sprite::from_image(building_icon_handle(building_icons, building.kind));
    sprite.custom_size = Some(Vec2::splat(if selected { 52.0 } else { 48.0 }));
    commands.spawn((
        sprite,
        Transform::from_xyz(pos.x, pos.y + 4.0, 4.2),
        SceneEntity,
    ));
}

fn spawn_building_silhouette(commands: &mut Commands, building: &Building) {
    let pos = cell_to_world(building.owner, building.lane, building.zone, building.cell);
    spawn_rect(
        commands,
        Vec2::new(pos.x, pos.y - 7.0),
        Vec2::new(CELL - 7.0, CELL - 7.0),
        Color::srgba(0.015, 0.014, 0.013, 0.50),
        2.7,
    );
    spawn_rect(
        commands,
        Vec2::new(pos.x, pos.y + 2.0),
        Vec2::new(CELL - 13.0, CELL - 13.0),
        Color::srgba(0.42, 0.38, 0.30, 0.24),
        3.0,
    );
    spawn_rect(
        commands,
        Vec2::new(pos.x, pos.y + 14.0),
        Vec2::new(CELL - 18.0, 3.0),
        Color::srgba(0.72, 0.66, 0.48, 0.18),
        3.2,
    );
}

fn spawn_castle(
    commands: &mut Commands,
    castle: &SimCastle,
    race: RaceKind,
    building_icons: &BuildingIconAssets,
    selected: bool,
) {
    let pos = castle_world_pos(castle.team);
    let sprite_size = if selected { 118.0 } else { 108.0 };
    if selected {
        spawn_rect(
            commands,
            Vec2::new(pos.x, pos.y - 22.0),
            Vec2::new(126.0, 28.0),
            Color::srgba(0.95, 0.75, 0.26, 0.34),
            5.4,
        );
    }
    spawn_rect(
        commands,
        Vec2::new(pos.x, pos.y - 24.0),
        Vec2::new(104.0, 20.0),
        Color::srgba(0.02, 0.018, 0.012, 0.48),
        5.2,
    );
    let mut sprite = Sprite::from_image(castle_icon_handle(building_icons, race));
    sprite.custom_size = Some(Vec2::splat(sprite_size));
    sprite.flip_x = castle.team == Team::Right;
    commands.spawn((
        sprite,
        Transform::from_xyz(pos.x, pos.y + 26.0, 7.2),
        SceneEntity,
    ));

    let health_pct = castle.health.max(0) as f32 / castle.max_health.max(1) as f32;
    spawn_rect(
        commands,
        Vec2::new(pos.x, pos.y + 94.0),
        Vec2::new(104.0, 11.0),
        Color::srgba(0.05, 0.04, 0.03, 0.88),
        9.0,
    );
    spawn_rect(
        commands,
        Vec2::new(pos.x - 104.0 * (1.0 - health_pct) * 0.5, pos.y + 94.0),
        Vec2::new(104.0 * health_pct, 11.0),
        Color::srgb(0.15, 0.78, 0.34),
        10.0,
    );
}

fn spawn_unit(
    commands: &mut Commands,
    unit: &Unit,
    balance: &BalanceConfig,
    unit_assets: &UnitSpriteAssets,
    selected: bool,
    tick: u64,
) {
    let config = balance.unit(unit.kind);
    let pos = unit_world_pos(unit);
    let phase = tick as f32 * 0.18 + unit.id as f32 * 1.37;
    let bob = phase.sin() * 2.0;
    let squash = 1.0 + phase.cos() * 0.018;
    let sprite_size = unit_sprite_size(unit.kind);
    if selected {
        spawn_diamond(
            commands,
            Vec2::new(pos.x, pos.y - 5.0),
            Vec2::new(sprite_size.x * 0.80, sprite_size.x * 0.80),
            Color::srgba(0.95, 0.76, 0.24, 0.48),
            7.6,
        );
    }
    spawn_rect(
        commands,
        Vec2::new(pos.x, pos.y - 15.0),
        Vec2::new(sprite_size.x * 0.50, 8.0),
        Color::srgba(0.02, 0.018, 0.014, 0.42),
        7.8,
    );
    let mut sprite = Sprite::from_image(unit_sprite_handle(unit_assets, unit.kind));
    sprite.custom_size = Some(sprite_size);
    sprite.flip_x = unit.owner == Team::Right;
    commands.spawn((
        sprite,
        Transform::from_xyz(pos.x, pos.y + bob + 12.0, 8.4).with_scale(Vec3::new(1.0, squash, 1.0)),
        SceneEntity,
    ));
    spawn_rect(
        commands,
        Vec2::new(pos.x - 9.0, pos.y + 27.0 + bob),
        Vec2::new(15.0, 4.0),
        attack_type_color(config.attack_type),
        11.0,
    );
    spawn_rect(
        commands,
        Vec2::new(pos.x + 9.0, pos.y + 27.0 + bob),
        Vec2::new(15.0, 4.0),
        armor_type_color(config.armor_type),
        11.0,
    );

    let health_pct = (unit.health.max(0) as f32 / config.max_health as f32).clamp(0.0, 1.0);
    spawn_rect(
        commands,
        Vec2::new(pos.x, pos.y - 24.0),
        Vec2::new(34.0, 4.0),
        Color::srgb(0.10, 0.11, 0.11),
        9.0,
    );
    spawn_rect(
        commands,
        Vec2::new(pos.x - (34.0 * (1.0 - health_pct) / 2.0), pos.y - 24.0),
        Vec2::new(34.0 * health_pct, 4.0),
        Color::srgb(0.18, 0.88, 0.40),
        10.0,
    );
}

fn spawn_rect(commands: &mut Commands, pos: Vec2, size: Vec2, color: Color, z: f32) {
    commands.spawn((
        Sprite::from_color(color, size),
        Transform::from_xyz(pos.x, pos.y, z),
        SceneEntity,
    ));
}

fn spawn_diamond(commands: &mut Commands, pos: Vec2, size: Vec2, color: Color, z: f32) {
    commands.spawn((
        Sprite::from_color(color, size),
        Transform::from_xyz(pos.x, pos.y, z)
            .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_4)),
        SceneEntity,
    ));
}

fn spawn_ui_panel(commands: &mut Commands, pos: Vec2, size: Vec2, z: f32) {
    spawn_ui_rect(
        commands,
        pos,
        size,
        Color::srgba(0.025, 0.020, 0.016, 0.96),
        z,
    );
    spawn_ui_rect(commands, pos, size - Vec2::splat(8.0), PANEL_BG, z + 0.2);
    spawn_ui_rect(
        commands,
        Vec2::new(pos.x, pos.y + size.y * 0.5 - 2.0),
        Vec2::new(size.x, 4.0),
        PANEL_EDGE,
        z + 1.0,
    );
    spawn_ui_rect(
        commands,
        Vec2::new(pos.x, pos.y - size.y * 0.5 + 2.0),
        Vec2::new(size.x, 4.0),
        Color::srgba(0.18, 0.12, 0.07, 0.96),
        z + 1.0,
    );
    spawn_ui_rect(
        commands,
        Vec2::new(pos.x - size.x * 0.5 + 2.0, pos.y),
        Vec2::new(4.0, size.y),
        Color::srgba(0.18, 0.13, 0.08, 0.92),
        z + 1.0,
    );
    spawn_ui_rect(
        commands,
        Vec2::new(pos.x + size.x * 0.5 - 2.0, pos.y),
        Vec2::new(4.0, size.y),
        Color::srgba(0.36, 0.25, 0.12, 0.92),
        z + 1.0,
    );
    for dx in [-1.0, 1.0] {
        for dy in [-1.0, 1.0] {
            spawn_ui_rect(
                commands,
                Vec2::new(
                    pos.x + dx * (size.x * 0.5 - 8.0),
                    pos.y + dy * (size.y * 0.5 - 8.0),
                ),
                Vec2::splat(10.0),
                Color::srgba(0.72, 0.52, 0.24, 0.86),
                z + 2.0,
            );
        }
    }
}

fn spawn_ui_button(commands: &mut Commands, pos: Vec2, size: Vec2, color: Color, selected: bool) {
    spawn_ui_button_layer(commands, pos, size, color, selected, 44.0);
}

fn spawn_ui_button_layer(
    commands: &mut Commands,
    pos: Vec2,
    size: Vec2,
    color: Color,
    selected: bool,
    z: f32,
) {
    spawn_ui_rect(
        commands,
        pos,
        size,
        Color::srgba(0.02, 0.018, 0.015, 0.96),
        z,
    );
    spawn_ui_rect(commands, pos, size - Vec2::splat(5.0), color, z + 1.0);
    spawn_ui_rect(
        commands,
        Vec2::new(pos.x, pos.y + size.y * 0.5 - 8.0),
        Vec2::new(size.x - 10.0, 2.0),
        Color::srgba(0.95, 0.72, 0.32, 0.16),
        z + 2.0,
    );
    let edge = if selected {
        TEXT_GOLD
    } else {
        Color::srgba(0.45, 0.34, 0.18, 0.94)
    };
    spawn_ui_rect(
        commands,
        Vec2::new(pos.x, pos.y + size.y * 0.5 - 3.0),
        Vec2::new(size.x, 4.0),
        edge,
        z + 2.0,
    );
    spawn_ui_rect(
        commands,
        Vec2::new(pos.x, pos.y - size.y * 0.5 + 3.0),
        Vec2::new(size.x, 4.0),
        edge,
        z + 2.0,
    );
}

fn spawn_ui_rect(commands: &mut Commands, pos: Vec2, size: Vec2, color: Color, z: f32) {
    commands.spawn((
        Sprite::from_color(color, size),
        Transform::from_xyz(pos.x, pos.y, z),
        UiPinned {
            pos,
            size: Some(size),
            font_size: None,
            z,
        },
        UiEntity,
    ));
}

fn spawn_ui_image(commands: &mut Commands, image: Handle<Image>, pos: Vec2, size: Vec2, z: f32) {
    commands.spawn((
        Sprite {
            image,
            custom_size: Some(size),
            ..default()
        },
        Transform::from_xyz(pos.x, pos.y, z),
        UiPinned {
            pos,
            size: Some(size),
            font_size: None,
            z,
        },
        UiEntity,
    ));
}

fn spawn_ui_label(
    commands: &mut Commands,
    text: &str,
    pos: Vec2,
    size: f32,
    color: Color,
    z: f32,
    anchor: Anchor,
    justify: Justify,
) {
    commands.spawn((
        Text2d::new(text),
        TextFont::from_font_size(size),
        TextColor(color),
        TextLayout::new_with_justify(justify),
        anchor,
        Transform::from_xyz(pos.x, pos.y, z),
        UiPinned {
            pos,
            size: None,
            font_size: Some(size),
            z,
        },
        UiEntity,
    ));
}

fn pin_ui_to_camera(
    camera_query: Query<(&Transform, &Projection), (With<Camera2d>, Without<UiPinned>)>,
    mut ui_query: Query<(
        &UiPinned,
        &mut Transform,
        Option<&mut Sprite>,
        Option<&mut TextFont>,
    )>,
) {
    let Ok((camera_transform, projection)) = camera_query.single() else {
        return;
    };
    let scale = if let Projection::Orthographic(orthographic) = projection {
        orthographic.scale
    } else {
        1.0
    };
    let camera_pos = camera_transform.translation.truncate();
    for (pin, mut transform, sprite, font) in &mut ui_query {
        let pos = camera_pos + pin.pos * scale;
        transform.translation = Vec3::new(pos.x, pos.y, pin.z);
        if let (Some(mut sprite), Some(size)) = (sprite, pin.size) {
            sprite.custom_size = Some(size * scale);
        }
        if let (Some(mut font), Some(size)) = (font, pin.font_size) {
            font.font_size = size * scale;
        }
    }
}

fn attack_type_color(kind: AttackType) -> Color {
    match kind {
        AttackType::Normal => Color::srgb(0.86, 0.82, 0.68),
        AttackType::Pierce => Color::srgb(0.45, 0.78, 0.95),
        AttackType::Magic => Color::srgb(0.70, 0.48, 0.95),
        AttackType::Siege => Color::srgb(0.91, 0.62, 0.28),
        AttackType::Chaos => Color::srgb(0.94, 0.24, 0.20),
    }
}

fn armor_type_color(kind: ArmorType) -> Color {
    match kind {
        ArmorType::Normal => Color::srgb(0.72, 0.70, 0.62),
        ArmorType::Light => Color::srgb(0.58, 0.88, 0.52),
        ArmorType::Heavy => Color::srgb(0.49, 0.60, 0.72),
        ArmorType::Fortified => Color::srgb(0.72, 0.54, 0.34),
        ArmorType::Unarmored => Color::srgb(0.86, 0.75, 0.52),
    }
}

fn lane_to_world(pos: f32) -> f32 {
    (pos / LANE_LENGTH - 0.5) * (WORLD_W - 120.0)
}

fn cell_to_world(team: Team, lane: Lane, zone: BuildZone, cell: GridCell) -> Vec2 {
    let x = lane_to_world(building_lane_pos(team, zone, cell));
    let y = lane_world_y(lane) + (cell.y as f32 - (GRID_H - 1) as f32 * 0.5) * CELL;
    Vec2::new(x, y)
}

fn world_to_build_slot(team: Team, world: Vec2) -> Option<(Lane, BuildZone, GridCell)> {
    for lane in Lane::ALL {
        for zone in BuildZone::ALL {
            for x in 0..GRID_W {
                for y in 0..GRID_H {
                    let cell = GridCell { x, y };
                    let center = cell_to_world(team, lane, zone, cell);
                    let half = CELL * 0.44;
                    if world.x >= center.x - half
                        && world.x <= center.x + half
                        && world.y >= center.y - half
                        && world.y <= center.y + half
                    {
                        return Some((lane, zone, cell));
                    }
                }
            }
        }
    }
    None
}

fn default_lane_for_team(team: Team) -> Lane {
    match team {
        Team::Left => Lane::Top,
        Team::Right => Lane::Bottom,
    }
}
