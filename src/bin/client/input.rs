//! input systems split out of the monolithic client (plan.md Phase 1 item 8).
#![allow(unused_imports)]
pub(crate) use super::audio::*;
pub(crate) use super::net::*;
pub(crate) use super::scene::*;
pub(crate) use super::ui::*;
pub(crate) use super::vfx::*;
use super::*;

pub(crate) fn menu_and_lobby_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut net: ResMut<ClientNet>,
    mut state: ResMut<SnapshotState>,
    mut overlays: ResMut<UiOverlays>,
    world_selection: Res<WorldSelection>,
) {
    if keys.just_pressed(KeyCode::KeyH) {
        overlays.help = !overlays.help;
    }
    if keys.just_pressed(KeyCode::KeyO) {
        overlays.settings = !overlays.settings;
    }
    if keys.just_pressed(KeyCode::Enter) {
        if !net.connected {
            send_join(&mut net);
        } else if net.player_id.is_none() {
            send_client(
                &net,
                &ClientPacket::CreateGame {
                    name: format!("{}'s Game", net.player_name),
                    team_size: None,
                    random_factions: false,
                },
            );
        }
    }
    if net.connected && net.player_id.is_none() {
        if keys.just_pressed(KeyCode::KeyC) {
            send_client(
                &net,
                &ClientPacket::CreateGame {
                    name: format!("{}'s Game", net.player_name),
                    team_size: None,
                    random_factions: false,
                },
            );
        }
        if keys.just_pressed(KeyCode::KeyL) {
            send_client(&net, &ClientPacket::ListGames);
        }
        for (key, idx) in [
            (KeyCode::Digit1, 0),
            (KeyCode::Digit2, 1),
            (KeyCode::Digit3, 2),
            (KeyCode::Digit4, 3),
            (KeyCode::Digit5, 4),
        ] {
            if keys.just_pressed(key) {
                if let Some(game) = state.games.get(idx) {
                    send_client(&net, &ClientPacket::JoinGame { game_id: game.id, spectator: false });
                }
            }
        }
    }
    if keys.just_pressed(KeyCode::Escape) && net.player_id.is_some() {
        send_client(&net, &ClientPacket::LeaveGame);
        net.game_id = None;
        net.player_id = None;
        net.team = None;
        state.snapshot = None;
        net.sent_auto_game = false;
        net.sent_auto_race = false;
        net.sent_auto_ready = false;
        net.sent_auto_build = false;
        send_client(&net, &ClientPacket::ListGames);
    }
    if keys.just_pressed(KeyCode::KeyR) {
        if let Some(player_id) = net.player_id {
            send_client(&net, &ClientPacket::VoteRematch { player_id });
        }
    }
    if keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight])
        && keys.just_pressed(KeyCode::KeyQ)
    {
        if let Some(player_id) = net.player_id {
            send_client(&net, &ClientPacket::Surrender { player_id });
        }
    }
    if keys.just_pressed(KeyCode::Backspace) {
        sell_selected_building(&mut net, &state, &world_selection);
    }
    if keys.just_pressed(KeyCode::KeyU) {
        upgrade_selected_building(&mut net, &state, &world_selection, 0);
    }
    if keys.just_pressed(KeyCode::KeyI) {
        upgrade_selected_building(&mut net, &state, &world_selection, 1);
    }
}

/// Upgrade the selected own building into one of its listed branches
/// (branch index 0 = key U, 1 = key I; plan.md Phase 2 item 3).
fn upgrade_selected_building(
    net: &mut ClientNet,
    state: &SnapshotState,
    world_selection: &WorldSelection,
    branch: usize,
) {
    let (Some(snapshot), Some(player_id)) = (&state.snapshot, net.player_id) else {
        return;
    };
    let Some(SelectedObject::Building(building_id)) = world_selection.selected else {
        return;
    };
    let Some(building) = snapshot.buildings.iter().find(|b| b.id == building_id) else {
        return;
    };
    let Some(player) = snapshot.players.iter().find(|p| p.id == player_id) else {
        return;
    };
    if side_of_player(snapshot, building.owner) != player.team {
        return;
    }
    let Some(target) = state
        .balance
        .building(building.kind)
        .upgrades
        .get(branch)
        .copied()
    else {
        return;
    };
    let seq = net.next_seq;
    net.next_seq = net.next_seq.wrapping_add(1);
    net.pending_placement = None;
    send_client(
        net,
        &ClientPacket::UpgradeBuilding {
            player_id,
            building_id,
            to: target,
            seq: Some(seq),
        },
    );
}

