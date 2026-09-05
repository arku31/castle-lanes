//! ui systems split out of the monolithic client (plan.md Phase 1 item 8).
#![allow(unused_imports)]
pub(crate) use super::audio::*;
pub(crate) use super::input::*;
pub(crate) use super::net::*;
pub(crate) use super::scene::*;
pub(crate) use super::vfx::*;
use super::*;

pub(crate) fn redraw_game_ui(
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
    help: Res<HelpOverlay>,
    mut hints: ResMut<MatchHints>,
    settings_overlay: Res<SettingsOverlay>,
    settings: Res<ClientSettings>,
) {
    for entity in &ui_query {
        commands.entity(entity).despawn();
    }

    let balance = active_balance(&state);
    let (phase_line, _economy_line, _message_line) = ui_summary(&state, &net);
    let selected_race = selected_race(&state, &net);
    let race_popup_open = race_selection_popup_open(&state, &net);
    spawn_top_hud(&mut commands, &state, &net, &balance, &phase_line);

    if state.snapshot.is_none() {
        spawn_bottom_console(&mut commands);
        spawn_lobby_browser(&mut commands, &state, &net);
        return;
    }

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

    if help.open {
        spawn_help_overlay(&mut commands);
    }
    if settings_overlay.open {
        spawn_settings_overlay(&mut commands, &settings);
    }
    spawn_match_hint(&mut commands, &state, &net, &mut hints);
}

/// Settings panel: volume, mute, fullscreen, resolution - all persisted to
/// config/client_settings.json (plan.md Phase 1 item 10).
pub(crate) fn spawn_settings_overlay(commands: &mut Commands, settings: &ClientSettings) {
    spawn_ui_rect(
        commands,
        Vec2::ZERO,
        Vec2::new(620.0, 360.0),
        Color::srgba(0.04, 0.036, 0.03, 0.97),
        60.0,
    );
    let (width, height) = settings.resolution();
    let percent = (settings.master_volume * 100.0).round() as i32;
    let bar = format!(
        "[{}{}]{}{:>3}%",
        "|".repeat((settings.master_volume * 10.0).round() as usize),
        " ".repeat(10 - (settings.master_volume * 10.0).round() as usize),
        if settings.muted { " MUTED" } else { "      " },
        percent
    );
    spawn_ui_label(
        commands,
        &format!(
            "SETTINGS                                        (O to close)\n\nMaster volume  {bar}\n       , quieter    . louder    M mute (global: V)\n\nFullscreen     {:<6}   F toggle\nResolution     {}x{}   N cycle preset\n\nSettings persist to config/client_settings.json",
            if settings.fullscreen { "On" } else { "Off" },
            width as i32,
            height as i32
        ),
        Vec2::new(0.0, 0.0),
        14.0,
        TEXT_PARCHMENT,
        61.0,
        Anchor::CENTER,
        Justify::Center,
    );
}

/// First-match guidance: one short line at a time, advancing as the player
/// demonstrably completes each step (plan.md P0-6).
pub(crate) fn spawn_match_hint(
    commands: &mut Commands,
    state: &SnapshotState,
    net: &ClientNet,
    hints: &mut MatchHints,
) {
    let hint = match (&state.snapshot, net.team) {
        (None, _) => Some("Press Enter to connect, then create or join a game."),
        (Some(snapshot), _) if snapshot.phase == MatchPhase::Lobby => {
            Some("Pick a race, then press the Ready button.")
        }
        (Some(snapshot), Some(team)) if snapshot.phase == MatchPhase::Playing => {
            let own = snapshot
                .buildings
                .iter()
                .filter(|building| side_of_player(snapshot, building.owner) == team)
                .count();
            let enemy_castle_hurt = snapshot
                .castles
                .iter()
                .any(|castle| castle.health < castle.max_health * 7 / 10);
            // Only advance once the step's condition holds.
            if hints.step == 1 && own >= 1 {
                hints.step = 2;
            }
            if hints.step == 2 && own >= 3 {
                hints.step = 3;
            }
            if hints.step == 3 && enemy_castle_hurt {
                hints.step = 4;
            }
            match hints.step {
                0 => {
                    hints.step = 1;
                    Some("Press 1-8 to pick a building, then click a glowing cell.")
                }
                1 => Some("Press 1-8 to pick a building, then click a glowing cell."),
                2 => Some("Top lane buildings feed the Top lane - build in both lanes."),
                3 => Some("Kills pay bounty gold. An economy building pays every tick."),
                4 => Some("Castle regen pauses while under attack - push your advantage."),
                _ => None,
            }
        }
        _ => None,
    };
    let Some(hint) = hint else {
        return;
    };
    spawn_ui_rect(
        commands,
        Vec2::new(0.0, TOP_BAR_Y + 26.0),
        Vec2::new(560.0, 22.0),
        Color::srgba(0.05, 0.045, 0.03, 0.88),
        48.0,
    );
    spawn_ui_label(
        commands,
        &format!("HINT: {hint}"),
        Vec2::new(0.0, TOP_BAR_Y + 26.0),
        12.0,
        TEXT_GOLD,
        49.0,
        Anchor::CENTER,
        Justify::Center,
    );
}

