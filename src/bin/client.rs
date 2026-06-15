use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::window::{PrimaryWindow, WindowResolution};
use castle_lanes::net::{
    ClientPacket, DEFAULT_SERVER_ADDR, PROTOCOL_VERSION, ServerPacket, decode_server, encode,
};
use castle_lanes::sim::{
    ArmorType, AttackType, BalanceConfig, Building, BuildingKind, GRID_H, GRID_W, GridCell,
    LANE_LENGTH, MatchPhase, MatchSnapshot, PlayerId, RaceKind, Team, Unit, UnitKind,
};
use std::collections::HashMap;
use std::env;
use std::io::ErrorKind;
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

const WORLD_W: f32 = 1000.0;
const LANE_Y: f32 = 0.0;
const CELL: f32 = 52.0;
const UI_PANEL_TOP_Y: f32 = -224.0;
const UI_PANEL_Y: f32 = -292.0;
const TOP_BAR_Y: f32 = 330.0;
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
}

#[derive(Resource)]
struct BuildSelection {
    kind: BuildingKind,
}

impl Default for BuildSelection {
    fn default() -> Self {
        Self {
            kind: BuildingKind::VanguardBarracks,
        }
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum SelectedObject {
    Unit(u64),
    Building(u64),
    Castle(Team),
    Cell(Team, GridCell),
}

#[derive(Resource, Default)]
struct CombatTracker {
    initialized: bool,
    units: HashMap<u64, TrackedUnit>,
    castle_health: [i32; 2],
}

#[derive(Clone, Copy)]
struct TrackedUnit {
    health: i32,
}

#[derive(Resource)]
struct UnitSpriteAssets {
    vanguard_guard: Handle<Image>,
    vanguard_archer: Handle<Image>,
    grove_bruiser: Handle<Image>,
    grove_needler: Handle<Image>,
    ember_runner: Handle<Image>,
    ember_caster: Handle<Image>,
}

#[derive(Resource)]
struct BuildingIconAssets {
    vanguard_barracks: Handle<Image>,
    vanguard_range_tower: Handle<Image>,
    vanguard_forge: Handle<Image>,
    grove_root_den: Handle<Image>,
    grove_thorn_spire: Handle<Image>,
    grove_bloom_well: Handle<Image>,
    ember_cinder_pit: Handle<Image>,
    ember_flame_spire: Handle<Image>,
    ember_ash_mine: Handle<Image>,
}

#[derive(Component)]
struct SceneEntity;

#[derive(Component)]
struct UiEntity;

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
        .init_resource::<CombatTracker>()
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
                redraw_scene,
                update_placement_preview,
                redraw_game_ui,
                update_combat_vfx,
            ),
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
        grove_bruiser: asset_server.load("art/units/grove_bruiser.png"),
        grove_needler: asset_server.load("art/units/grove_needler.png"),
        ember_runner: asset_server.load("art/units/ember_runner.png"),
        ember_caster: asset_server.load("art/units/ember_caster.png"),
    });
    commands.insert_resource(BuildingIconAssets {
        vanguard_barracks: asset_server.load("art/buildings/vanguard_barracks.png"),
        vanguard_range_tower: asset_server.load("art/buildings/vanguard_range_tower.png"),
        vanguard_forge: asset_server.load("art/buildings/vanguard_forge.png"),
        grove_root_den: asset_server.load("art/buildings/grove_root_den.png"),
        grove_thorn_spire: asset_server.load("art/buildings/grove_thorn_spire.png"),
        grove_bloom_well: asset_server.load("art/buildings/grove_bloom_well.png"),
        ember_cinder_pit: asset_server.load("art/buildings/ember_cinder_pit.png"),
        ember_flame_spire: asset_server.load("art/buildings/ember_flame_spire.png"),
        ember_ash_mine: asset_server.load("art/buildings/ember_ash_mine.png"),
    });
    commands.spawn((
        Sprite {
            image: asset_server.load("art/isometric_battlefield.png"),
            custom_size: Some(Vec2::new(1110.0, 625.0)),
            ..default()
        },
        Transform::from_xyz(0.0, 28.0, -30.0),
    ));
    spawn_static_board(&mut commands, None);
}