/// Sell the selected own building for a 70% refund (Delete/Backspace).
pub(crate) fn sell_selected_building(
    net: &mut ClientNet,
    state: &SnapshotState,
    world_selection: &WorldSelection,
) {
    let (Some(snapshot), Some(player_id)) = (&state.snapshot, net.player_id) else {
        return;
    };
    let Some(SelectedObject::Building(building_id)) = world_selection.selected else {
        return;
    };
    let Some(building) = snapshot.buildings.iter().find(|b| b.id == building_id) else {
        return;
    };
    let Some(player) = snapshot.players.iter().find(|p| p.id == player_id) else {
        return;
    };
    if building.owner != player_id {
        return;
    }
    let seq = net.next_seq;
    net.next_seq = net.next_seq.wrapping_add(1);
    net.pending_placement = None;
    send_client(
        net,
        &ClientPacket::SellBuilding {
            player_id,
            building_id,
            seq: Some(seq),
        },
    );
}

pub(crate) fn build_selection_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut selection: ResMut<BuildSelection>,
    net: Res<ClientNet>,
    state: Res<SnapshotState>,
    mut sfx: ResMut<SfxQueue>,
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
                if selection.kind != Some(*kind) {
                    sfx.push(Sfx::UiClick);
                }
                selection.kind = Some(*kind);
            }
        }
    }
}