pub(crate) fn spawn_help_overlay(commands: &mut Commands) {
    spawn_ui_rect(
        commands,
        Vec2::ZERO,
        Vec2::new(720.0, 560.0),
        Color::srgba(0.04, 0.036, 0.03, 0.97),
        60.0,
    );
    spawn_ui_label(
        commands,
        &format!(
            "CASTLE LANES - HOW TO PLAY            (press H to close)\n\nGOAL\nDestroy the enemy castle before they destroy yours.\n\nECONOMY\nEvery 10s you gain income plus 4% interest on banked gold.\nEconomy buildings add income. Kills pay bounty gold.\n\nBUILDING\n1-8 or the command card selects a building; left-click a\nglowing cell to place it. Top lane buildings feed the Top lane.\nFront zones build closer to the fight; Back zones are safer.\n\nCOMBAT IS AUTOMATIC - your job is to counter-build.\nPierce 130% vs Light, 70% vs Heavy.\nMagic 130% vs Heavy, 70% vs Light.\nSiege 150% vs Fortified (castles and buildings).\nNormal is neutral, 70% vs Fortified.\n\nTIPS\nCastle regen pauses while the castle is under attack.\nSudden death at 8:00 ramps up pressure until a castle falls.\n\nCONTROLS\nEnter connect/join   1-8 build   Left-click place/select\nEsc cancel/leave   Arrows/WASD pan   +/- zoom   Home reset\nDelete sell building   Delete sell   R rematch   Ctrl+Q concede   H help   O settings   V mute"
        ),
        Vec2::new(0.0, 0.0),
        13.0,
        TEXT_PARCHMENT,
        61.0,
        Anchor::CENTER,
        Justify::Center,
    );
}