fn receive_packets(mut net: ResMut<ClientNet>, mut state: ResMut<SnapshotState>) {
    let mut buf = [0_u8; 16_384];
    loop {
        match net.socket.recv_from(&mut buf) {
            Ok((len, _)) => match decode_server(&buf[..len]) {
                Ok(ServerPacket::Welcome { player }) => {
                    net.player_id = Some(player.id);
                    net.team = Some(player.team);
                    net.status = format!("Joined as {} ({:?}).", player.name, player.team);
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
    let kind = snapshot.balance.race(race).buildings[0];
    send_client(
        &net,
        &ClientPacket::PlaceBuilding {
            player_id,
            kind,
            cell: GridCell { x: 0, y: 0 },
        },
    );
    net.sent_auto_build = true;
}

fn menu_and_lobby_input(keys: Res<ButtonInput<KeyCode>>, mut net: ResMut<ClientNet>) {
    if keys.just_pressed(KeyCode::Enter) {
        send_join(&mut net);
    }
    if let Some(player_id) = net.player_id {
        if keys.just_pressed(KeyCode::KeyQ) {
            send_client(
                &net,
                &ClientPacket::SetRace {
                    player_id,
                    race: RaceKind::Vanguard,
                },
            );
        }
        if keys.just_pressed(KeyCode::KeyW) {
            send_client(
                &net,
                &ClientPacket::SetRace {
                    player_id,
                    race: RaceKind::Grove,
                },
            );
        }
        if keys.just_pressed(KeyCode::KeyE) {
            send_client(
                &net,
                &ClientPacket::SetRace {
                    player_id,
                    race: RaceKind::Ember,
                },
            );
        }
    }
    if keys.just_pressed(KeyCode::Space) {
        if let Some(player_id) = net.player_id {
            send_client(
                &net,
                &ClientPacket::SetReady {
                    player_id,
                    ready: true,
                },
            );
        }
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
    let race = selected_race(&state, &net).unwrap_or(RaceKind::Vanguard);
    let buildings = active_balance(&state).race(race).buildings;
    if selection.kind.race() != race {
        selection.kind = buildings[0];
    }
    if keys.just_pressed(KeyCode::Digit1) {
        selection.kind = buildings[0];
    }
    if keys.just_pressed(KeyCode::Digit2) {
        selection.kind = buildings[1];
    }
    if keys.just_pressed(KeyCode::Digit3) {
        selection.kind = buildings[2];
    }
}

fn ui_mouse_input(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    mut selection: ResMut<BuildSelection>,
    net: Res<ClientNet>,
    state: Res<SnapshotState>,
) {
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let Some(world) = cursor_world(&windows, &camera_query) else {
        return;
    };
    if world.y > UI_PANEL_TOP_Y {
        return;
    }

    if let Some(player_id) = net.player_id {
        for (idx, race) in RaceKind::ALL.iter().enumerate() {
            let center = race_button_center(idx);
            if point_in_rect(world, center, Vec2::new(116.0, 34.0)) {
                send_client(
                    &net,
                    &ClientPacket::SetRace {
                        player_id,
                        race: *race,
                    },
                );
                return;
            }
        }
    }

    let race = selected_race(&state, &net).unwrap_or(RaceKind::Vanguard);
    let balance = active_balance(&state);
    for (idx, kind) in balance.race(race).buildings.iter().enumerate() {
        let center = command_button_center(idx);
        if point_in_rect(world, center, Vec2::splat(68.0)) {
            selection.kind = *kind;
            return;
        }
    }
}

fn placement_input(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    net: Res<ClientNet>,
    selection: Res<BuildSelection>,
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
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Ok((camera, transform)) = camera_query.single() else {
        return;
    };
    let Ok(world) = camera.viewport_to_world_2d(transform, cursor) else {
        return;
    };
    if world.y <= UI_PANEL_TOP_Y {
        return;
    }
    if let Some(picked) = pick_world_object(snapshot, world) {
        world_selection.selected = Some(picked);
        return;
    }
    if let Some(cell) = world_to_cell(team, world) {
        send_client(
            &net,
            &ClientPacket::PlaceBuilding {
                player_id,
                kind: selection.kind,
                cell,
            },
        );
        world_selection.selected = Some(SelectedObject::Cell(team, cell));
    } else {
        world_selection.selected = None;
    }
}

fn update_placement_preview(
    mut commands: Commands,
    preview_query: Query<Entity, With<PreviewEntity>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    net: Res<ClientNet>,
    selection: Res<BuildSelection>,
    state: Res<SnapshotState>,
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
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Ok((camera, transform)) = camera_query.single() else {
        return;
    };
    let Ok(world) = camera.viewport_to_world_2d(transform, cursor) else {
        return;
    };
    if world.y <= UI_PANEL_TOP_Y {
        return;
    }
    let Some(cell) = world_to_cell(team, world) else {
        return;
    };

    let occupied = snapshot
        .buildings
        .iter()
        .any(|building| building.owner == team && building.cell == cell);
    let gold = snapshot
        .players
        .iter()
        .find(|player| player.id == player_id)
        .map(|player| snapshot.economies[player.team.slot()].gold)
        .unwrap_or_default();
    let valid = !occupied
        && selection.kind.race() == team_race(snapshot, team).unwrap_or(selection.kind.race())
        && gold >= snapshot.balance.building(selection.kind).cost;
    let color = if valid {
        Color::srgb(0.30, 0.85, 0.46).with_alpha(0.45)
    } else {
        Color::srgb(0.95, 0.20, 0.18).with_alpha(0.45)
    };
    let pos = cell_to_world(team, cell);
    commands.spawn((
        Sprite::from_color(color, Vec2::splat(CELL - 1.0)),
        Transform::from_xyz(pos.x, pos.y, 12.0),
        PreviewEntity,
    ));
}

fn update_build_hover(
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    net: Res<ClientNet>,
    state: Res<SnapshotState>,
    mut hover: ResMut<BuildHover>,
) {
    let Some(world) = cursor_world(&windows, &camera_query) else {
        if hover.kind.is_some() {
            hover.kind = None;
        }
        return;
    };
    if world.y > UI_PANEL_TOP_Y {
        if hover.kind.is_some() {
            hover.kind = None;
        }
        return;
    }

    let race = selected_race(&state, &net).unwrap_or(RaceKind::Vanguard);
    let balance = active_balance(&state);
    let hovered = balance
        .race(race)
        .buildings
        .iter()
        .enumerate()
        .find_map(|(idx, kind)| {
            let center = command_button_center(idx);
            point_in_rect(world, center, Vec2::splat(68.0)).then_some(*kind)
        });

    if hover.kind != hovered {
        hover.kind = hovered;
    }
}

fn detect_combat_vfx(
    mut commands: Commands,
    state: Res<SnapshotState>,
    mut tracker: ResMut<CombatTracker>,
) {
    if !state.is_changed() {
        return;
    }
    let Some(snapshot) = &state.snapshot else {
        tracker.initialized = false;
        tracker.units.clear();
        return;
    };
    if snapshot.phase != MatchPhase::Playing {
        tracker.initialized = false;
        tracker.units.clear();
        tracker.castle_health = [0, 0];
        return;
    }

    if tracker.initialized {
        for unit in &snapshot.units {
            if let Some(previous) = tracker.units.get(&unit.id) {
                if unit.health < previous.health {
                    let damage = previous.health - unit.health;
                    let pos = unit_world_pos(unit);
                    let config = snapshot.balance.unit(unit.kind);
                    let attacker = infer_attacker(snapshot, pos, unit.owner)
                        .map(|attacker| {
                            (
                                unit_world_pos(attacker),
                                snapshot.balance.unit(attacker.kind).attack_type,
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

        for castle in &snapshot.castles {
            let slot = castle.team.slot();
            let previous = tracker.castle_health[slot];
            if previous > 0 && castle.health < previous {
                let damage = previous - castle.health;
                let pos = Vec2::new(lane_to_world(castle.team.castle_pos()), LANE_Y + 78.0);
                let attacker = infer_attacker(snapshot, pos, castle.team)
                    .map(|attacker| {
                        (
                            unit_world_pos(attacker),
                            snapshot.balance.unit(attacker.kind).attack_type,
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

fn camera_controls(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut camera_query: Query<(&mut Transform, &mut Projection), With<Camera2d>>,
) {
    let Ok((mut transform, mut projection)) = camera_query.single_mut() else {
        return;
    };
    let mut direction = Vec2::ZERO;
    if keys.pressed(KeyCode::ArrowLeft) {
        direction.x -= 1.0;
    }
    if keys.pressed(KeyCode::ArrowRight) {
        direction.x += 1.0;
    }
    if keys.pressed(KeyCode::ArrowUp) {
        direction.y += 1.0;
    }
    if keys.pressed(KeyCode::ArrowDown) {
        direction.y -= 1.0;
    }
    if direction != Vec2::ZERO {
        let speed = 360.0 * time.delta_secs();
        transform.translation.x += direction.normalize().x * speed;
        transform.translation.y += direction.normalize().y * speed;
        transform.translation.x = transform.translation.x.clamp(-140.0, 140.0);
        transform.translation.y = transform.translation.y.clamp(-70.0, 80.0);
    }
    if let Projection::Orthographic(orthographic) = projection.as_mut() {
        if keys.just_pressed(KeyCode::Equal) || keys.just_pressed(KeyCode::NumpadAdd) {
            orthographic.scale = (orthographic.scale * 0.88).clamp(0.72, 1.35);
        }
        if keys.just_pressed(KeyCode::Minus) || keys.just_pressed(KeyCode::NumpadSubtract) {
            orthographic.scale = (orthographic.scale * 1.12).clamp(0.72, 1.35);
        }
        if keys.just_pressed(KeyCode::Home) {
            orthographic.scale = 1.0;
            transform.translation.x = 0.0;
            transform.translation.y = 0.0;
        }
    }
}

fn redraw_scene(
    mut commands: Commands,
    scene_query: Query<Entity, With<SceneEntity>>,
    state: Res<SnapshotState>,
    net: Res<ClientNet>,
    world_selection: Res<WorldSelection>,
    unit_assets: Res<UnitSpriteAssets>,
) {
    if !state.is_changed() && !world_selection.is_changed() {
        return;
    }
    for entity in &scene_query {
        commands.entity(entity).despawn();
    }

    spawn_static_board(&mut commands, net.team);

    let Some(snapshot) = &state.snapshot else {
        return;
    };

    for building in &snapshot.buildings {
        let selected = world_selection.selected == Some(SelectedObject::Building(building.id));
        spawn_building(&mut commands, building, &snapshot.balance, selected);
    }
    for unit in &snapshot.units {
        let selected = world_selection.selected == Some(SelectedObject::Unit(unit.id));
        spawn_unit(
            &mut commands,
            unit,
            &snapshot.balance,
            &unit_assets,
            selected,
            snapshot.tick,
        );
    }
    for castle in &snapshot.castles {
        let x = lane_to_world(castle.team.castle_pos());
        let health_pct = castle.health.max(0) as f32 / castle.max_health as f32;
        if world_selection.selected == Some(SelectedObject::Castle(castle.team)) {
            spawn_rect(
                &mut commands,
                Vec2::new(x, LANE_Y + 92.0),
                Vec2::new(110.0, 20.0),
                Color::srgba(0.95, 0.75, 0.26, 0.36),
                4.7,
            );
        }
        spawn_rect(
            &mut commands,
            Vec2::new(x, LANE_Y + 92.0),
            Vec2::new(96.0, 10.0),
            Color::srgba(0.05, 0.04, 0.03, 0.86),
            5.0,
        );
        spawn_rect(
            &mut commands,
            Vec2::new(x - (96.0 * (1.0 - health_pct) / 2.0), LANE_Y + 92.0),
            Vec2::new(96.0 * health_pct, 10.0),
            Color::srgb(0.15, 0.78, 0.34),
            6.0,
        );
    }
}

fn redraw_game_ui(
    mut commands: Commands,
    ui_query: Query<Entity, With<UiEntity>>,
    state: Res<SnapshotState>,
    net: Res<ClientNet>,
    selection: Res<BuildSelection>,
    hover: Res<BuildHover>,
    world_selection: Res<WorldSelection>,
    building_icons: Res<BuildingIconAssets>,
) {
    if !state.is_changed()
        && !net.is_changed()
        && !selection.is_changed()
        && !hover.is_changed()
        && !world_selection.is_changed()
    {
        return;
    }
    for entity in &ui_query {
        commands.entity(entity).despawn();
    }

    let balance = active_balance(&state);
    spawn_ui_panel(
        &mut commands,
        Vec2::new(0.0, TOP_BAR_Y),
        Vec2::new(1048.0, 46.0),
        40.0,
    );
    spawn_ui_label(
        &mut commands,
        "CASTLE LANES",
        Vec2::new(-506.0, TOP_BAR_Y + 6.0),
        20.0,
        TEXT_GOLD,
        45.0,
        Anchor::TOP_LEFT,
        Justify::Left,
    );
    spawn_ui_label(
        &mut commands,
        &format!("{}  |  {}", net.player_name, net.server_addr),
        Vec2::new(-506.0, TOP_BAR_Y - 15.0),
        11.0,
        Color::srgb(0.71, 0.68, 0.58),
        45.0,
        Anchor::TOP_LEFT,
        Justify::Left,
    );

    let (phase_line, economy_line, message_line) = ui_summary(&state, &net);
    spawn_ui_label(
        &mut commands,
        &phase_line,
        Vec2::new(0.0, TOP_BAR_Y + 10.0),
        15.0,
        TEXT_PARCHMENT,
        45.0,
        Anchor::CENTER,
        Justify::Center,
    );
    spawn_ui_label(
        &mut commands,
        &economy_line,
        Vec2::new(508.0, TOP_BAR_Y - 12.0),
        13.0,
        TEXT_GOLD,
        45.0,
        Anchor::CENTER_RIGHT,
        Justify::Right,
    );

    spawn_ui_panel(
        &mut commands,
        Vec2::new(0.0, UI_PANEL_Y),
        Vec2::new(1048.0, 134.0),
        40.0,
    );
    spawn_ui_panel(
        &mut commands,
        Vec2::new(-430.0, UI_PANEL_Y + 9.0),
        Vec2::new(180.0, 82.0),
        42.0,
    );
    spawn_ui_panel(
        &mut commands,
        Vec2::new(-72.0, UI_PANEL_Y + 9.0),
        Vec2::new(302.0, 82.0),
        42.0,
    );
    spawn_ui_panel(
        &mut commands,
        Vec2::new(352.0, UI_PANEL_Y + 9.0),
        Vec2::new(300.0, 82.0),
        42.0,
    );

    let race = selected_race(&state, &net).unwrap_or(RaceKind::Vanguard);
    let local_title = local_player_title(&state, &net);
    spawn_ui_label(
        &mut commands,
        &local_title,
        Vec2::new(-510.0, UI_PANEL_Y + 42.0),
        14.0,
        TEXT_PARCHMENT,
        46.0,
        Anchor::TOP_LEFT,
        Justify::Left,
    );
    spawn_ui_label(
        &mut commands,
        &format!("Race: {}", balance.race(race).name),
        Vec2::new(-510.0, UI_PANEL_Y + 20.0),
        12.0,
        Color::srgb(0.72, 0.91, 0.82),
        46.0,
        Anchor::TOP_LEFT,
        Justify::Left,
    );
    spawn_ui_label(
        &mut commands,
        "Q / W / E or click",
        Vec2::new(-510.0, UI_PANEL_Y - 2.0),
        11.0,
        Color::srgb(0.64, 0.61, 0.52),
        46.0,
        Anchor::TOP_LEFT,
        Justify::Left,
    );
    for (idx, race_kind) in RaceKind::ALL.iter().enumerate() {
        let center = race_button_center(idx);
        let is_selected = *race_kind == race;
        let color = if is_selected {
            Color::srgba(0.25, 0.20, 0.11, 0.98)
        } else {
            Color::srgba(0.11, 0.10, 0.09, 0.90)
        };
        spawn_ui_button(
            &mut commands,
            center,
            Vec2::new(116.0, 34.0),
            color,
            is_selected,
        );
        spawn_ui_label(
            &mut commands,
            balance.race(*race_kind).name.as_str(),
            Vec2::new(center.x, center.y - 1.0),
            12.0,
            TEXT_PARCHMENT,
            47.0,
            Anchor::CENTER,
            Justify::Center,
        );
    }

    let selected_details =
        selected_object_details(&state, &world_selection, selection.kind).unwrap_or(message_line);
    spawn_ui_label(
        &mut commands,
        &selected_details,
        Vec2::new(-214.0, UI_PANEL_Y + 42.0),
        12.0,
        TEXT_PARCHMENT,
        46.0,
        Anchor::TOP_LEFT,
        Justify::Left,
    );
    spawn_ui_label(
        &mut commands,
        "Enter join  |  Space ready  |  R rematch",
        Vec2::new(-214.0, UI_PANEL_Y - 16.0),
        10.5,
        Color::srgb(0.66, 0.62, 0.51),
        46.0,
        Anchor::TOP_LEFT,
        Justify::Left,
    );

    spawn_ui_label(
        &mut commands,
        "BUILD",
        Vec2::new(216.0, UI_PANEL_Y + 45.0),
        11.0,
        TEXT_GOLD,
        46.0,
        Anchor::TOP_LEFT,
        Justify::Left,
    );
    for (idx, kind) in balance.race(race).buildings.iter().enumerate() {
        let config = balance.building(*kind);
        let center = command_button_center(idx);
        let selected = selection.kind == *kind;
        let hovered = hover.kind == Some(*kind);
        let color = if hovered {
            Color::srgba(0.22, 0.18, 0.10, 0.98)
        } else {
            Color::srgba(0.08, 0.07, 0.055, 0.96)
        };
        spawn_ui_button(&mut commands, center, Vec2::splat(68.0), color, selected);
        spawn_ui_image(
            &mut commands,
            building_icon_handle(&building_icons, *kind),
            center,
            Vec2::splat(if hovered { 62.0 } else { 58.0 }),
            46.2,
        );
        if let Some(unit_kind) = config.spawned_unit {
            let unit = balance.unit(unit_kind);
            spawn_ui_rect(
                &mut commands,
                Vec2::new(center.x - 18.0, center.y + 19.0),
                Vec2::new(22.0, 7.0),
                attack_type_color(unit.attack_type),
                47.0,
            );
            spawn_ui_rect(
                &mut commands,
                Vec2::new(center.x + 18.0, center.y + 19.0),
                Vec2::new(22.0, 7.0),
                armor_type_color(unit.armor_type),
                47.0,
            );
        }
        spawn_ui_label(
            &mut commands,
            &command_button_badge(&balance, *kind),
            Vec2::new(center.x, center.y - 23.0),
            10.0,
            Color::srgb(0.98, 0.90, 0.66),
            47.0,
            Anchor::CENTER,
            Justify::Center,
        );
    }
    spawn_ui_label(
        &mut commands,
        &selected_build_details(&balance, selection.kind),
        Vec2::new(462.0, UI_PANEL_Y + 28.0),
        11.0,
        TEXT_PARCHMENT,
        46.0,
        Anchor::TOP_LEFT,
        Justify::Left,
    );

    if let Some(kind) = hover.kind.or(Some(selection.kind)) {
        spawn_build_tooltip(&mut commands, &balance, kind);
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

fn point_in_rect(point: Vec2, center: Vec2, size: Vec2) -> bool {
    let half = size * 0.5;
    point.x >= center.x - half.x
        && point.x <= center.x + half.x
        && point.y >= center.y - half.y
        && point.y <= center.y + half.y
}

fn command_button_center(idx: usize) -> Vec2 {
    Vec2::new(276.0 + idx as f32 * 76.0, UI_PANEL_Y + 1.0)
}

fn race_button_center(idx: usize) -> Vec2 {
    Vec2::new(-452.0 + idx as f32 * 126.0, UI_PANEL_Y - 33.0)
}

fn pick_world_object(snapshot: &MatchSnapshot, world: Vec2) -> Option<SelectedObject> {
    let unit_pick = snapshot
        .units
        .iter()
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
        let center = cell_to_world(building.owner, building.cell);
        if point_in_rect(world, center, Vec2::splat(CELL)) {
            return Some(SelectedObject::Building(building.id));
        }
    }

    for castle in &snapshot.castles {
        let center = Vec2::new(lane_to_world(castle.team.castle_pos()), LANE_Y + 25.0);
        if point_in_rect(world, center, Vec2::new(96.0, 160.0)) {
            return Some(SelectedObject::Castle(castle.team));
        }
    }

    None
}

fn selected_object_details(
    state: &SnapshotState,
    selection: &WorldSelection,
    build_selection: BuildingKind,
) -> Option<String> {
    let snapshot = state.snapshot.as_ref()?;
    let balance = &snapshot.balance;
    match selection.selected? {
        SelectedObject::Unit(id) => {
            let unit = snapshot.units.iter().find(|unit| unit.id == id)?;
            let config = balance.unit(unit.kind);
            Some(format!(
                "{} {:?} {}\nHP {}/{}   {} {} dmg\nMv {:.1}   AS {:.2}/s   AtkR {:.1}\nArmor {}",
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
                config.armor_type.label()
            ))
        }
        SelectedObject::Building(id) => {
            let building = snapshot
                .buildings
                .iter()
                .find(|building| building.id == id)?;
            let config = balance.building(building.kind);
            let spawn_line = config
                .spawned_unit
                .map(|kind| unit_spawn_line(balance, kind))
                .unwrap_or_else(|| format!("Income +{}", config.income_bonus));
            Some(format!(
                "{} {:?}\n{}\nNext wave in {:.1}s",
                config.name,
                building.owner,
                spawn_line,
                building.spawn_timer.max(0.0)
            ))
        }
        SelectedObject::Castle(team) => {
            let castle = &snapshot.castles[team.slot()];
            Some(format!(
                "{team:?} Castle\nHP {}/{}   Armor {}",
                castle.health.max(0),
                castle.max_health,
                castle.armor_type.label()
            ))
        }
        SelectedObject::Cell(team, cell) => {
            let config = balance.building(build_selection);
            Some(format!(
                "{team:?} build cell {},{}\nSelected: {}\n{}",
                cell.x,
                cell.y,
                config.name,
                selected_build_details(balance, build_selection)
            ))
        }
    }
}

fn selected_build_details(balance: &BalanceConfig, kind: BuildingKind) -> String {
    let config = balance.building(kind);
    if let Some(unit_kind) = config.spawned_unit {
        let unit = balance.unit(unit_kind);
        format!(
            "{}\n{}g\n{} {}\nR {:.1} AS {:.1}",
            config.name,
            config.cost,
            unit.attack_mode.short_label(),
            unit.attack_type.short_label(),
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
    spawn_ui_panel(commands, pos, Vec2::new(326.0, 108.0), 49.0);
    spawn_ui_label(
        commands,
        &building_tooltip_text(balance, kind),
        Vec2::new(pos.x - 148.0, pos.y + 38.0),
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
            "{}\nCost: {} gold   Spawns every {:.1}s\nProduces: {} ({})\nDamage: {} {}   Armor: {}\nMove: {:.1}   AtkR: {:.1}   AS: {:.2}/s",
            config.name,
            config.cost,
            config.spawn_interval.unwrap_or_default(),
            unit.name,
            unit.attack_mode.label(),
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

fn unit_spawn_line(balance: &BalanceConfig, kind: UnitKind) -> String {
    let unit = balance.unit(kind);
    format!(
        "Spawns {}: {} {}, {} dmg, Mv {:.1}, AtkR {:.1}, AS {:.2}/s",
        unit.name,
        unit.attack_mode.label(),
        unit.attack_type.label(),
        damage_range_text(unit.damage, unit.damage_variance),
        unit.speed,
        unit.attack_range,
        attacks_per_second(unit.attack_interval)
    )
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
    target_pos: Vec2,
    target_team: Team,
) -> Option<&'a Unit> {
    snapshot
        .units
        .iter()
        .filter(|unit| unit.owner != target_team)
        .map(|unit| {
            let unit_config = snapshot.balance.unit(unit.kind);
            let pos = unit_world_pos(unit);
            let lane_distance = (unit.lane_pos - world_to_lane(target_pos.x)).abs();
            let score = pos.distance(target_pos) + lane_distance * 3.0;
            (unit, score, unit_config.attack_range + 3.5)
        })
        .filter(|(_, score, range)| *score <= range * 9.0)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(unit, _, _)| unit)
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
        BuildingKind::GroveRootDen => assets.grove_root_den.clone(),
        BuildingKind::GroveThornSpire => assets.grove_thorn_spire.clone(),
        BuildingKind::GroveBloomWell => assets.grove_bloom_well.clone(),
        BuildingKind::EmberCinderPit => assets.ember_cinder_pit.clone(),
        BuildingKind::EmberFlameSpire => assets.ember_flame_spire.clone(),
        BuildingKind::EmberAshMine => assets.ember_ash_mine.clone(),
    }
}

fn unit_sprite_handle(assets: &UnitSpriteAssets, kind: UnitKind) -> Handle<Image> {
    match kind {
        UnitKind::VanguardGuard => assets.vanguard_guard.clone(),
        UnitKind::VanguardArcher => assets.vanguard_archer.clone(),
        UnitKind::GroveBruiser => assets.grove_bruiser.clone(),
        UnitKind::GroveNeedler => assets.grove_needler.clone(),
        UnitKind::EmberRunner => assets.ember_runner.clone(),
        UnitKind::EmberCaster => assets.ember_caster.clone(),
    }
}

fn unit_sprite_size(kind: UnitKind) -> Vec2 {
    match kind {
        UnitKind::GroveBruiser => Vec2::splat(72.0),
        UnitKind::VanguardGuard => Vec2::splat(66.0),
        UnitKind::VanguardArcher => Vec2::splat(62.0),
        UnitKind::GroveNeedler => Vec2::splat(64.0),
        UnitKind::EmberRunner => Vec2::splat(61.0),
        UnitKind::EmberCaster => Vec2::splat(64.0),
    }
}

fn unit_world_pos(unit: &Unit) -> Vec2 {
    let x = lane_to_world(unit.lane_pos);
    let y = match unit.kind.race_like() {
        RaceKind::Vanguard | RaceKind::Ember => -16.0,
        RaceKind::Grove => 18.0,
    };
    Vec2::new(x, y)
}

fn world_to_lane(x: f32) -> f32 {
    ((x / (WORLD_W - 120.0)) + 0.5) * LANE_LENGTH
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
                        (snapshot.balance.sudden_death_start - snapshot.elapsed_secs).max(0.0)
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

fn local_player_title(state: &SnapshotState, net: &ClientNet) -> String {
    let Some(snapshot) = &state.snapshot else {
        return "Commander: not joined".to_string();
    };
    let Some(player_id) = net.player_id else {
        return "Commander: not joined".to_string();
    };
    let Some(player) = current_player(snapshot, player_id) else {
        return "Commander: connecting".to_string();
    };
    let ready = if player.ready { "ready" } else { "not ready" };
    format!("{}  {:?}  {}", player.name, player.team, ready)
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
    state
        .snapshot
        .as_ref()
        .map(|snapshot| snapshot.balance.clone())
        .unwrap_or_default()
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

fn spawn_static_board(commands: &mut Commands, team: Option<Team>) {
    spawn_rect(
        commands,
        Vec2::new(0.0, LANE_Y),
        Vec2::new(WORLD_W - 120.0, 32.0),
        Color::srgba(0.12, 0.10, 0.07, 0.34),
        -1.0,
    );
    spawn_rect(
        commands,
        Vec2::new(0.0, LANE_Y),
        Vec2::new(WORLD_W - 160.0, 3.0),
        Color::srgba(0.80, 0.64, 0.34, 0.42),
        0.0,
    );

    for side in [Team::Left, Team::Right] {
        for x in 0..GRID_W {
            for y in 0..GRID_H {
                let pos = cell_to_world(side, GridCell { x, y });
                let color = if Some(side) == team {
                    Color::srgba(0.26, 0.56, 0.72, 0.34)
                } else {
                    Color::srgba(0.07, 0.06, 0.05, 0.26)
                };
                spawn_diamond(commands, pos, Vec2::splat(CELL - 9.0), color, 0.5);
            }
        }
    }
}

fn spawn_building(
    commands: &mut Commands,
    building: &Building,
    balance: &BalanceConfig,
    selected: bool,
) {
    let pos = cell_to_world(building.owner, building.cell);
    let config = balance.building(building.kind);
    let color = Color::srgb(config.color[0], config.color[1], config.color[2]);
    if selected {
        spawn_diamond(
            commands,
            Vec2::new(pos.x, pos.y - 2.0),
            Vec2::new(CELL + 4.0, CELL + 4.0),
            Color::srgba(0.95, 0.76, 0.24, 0.42),
            2.5,
        );
    }
    spawn_diamond(
        commands,
        Vec2::new(pos.x, pos.y - 8.0),
        Vec2::new(CELL - 8.0, CELL - 8.0),
        Color::srgba(0.04, 0.035, 0.025, 0.58),
        2.7,
    );
    spawn_rect(
        commands,
        Vec2::new(pos.x, pos.y + 3.0),
        Vec2::new(CELL - 20.0, CELL - 16.0),
        color,
        3.0,
    );
    spawn_rect(
        commands,
        Vec2::new(pos.x, pos.y + 20.0),
        Vec2::new(CELL - 27.0, 9.0),
        color.mix(&Color::WHITE, 0.24),
        4.0,
    );
    spawn_label(
        commands,
        &config.label,
        Vec2::new(pos.x, pos.y + 1.0),
        18.0,
        Color::srgb(0.08, 0.07, 0.05),
        13.0,
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

fn spawn_label(commands: &mut Commands, text: &str, pos: Vec2, size: f32, color: Color, z: f32) {
    commands.spawn((
        Text2d::new(text),
        TextFont::from_font_size(size),
        TextColor(color),
        TextLayout::new_with_justify(Justify::Center),
        Anchor::CENTER,
        Transform::from_xyz(pos.x, pos.y + 1.0, z),
        SceneEntity,
    ));
}

fn spawn_ui_panel(commands: &mut Commands, pos: Vec2, size: Vec2, z: f32) {
    spawn_ui_rect(commands, pos, size, PANEL_BG, z);
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
}

fn spawn_ui_button(commands: &mut Commands, pos: Vec2, size: Vec2, color: Color, selected: bool) {
    spawn_ui_rect(
        commands,
        pos,
        size,
        Color::srgba(0.02, 0.018, 0.015, 0.96),
        44.0,
    );
    spawn_ui_rect(commands, pos, size - Vec2::splat(5.0), color, 45.0);
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
        46.0,
    );
    spawn_ui_rect(
        commands,
        Vec2::new(pos.x, pos.y - size.y * 0.5 + 3.0),
        Vec2::new(size.x, 4.0),
        edge,
        46.0,
    );
}

fn spawn_ui_rect(commands: &mut Commands, pos: Vec2, size: Vec2, color: Color, z: f32) {
    commands.spawn((
        Sprite::from_color(color, size),
        Transform::from_xyz(pos.x, pos.y, z),
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
        UiEntity,
    ));
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

fn cell_to_world(team: Team, cell: GridCell) -> Vec2 {
    let base_x = match team {
        Team::Left => -350.0,
        Team::Right => 350.0,
    };
    let x = base_x + (cell.x as f32 - 1.5) * CELL;
    let y = -162.0 + (cell.y as f32 - 1.0) * CELL;
    Vec2::new(x, y)
}

fn world_to_cell(team: Team, world: Vec2) -> Option<GridCell> {
    for x in 0..GRID_W {
        for y in 0..GRID_H {
            let center = cell_to_world(team, GridCell { x, y });
            let half = CELL * 0.5;
            if world.x >= center.x - half
                && world.x <= center.x + half
                && world.y >= center.y - half
                && world.y <= center.y + half
            {
                return Some(GridCell { x, y });
            }
        }
    }
    None
}