pub(crate) fn ui_mouse_input(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut selection: ResMut<BuildSelection>,
    net: Res<ClientNet>,
    state: Res<SnapshotState>,
    mut sfx: ResMut<SfxQueue>,
) {
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let Some(screen) = cursor_screen_pos(&windows) else {
        return;
    };

    if net.connected && net.player_id.is_none() {
        if point_in_rect(screen, lobby_create_button_center(), lobby_button_size()) {
            sfx.push(Sfx::UiClick);
            send_client(
                &net,
                &ClientPacket::CreateGame {
                    name: format!("{}'s Game", net.player_name),
                    team_size: None,
                    random_factions: false,
                },
            );
            return;
        }
        if point_in_rect(screen, lobby_refresh_button_center(), lobby_button_size()) {
            sfx.push(Sfx::UiClick);
            send_client(&net, &ClientPacket::ListGames);
            return;
        }
        for (idx, game) in state.games.iter().take(5).enumerate() {
            if point_in_rect(screen, lobby_game_row_center(idx), lobby_game_row_size()) {
                sfx.push(Sfx::UiClick);
                send_client(&net, &ClientPacket::JoinGame { game_id: game.id, spectator: false });
                return;
            }
        }
        return;
    }

    if race_selection_popup_open(&state, &net) {
        if let (Some(snapshot), Some(player_id)) = (&state.snapshot, net.player_id) {
            let selected_race = current_player(snapshot, player_id).and_then(|player| player.race);
            if selected_race.is_some()
                && point_in_rect(screen, ready_button_center(), ready_button_size())
            {
                sfx.push(Sfx::UiClick);
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
                    sfx.push(Sfx::UiClick);
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

pub(crate) fn retry_pending_placement(mut net: ResMut<ClientNet>) {
    let Some(pending) = net.pending_placement else {
        return;
    };
    let since_last = pending.last_sent.elapsed();
    let since_first = pending.first_sent.elapsed();
    if since_last < PLACEMENT_RETRY_INTERVAL {
        return;
    }
    if since_first > PLACEMENT_RETRY_WINDOW {
        net.pending_placement = None;
        net.status = "Last placement was not acknowledged by the server.".to_string();
        return;
    }
    send_client(
        &net,
        &ClientPacket::PlaceBuilding {
            player_id: net.player_id.unwrap_or(PlayerId(0)),
            kind: pending.kind,
            lane: pending.lane,
            zone: pending.zone,
            cell: pending.cell,
            seq: Some(pending.seq),
        },
    );
    net.pending_placement = Some(PendingPlacement {
        last_sent: Instant::now(),
        ..pending
    });
}

pub(crate) fn placement_input(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    mut net: ResMut<ClientNet>,
    mut selection: ResMut<BuildSelection>,
    mut world_selection: ResMut<WorldSelection>,
    state: Res<SnapshotState>,
    mut sfx: ResMut<SfxQueue>,
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

    let Some((lane, zone, cell)) = world_to_build_slot(snapshot, player_id, team, world) else {
        return;
    };
    let balance = active_balance(&state);
    if !can_place_building(&balance, snapshot, player_id, team, kind, lane, zone, cell) {
        sfx.push(Sfx::BuildError);
        return;
    }
    sfx.push(Sfx::BuildPlace);
    let seq = net.next_seq;
    net.next_seq = net.next_seq.wrapping_add(1);
    send_client(
        &net,
        &ClientPacket::PlaceBuilding {
            player_id,
            kind,
            lane,
            zone,
            cell,
            seq: Some(seq),
        },
    );
    net.pending_placement = Some(PendingPlacement {
        seq,
        kind,
        lane,
        zone,
        cell,
        first_sent: Instant::now(),
        last_sent: Instant::now(),
    });
    world_selection.selected = Some(SelectedObject::Cell(team, lane, zone, cell));
    selection.kind = None;
}

pub(crate) fn can_place_building(
    balance: &BalanceConfig,
    snapshot: &MatchSnapshot,
    player_id: PlayerId,
    team: Team,
    kind: BuildingKind,
    lane: Lane,
    zone: BuildZone,
    cell: GridCell,
) -> bool {
    // Team play: each player builds only in their assigned lane
    let player_index = snapshot
        .players
        .iter()
        .position(|p| p.id == player_id)
        .unwrap_or(0);
    let team_size = snapshot.players.len() / 2;
    if team_size > 1 {
        let side_index = player_index % team_size;
        let assigned = Lane::for_player(team, side_index);
        if lane != assigned {
            return false;
        }
    }
    let occupied = snapshot.buildings.iter().any(|building| {
        side_of_player(snapshot, building.owner) == team
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
        .map(|player| snapshot.economies[player_index_of(snapshot, player.id)].gold)
        .unwrap_or_default();
    kind.race() == team_race(snapshot, team).unwrap_or(kind.race())
        && gold >= balance.building(kind).cost
}

pub(crate) fn update_placement_preview(
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
    let slot = world_to_build_slot(snapshot, player_id, team, world);
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

pub(crate) fn update_build_hover(
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

pub(crate) fn update_world_hover(
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

// ---- Audio (plan.md P0-3) ----

pub(crate) fn camera_controls(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    windows: Query<&Window, With<PrimaryWindow>>,
    net: Res<ClientNet>,
    state: Res<SnapshotState>,
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
            transform.translation.x = camera_home
                .home_override
                .map(|fraction| (fraction - 0.5) * (WORLD_W - 120.0))
                .unwrap_or_else(|| home_camera_x(team, scale, windows.single().ok()));
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

pub(crate) fn home_camera_x(team: Team, scale: f32, window: Option<&Window>) -> f32 {
    let castle_x = lane_to_world(team.castle_pos());
    let desired = castle_x + team.direction() * 290.0 * scale;
    camera_x_bounds(scale, window)
        .map(|(min_x, max_x)| desired.clamp(min_x, max_x))
        .unwrap_or(desired)
}

pub(crate) fn clamp_camera(transform: &mut Transform, scale: f32, window: Option<&Window>) {
    if let Some((min_x, max_x)) = camera_x_bounds(scale, window) {
        transform.translation.x = transform.translation.x.clamp(min_x, max_x);
    }
    let y_bound =
        (MAP_H * 0.5 - window.map(|w| w.height() * scale * 0.5).unwrap_or(360.0)).max(0.0) + 36.0;
    transform.translation.y = transform.translation.y.clamp(-y_bound, y_bound);
}

pub(crate) fn camera_x_bounds(scale: f32, window: Option<&Window>) -> Option<(f32, f32)> {
    let half_map = MAP_W * 0.5;
    let half_view = window.map(|w| w.width() * scale * 0.5)?;
    let bound = (half_map - half_view).max(0.0);
    Some((-bound, bound))
}

pub(crate) fn cursor_world(
    windows: &Query<&Window, With<PrimaryWindow>>,
    camera_query: &Query<(&Camera, &GlobalTransform), With<Camera2d>>,
) -> Option<Vec2> {
    let window = windows.single().ok()?;
    let cursor = window.cursor_position()?;
    let (camera, transform) = camera_query.single().ok()?;
    camera.viewport_to_world_2d(transform, cursor).ok()
}

pub(crate) fn cursor_screen_pos(windows: &Query<&Window, With<PrimaryWindow>>) -> Option<Vec2> {
    let window = windows.single().ok()?;
    let cursor = window.cursor_position()?;
    Some(Vec2::new(
        cursor.x - window.width() * 0.5,
        window.height() * 0.5 - cursor.y,
    ))
}

pub(crate) fn point_in_rect(point: Vec2, center: Vec2, size: Vec2) -> bool {
    let half = size * 0.5;
    point.x >= center.x - half.x
        && point.x <= center.x + half.x
        && point.y >= center.y - half.y
        && point.y <= center.y + half.y
}

pub(crate) fn pick_world_object(
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
        let center = cell_to_world(
            side_of_player(snapshot, building.owner),
            building.lane,
            building.zone,
            building.cell,
        );
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

pub(crate) fn world_to_build_slot(
    snapshot: &MatchSnapshot,
    player_id: PlayerId,
    team: Team,
    world: Vec2,
) -> Option<(Lane, BuildZone, GridCell)> {
    // Team play: only the assigned lane is clickable
    let team_size = snapshot.players.len() / 2;
    let team_players: Vec<_> = snapshot
        .players
        .iter()
        .filter(|p| p.team == team)
        .collect();
    let side_index = team_players
        .iter()
        .position(|p| snapshot.players.iter().any(|q| q.id == player_id && q.name == p.name))
        .unwrap_or(0);
    let lanes: Vec<Lane> = if team_size > 1 {
        vec![Lane::for_player(team, side_index)]
    } else {
        Lane::ALL.to_vec()
    };
    for lane in lanes {
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