pub(crate) fn spawn_top_hud(
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
            let index = player_index_of(snapshot, player.id);
            let econ = &snapshot.economies[index];
            let castle = &snapshot.castles[index];
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

pub(crate) fn spawn_resource_chip(
    commands: &mut Commands,
    pos: Vec2,
    label: &str,
    value: &str,
    color: Color,
) {
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

pub(crate) fn spawn_lobby_browser(commands: &mut Commands, state: &SnapshotState, net: &ClientNet) {
    spawn_ui_panel(
        commands,
        Vec2::new(0.0, 68.0),
        Vec2::new(540.0, 360.0),
        48.0,
    );
    spawn_ui_label(
        commands,
        "Game Lobby",
        Vec2::new(0.0, 210.0),
        30.0,
        TEXT_GOLD,
        51.0,
        Anchor::CENTER,
        Justify::Center,
    );
    let status = if net.connected {
        format!("Connected as {}  |  {}", net.player_name, net.status)
    } else {
        format!("{}  |  {}", net.server_addr, net.status)
    };
    spawn_ui_label(
        commands,
        &truncate_text(&status, 72),
        Vec2::new(0.0, 174.0),
        13.0,
        TEXT_PARCHMENT,
        51.0,
        Anchor::CENTER,
        Justify::Center,
    );

    spawn_ui_button_layer(
        commands,
        lobby_create_button_center(),
        lobby_button_size(),
        Color::srgba(0.18, 0.12, 0.07, 0.96),
        false,
        51.0,
    );
    spawn_ui_label(
        commands,
        "CREATE GAME",
        lobby_create_button_center() + Vec2::new(0.0, -2.0),
        15.0,
        TEXT_GOLD,
        54.0,
        Anchor::CENTER,
        Justify::Center,
    );
    spawn_ui_button_layer(
        commands,
        lobby_refresh_button_center(),
        lobby_button_size(),
        Color::srgba(0.11, 0.13, 0.10, 0.96),
        false,
        51.0,
    );
    spawn_ui_label(
        commands,
        "REFRESH",
        lobby_refresh_button_center() + Vec2::new(0.0, -2.0),
        15.0,
        TEXT_PARCHMENT,
        54.0,
        Anchor::CENTER,
        Justify::Center,
    );

    spawn_ui_label(
        commands,
        "Available Games",
        Vec2::new(-214.0, 100.0),
        15.0,
        TEXT_PARCHMENT,
        51.0,
        Anchor::CENTER_LEFT,
        Justify::Left,
    );

    if state.games.is_empty() {
        spawn_ui_label(
            commands,
            "No open games. Create one to host.",
            Vec2::new(0.0, 40.0),
            15.0,
            Color::srgba(0.78, 0.72, 0.58, 0.9),
            51.0,
            Anchor::CENTER,
            Justify::Center,
        );
    }

    for (idx, game) in state.games.iter().take(5).enumerate() {
        let center = lobby_game_row_center(idx);
        spawn_ui_button_layer(
            commands,
            center,
            lobby_game_row_size(),
            Color::srgba(0.065, 0.060, 0.050, 0.96),
            false,
            51.0,
        );
        let label = format!(
            "{}. {}    {}/{}    {:?}",
            idx + 1,
            truncate_text(&game.name, 24),
            game.players,
            game.max_players,
            game.phase
        );
        spawn_ui_label(
            commands,
            &label,
            center + Vec2::new(-214.0, -2.0),
            14.0,
            TEXT_PARCHMENT,
            54.0,
            Anchor::CENTER_LEFT,
            Justify::Left,
        );
    }

    let hint = if net.connected {
        "Enter/C: create  |  1-5/click: join  |  L: refresh"
    } else {
        "Enter: connect to server"
    };
    spawn_ui_label(
        commands,
        hint,
        Vec2::new(0.0, -80.0),
        13.0,
        Color::srgba(0.76, 0.66, 0.48, 0.95),
        51.0,
        Anchor::CENTER,
        Justify::Center,
    );
}

pub(crate) fn lobby_create_button_center() -> Vec2 {
    Vec2::new(-82.0, 136.0)
}

pub(crate) fn lobby_refresh_button_center() -> Vec2 {
    Vec2::new(112.0, 136.0)
}

pub(crate) fn lobby_button_size() -> Vec2 {
    Vec2::new(164.0, 38.0)
}

pub(crate) fn lobby_game_row_center(idx: usize) -> Vec2 {
    Vec2::new(0.0, 66.0 - idx as f32 * 38.0)
}

pub(crate) fn lobby_game_row_size() -> Vec2 {
    Vec2::new(460.0, 32.0)
}

pub(crate) fn spawn_bottom_console(commands: &mut Commands) {
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

pub(crate) fn spawn_context_bay(
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

pub(crate) fn spawn_command_card(
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

pub(crate) fn format_match_time(secs: f32) -> String {
    let secs = secs.max(0.0).floor() as u32;
    format!("{}:{:02}", secs / 60, secs % 60)
}

pub(crate) fn command_button_center(idx: usize) -> Vec2 {
    let col = idx % 3;
    let row = idx / 3;
    Vec2::new(
        COMMAND_GRID_ORIGIN.x + col as f32 * 58.0,
        COMMAND_GRID_ORIGIN.y - row as f32 * 50.0,
    )
}

pub(crate) fn race_button_center(idx: usize) -> Vec2 {
    Vec2::new(-184.0 + idx as f32 * 184.0, 42.0)
}

pub(crate) fn race_button_size() -> Vec2 {
    Vec2::new(156.0, 132.0)
}

pub(crate) fn ready_button_center() -> Vec2 {
    Vec2::new(0.0, -96.0)
}

pub(crate) fn ready_button_size() -> Vec2 {
    Vec2::new(178.0, 44.0)
}

pub(crate) fn selected_object_details(
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
                "{} {:?} {}\nHP {}/{}   {} {} dmg\nMv {:.1}   AS {:.2}/s   AtkR {:.1}\nArmor {}   Bounty {}g{}",
                config.name,
                side_of_player(snapshot, unit.owner),
                config.attack_mode.label(),
                unit.health.max(0),
                config.max_health,
                config.attack_type.label(),
                damage_range_text(config.damage, config.damage_variance),
                config.speed,
                attacks_per_second(config.attack_interval),
                config.attack_range,
                config.armor_type.label(),
                config.bounty,
                ability_line(config),
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
            let sell_hint = if side_matches_viewer(snapshot, building.owner, viewer_team)
                && viewer_team.is_some()
            {
                format!(
                    "\nDelete: sell for {}g",
                    (castle_lanes::sim::SELL_REFUND_RATIO * config.cost as f32).floor() as i32
                )
            } else {
                String::new()
            };
            if let Some(unit_kind) = config.spawned_unit {
                let unit = balance.unit(unit_kind);
                Some(format!(
                    "{} {:?} {:?} {:?}\nHP {}/{}   Next {} in {:.1}s\nProduces: {}  HP {}\nDamage {} {}   Armor {}\nMv {:.1}   AtkR {:.1}   AS {:.2}/s{}",
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
                    sell_hint,
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

pub(crate) fn selected_build_details(balance: &BalanceConfig, kind: BuildingKind) -> String {
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

pub(crate) fn command_button_badge(balance: &BalanceConfig, kind: BuildingKind) -> String {
    let config = balance.building(kind);
    if let Some(unit_kind) = config.spawned_unit {
        let unit = balance.unit(unit_kind);
        format!("{}  {}g", unit.attack_mode.short_label(), config.cost)
    } else {
        format!("+{}  {}g", config.income_bonus, config.cost)
    }
}

pub(crate) fn spawn_build_tooltip(
    commands: &mut Commands,
    balance: &BalanceConfig,
    kind: BuildingKind,
) {
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

pub(crate) fn building_tooltip_text(balance: &BalanceConfig, kind: BuildingKind) -> String {
    let config = balance.building(kind);
    if let Some(unit_kind) = config.spawned_unit {
        let unit = balance.unit(unit_kind);
        format!(
            "{}\nCost: {} gold   Spawns every {:.1}s\nProduces: {} ({})   Bounty: {}g\nDamage: {} {}   Armor: {}\n{}\n{}\nMove: {:.1}   AtkR: {:.1}   AS: {:.2}/s",
            config.name,
            config.cost,
            config.spawn_interval.unwrap_or_default(),
            unit.name,
            unit.attack_mode.label(),
            unit.bounty,
            damage_range_text(unit.damage, unit.damage_variance),
            unit.attack_type.label(),
            unit.armor_type.label(),
            counter_summary(unit.attack_type),
            ability_line(unit),
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

pub(crate) fn spawn_race_selection_popup(
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

pub(crate) fn race_card_summary(balance: &BalanceConfig, race: RaceKind) -> String {
    let config = balance.race(race);
    let blurb = match race {
        RaceKind::Vanguard => "Steady frontline + ranged. Flexible and forgiving.",
        RaceKind::Grove => "Cheap swarms, tanky guards, healing. Wins long games.",
        RaceKind::Ember => "Fast cheap attackers and burst casters. Press early.",
    };
    format!(
        "{}\nCastle HP: {}   Armor: {}   Buildings: {}",
        blurb,
        config.castle_health,
        config.castle_armor.label(),
        config.buildings.len()
    )
}

pub(crate) fn race_card_color(race: RaceKind) -> Color {
    match race {
        RaceKind::Vanguard => Color::srgba(0.08, 0.10, 0.13, 0.98),
        RaceKind::Grove => Color::srgba(0.07, 0.13, 0.08, 0.98),
        RaceKind::Ember => Color::srgba(0.15, 0.07, 0.045, 0.98),
    }
}

pub(crate) fn race_accent_color(race: RaceKind) -> Color {
    match race {
        RaceKind::Vanguard => Color::srgb(0.36, 0.54, 0.78),
        RaceKind::Grove => Color::srgb(0.36, 0.68, 0.32),
        RaceKind::Ember => Color::srgb(0.88, 0.34, 0.17),
    }
}

pub(crate) fn spawn_minimap(
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
                    side_of_player(snapshot, building.owner),
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
        let side = side_of_player(snapshot, building.owner);
        let pos = minimap_world_to_ui(cell_to_world(
            side,
            building.lane,
            building.zone,
            building.cell,
        ));
        spawn_ui_rect(
            commands,
            pos,
            Vec2::splat(4.0),
            team_minimap_color(side, viewer_team),
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
            team_minimap_color(side_of_player(snapshot, unit.owner), viewer_team),
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

pub(crate) fn spawn_minimap_fog(
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

pub(crate) fn minimap_world_to_ui(pos: Vec2) -> Vec2 {
    Vec2::new(
        MINIMAP_CENTER.x + (pos.x / MAP_W).clamp(-0.5, 0.5) * MINIMAP_SIZE.x,
        MINIMAP_CENTER.y + (pos.y / MAP_H).clamp(-0.5, 0.5) * MINIMAP_SIZE.y,
    )
}

pub(crate) fn team_color(team: Team) -> Color {
    match team {
        Team::Left => TEAM_COLOR_LEFT,
        Team::Right => TEAM_COLOR_RIGHT,
    }
}

pub(crate) fn team_minimap_color(team: Team, _viewer_team: Option<Team>) -> Color {
    // One color per side everywhere (units, bars, minimap, bounty text) so
    // team identity reads at a glance even for spectators.
    team_color(team)
}

/// Readable counter line computed from the actual damage matrix, e.g.
/// "Strong vs Heavy(130%) / weak vs Light(70%)" - plan.md P0-6.
pub(crate) fn counter_summary(attack: AttackType) -> String {
    let armors = [
        (ArmorType::Normal, "Normal"),
        (ArmorType::Light, "Light"),
        (ArmorType::Heavy, "Heavy"),
        (ArmorType::Fortified, "Fort"),
        (ArmorType::Unarmored, "Unarm"),
    ];
    let mut strong = Vec::new();
    let mut weak = Vec::new();
    for (armor, label) in armors {
        let multiplier = castle_lanes::sim::attack_multiplier(attack, armor);
        if multiplier >= 1.2 {
            strong.push(format!("{label} {}%", (multiplier * 100.0) as i32));
        } else if multiplier <= 0.85 {
            weak.push(format!("{label} {}%", (multiplier * 100.0) as i32));
        }
    }
    format!(
        "Counters: strong vs {} / weak vs {}",
        if strong.is_empty() {
            "-".to_string()
        } else {
            strong.join(", ")
        },
        if weak.is_empty() {
            "-".to_string()
        } else {
            weak.join(", ")
        }
    )
}

/// "Ability: ..." tooltip line when the unit has one (plan.md Phase 2 item 2).
fn ability_line(config: &castle_lanes::sim::UnitConfig) -> String {
    match &config.ability {
        Some(ability) => format!("Ability: {}", ability.describe()),
        None => String::new(),
    }
}

fn attacks_per_second(attack_interval: f32) -> f32 {
    if attack_interval <= f32::EPSILON {
        return 0.0;
    }
    1.0 / attack_interval
}

pub(crate) fn damage_range_text(midpoint: i32, variance: f32) -> String {
    let (min_damage, max_damage) = castle_lanes::sim::damage_range(midpoint, variance);
    if min_damage == max_damage {
        min_damage.to_string()
    } else {
        format!("{min_damage}-{max_damage}")
    }
}

pub(crate) fn ui_summary(state: &SnapshotState, net: &ClientNet) -> (String, String, String) {
    let Some(snapshot) = &state.snapshot else {
        let prompt = if net.connected {
            "Create or join a game".to_string()
        } else {
            "Press Enter to connect".to_string()
        };
        return (
            "Server Lobby".to_string(),
            prompt,
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
            let index = player_index_of(snapshot, player.id);
            let econ = &snapshot.economies[index];
            let castle = &snapshot.castles[index];
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

pub(crate) fn spawn_value_bar(
    commands: &mut Commands,
    pos: Vec2,
    size: Vec2,
    pct: f32,
    color: Color,
    z: f32,
) {
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

pub(crate) fn truncate_text(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let mut out: String = value.chars().take(max_chars.saturating_sub(3)).collect();
    out.push_str("...");
    out
}

pub(crate) fn spawn_ui_panel(commands: &mut Commands, pos: Vec2, size: Vec2, z: f32) {
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

pub(crate) fn spawn_ui_button(
    commands: &mut Commands,
    pos: Vec2,
    size: Vec2,
    color: Color,
    selected: bool,
) {
    spawn_ui_button_layer(commands, pos, size, color, selected, 44.0);
}

pub(crate) fn spawn_ui_button_layer(
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

pub(crate) fn spawn_ui_rect(commands: &mut Commands, pos: Vec2, size: Vec2, color: Color, z: f32) {
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

pub(crate) fn spawn_ui_image(
    commands: &mut Commands,
    image: Handle<Image>,
    pos: Vec2,
    size: Vec2,
    z: f32,
) {
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

pub(crate) fn spawn_ui_label(
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

pub(crate) fn pin_ui_to_camera(
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
