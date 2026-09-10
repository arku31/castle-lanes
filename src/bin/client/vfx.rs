//! vfx systems split out of the monolithic client (plan.md Phase 1 item 8).
#![allow(unused_imports)]
pub(crate) use super::audio::*;
pub(crate) use super::input::*;
pub(crate) use super::net::*;
pub(crate) use super::render3d::*;
pub(crate) use super::scene::*;
pub(crate) use super::ui::*;
use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) fn detect_combat_vfx(
    mut commands: Commands,
    net: Res<ClientNet>,
    state: Res<SnapshotState>,
    mut tracker: ResMut<CombatTracker>,
    mut sfx: ResMut<SfxQueue>,
    mut budget: ResMut<VfxBudget>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut world_assets: ResMut<World3dAssets>,
) {
    let mut res = Res3d {
        meshes: &mut meshes,
        materials: &mut materials,
        assets: &mut world_assets,
    };
    budget.remaining = VFX_BUDGET_PER_SNAPSHOT;
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
        tracker.castle_health = Vec::new();
        return;
    }
    let balance = active_balance(&state);

    if tracker.initialized {
        tracker
            .seen_bounty_events
            .retain(|id| snapshot.bounty_events.iter().any(|event| event.id == *id));

        for (tracked_id, tracked) in tracker.units.iter() {
            // A tracked unit missing from the snapshot died; only make it
            // audible if it died somewhere we can currently see.
            let gone = !snapshot.units.iter().any(|unit| unit.id == *tracked_id);
            if gone && is_world_revealed(snapshot, net.team, tracked.pos) {
                sfx.push(Sfx::UnitDeath);
            }
        }
        for unit in &snapshot.units {
            if !is_unit_visible(snapshot, net.team, unit) {
                continue;
            }
            if let Some(previous) = tracker.units.get(&unit.id) {
                if unit.health < previous.health {
                    let damage = previous.health - unit.health;
                    let pos = unit_world_pos(unit);
                    let config = balance.unit(unit.kind);
                    let unit_side = side_of_player(snapshot, unit.owner);
                    let attacker =
                        infer_attacker(snapshot, &balance, pos, unit_side).map(|attacker| {
                            (
                                unit_world_pos(attacker),
                                balance.unit(attacker.kind).attack_type,
                            )
                        });
                    let attacker = attacker.unwrap_or((
                        Vec2::new(pos.x - unit_side.direction() * 38.0, pos.y),
                        config.attack_type,
                    ));
                    spawn_combat_impact_budgeted(
                        &mut commands,
                        &mut res,
                        &mut budget,
                        pos,
                        damage,
                        attacker.0,
                        attacker.1,
                        false,
                    );
                    if unit.health > 0 {
                        sfx.push(match config.attack_mode {
                            castle_lanes::sim::AttackMode::Melee => Sfx::MeleeHit,
                            castle_lanes::sim::AttackMode::Ranged => Sfx::RangedShot,
                        });
                    }
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
                    let pos = building_hit_pos(building, side_of_player(snapshot, building.owner));
                    let building_side = side_of_player(snapshot, building.owner);
                    let attacker =
                        infer_attacker(snapshot, &balance, pos, building_side).map(|attacker| {
                            (
                                unit_world_pos(attacker),
                                balance.unit(attacker.kind).attack_type,
                            )
                        });
                    let attacker = attacker.unwrap_or((
                        Vec2::new(pos.x - building_side.opponent().direction() * 58.0, pos.y),
                        AttackType::Siege,
                    ));
                    spawn_structure_impact(
                        &mut commands,
                        &mut res,
                        pos,
                        damage,
                        attacker.0,
                        attacker.1,
                    );
                }
            }
        }

        for castle in &snapshot.castles {
            if !is_castle_visible(snapshot, net.team, castle.team) {
                continue;
            }
            let Some(index) = snapshot
                .castles
                .iter()
                .position(|tracked| tracked.owner == castle.owner)
            else {
                continue;
            };
            let previous = tracker
                .castle_health
                .get(index)
                .copied()
                .unwrap_or(castle.health);
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
                spawn_combat_impact_budgeted(
                    &mut commands,
                    &mut res,
                    &mut budget,
                    pos,
                    damage,
                    attacker.0,
                    attacker.1,
                    true,
                );
                sfx.push(Sfx::CastleAlarm);
            }
        }

        if let Some(team) = net.team {
            for event in &snapshot.bounty_events {
                if event.team == team && !tracker.seen_bounty_events.contains(&event.id) {
                    let pos = sim_pos_to_world(event.pos) + Vec2::new(0.0, 34.0);
                    spawn_bounty_text(&mut commands, &mut res, pos, event.amount, event.team);
                    sfx.push(Sfx::BountyCoin);
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
                    pos: unit_world_pos(unit),
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
    tracker.castle_health = snapshot
        .castles
        .iter()
        .map(|castle| castle.health)
        .collect::<Vec<_>>();
    tracker.initialized = true;
}

#[allow(clippy::type_complexity)]
pub(crate) fn update_combat_vfx(
    mut commands: Commands,
    time: Res<Time>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut query: Query<
        (
            Entity,
            &mut CombatVfx,
            &mut Transform,
            Option<&mut TextFont>,
            Option<&mut TextColor>,
            Option<&mut Sprite>,
        ),
        (Without<VfxBillboard>, Without<CombatText3d>),
    >,
    vfx3d: Query<
        (
            Entity,
            &mut CombatVfx,
            &mut Transform,
            &MeshMaterial3d<StandardMaterial>,
        ),
        With<VfxBillboard>,
    >,
    windows: Query<&Window, With<PrimaryWindow>>,
    cam3d: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut texts: Query<
        (
            Entity,
            &mut CombatText3d,
            &mut Transform,
            &mut TextColor,
            &CombatVfx,
        ),
        Without<VfxBillboard>,
    >,
) {
    // texts is passed through by reference below

    if is_3d() {
        update_combat_text_3d(&mut commands, &time, &windows, &cam3d, &mut texts);
        update_combat_vfx_3d(commands, time, materials, vfx3d);
        return;
    }
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

pub(crate) fn infer_attacker<'a>(
    snapshot: &'a MatchSnapshot,
    balance: &BalanceConfig,
    target_pos: Vec2,
    target_team: Team,
) -> Option<&'a Unit> {
    snapshot
        .units
        .iter()
        .filter(|unit| side_of_player(snapshot, unit.owner) != target_team)
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

/// Animation-pass pilot (plan.md Phase 2 item 5, Vanguard only): ranged
/// Vanguard attacks fire a visible bolt that flies from attacker to target.
/// Purely cosmetic - the server's damage is already applied in the snapshot.
fn spawn_projectile(commands: &mut Commands, from: Vec2, to: Vec2, attack_type: AttackType) {
    let delta = to - from;
    let distance = delta.length();
    if distance < 8.0 {
        return;
    }
    let speed = 700.0_f32;
    let lifetime = (distance / speed).clamp(0.03, 0.5);
    let velocity = delta / distance * speed;
    let angle = delta.y.atan2(delta.x);
    commands.spawn((
        Sprite::from_color(
            damage_number_color(attack_type, false),
            Vec2::new(13.0, 2.0),
        ),
        Transform::from_xyz(from.x, from.y + 14.0, VFX_Z - 2.0)
            .with_rotation(Quat::from_rotation_z(angle)),
        CombatVfx {
            lifetime,
            max_lifetime: lifetime,
            velocity,
        },
    ));
}

pub(crate) fn building_hit_pos(building: &Building, side: Team) -> Vec2 {
    cell_to_world(side, building.lane, building.zone, building.cell) + Vec2::new(0.0, 14.0)
}

pub(crate) fn spawn_structure_impact(
    commands: &mut Commands,
    res: &mut Res3d,
    target: Vec2,
    damage: i32,
    source: Vec2,
    attack_type: AttackType,
) {
    if is_3d() {
        spawn_structure_impact_3d(res, commands, target);
        spawn_combat_impact(commands, res, target, damage, source, attack_type, false);
        return;
    }
    commands.spawn((
        Sprite::from_color(Color::srgba(1.0, 0.24, 0.08, 0.34), Vec2::new(58.0, 46.0)),
        Transform::from_xyz(target.x, target.y - 8.0, VFX_Z + 1.0)
            .with_rotation(Quat::from_rotation_z(0.10)),
        CombatVfx {
            lifetime: 0.26,
            max_lifetime: 0.26,
            velocity: Vec2::ZERO,
        },
    ));
    commands.spawn((
        Sprite::from_color(Color::srgba(1.0, 0.78, 0.24, 0.28), Vec2::new(44.0, 34.0)),
        Transform::from_xyz(target.x, target.y - 8.0, VFX_Z + 1.5)
            .with_rotation(Quat::from_rotation_z(-0.08)),
        CombatVfx {
            lifetime: 0.18,
            max_lifetime: 0.18,
            velocity: Vec2::ZERO,
        },
    ));
    spawn_combat_impact(commands, res, target, damage, source, attack_type, false);
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

pub(crate) fn spawn_combat_impact(
    commands: &mut Commands,
    res: &mut Res3d,
    target: Vec2,
    damage: i32,
    source: Vec2,
    attack_type: AttackType,
    castle_hit: bool,
) {
    if is_3d() {
        spawn_impact_3d(res, commands, target, attack_type, castle_hit);
        spawn_streak_3d(res, commands, source, target, attack_type);
        let big_hit = damage >= if castle_hit { 12 } else { 8 };
        let text = if big_hit {
            format!("{damage}!")
        } else {
            damage.to_string()
        };
        let base = world2_to_3d(target) + Vec3::Y * 34.0;
        spawn_damage_text_3d(
            commands,
            base + Vec3::Y * 2.0,
            &text,
            Color::srgb(0.05, 0.02, 0.01),
            if castle_hit { 34.0 } else { 27.0 },
        );
        spawn_damage_text_3d(
            commands,
            base,
            &text,
            damage_number_color(attack_type, castle_hit),
            if castle_hit { 32.0 } else { 25.0 },
        );
        return;
    }
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

/// Budgeted variant: skips streak/burst when the per-snapshot budget is spent.
fn spawn_combat_impact_budgeted(
    commands: &mut Commands,
    res: &mut Res3d,
    budget: &mut VfxBudget,
    target: Vec2,
    damage: i32,
    source: Vec2,
    attack_type: AttackType,
    castle_hit: bool,
) {
    if budget.take() {
        spawn_combat_impact(
            commands,
            res,
            target,
            damage,
            source,
            attack_type,
            castle_hit,
        );
    } else if is_3d() {
        spawn_impact_3d(res, commands, target, attack_type, castle_hit);
    } else {
        let color = damage_number_color(attack_type, castle_hit);
        spawn_damage_text(
            commands,
            &damage.to_string(),
            target + Vec2::new(0.0, 29.0),
            color,
            if castle_hit { 32.0 } else { 25.0 },
            VFX_Z + 5.0,
        );
    }
}

/// World-anchored floating text for the 3D renderer; the updater projects
/// `world` to screen space every frame (M5 overlay pass, plan-0.2.md §8).
#[derive(Component)]
pub(crate) struct CombatText3d {
    pub world: Vec3,
    pub rise: f32,
}

pub(crate) fn spawn_damage_text_3d(
    commands: &mut Commands,
    world: Vec3,
    text: &str,
    color: Color,
    size: f32,
) {
    commands.spawn((
        Text2d::new(text),
        TextFont::from_font_size(size),
        TextColor(color),
        TextLayout::new_with_justify(Justify::Center),
        Anchor::CENTER,
        Transform::from_xyz(-9999.0, -9999.0, 0.0),
        CombatText3d { world, rise: 0.0 },
        CombatVfx {
            lifetime: 0.95,
            max_lifetime: 0.95,
            velocity: Vec2::ZERO,
        },
    ));
}

pub(crate) fn update_combat_text_3d(
    commands: &mut Commands,
    time: &Time,
    windows: &Query<&Window, With<PrimaryWindow>>,
    cam: &Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    query: &mut Query<
        (
            Entity,
            &mut CombatText3d,
            &mut Transform,
            &mut TextColor,
            &CombatVfx,
        ),
        Without<VfxBillboard>,
    >,
) {
    let Ok((camera, cam_transform)) = cam.single() else {
        return;
    };
    let Ok(window) = windows.single() else {
        return;
    };
    let dt = time.delta_secs();
    for (entity, mut text, mut transform, mut color, vfx) in &mut *query {
        text.rise += 46.0 * dt;
        let pos = text.world + Vec3::Y * text.rise;
        let Ok(screen) = camera.world_to_viewport(cam_transform, pos) else {
            continue;
        };
        transform.translation = Vec3::new(
            screen.x - window.width() / 2.0,
            window.height() / 2.0 - screen.y,
            0.0,
        );
        let alpha = (vfx.lifetime / vfx.max_lifetime).clamp(0.0, 1.0);
        color.0 = color.0.with_alpha(alpha);
        if vfx.lifetime <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}

pub(crate) fn spawn_damage_text(
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

pub(crate) fn spawn_bounty_text(
    commands: &mut Commands,
    res: &mut Res3d,
    pos: Vec2,
    amount: i32,
    team: Team,
) {
    let accent = team_color(team);
    if is_3d() {
        let base = world2_to_3d(pos) + Vec3::Y * 30.0;
        spawn_damage_text_3d(
            commands,
            base + Vec3::Y * 3.0,
            &format!("+{amount}g"),
            Color::srgb(0.08, 0.04, 0.0),
            30.0,
        );
        spawn_damage_text_3d(commands, base, &format!("+{amount}g"), accent, 28.0);
        for offset in [
            Vec2::new(-16.0, -2.0),
            Vec2::new(19.0, 3.0),
            Vec2::new(4.0, 15.0),
        ] {
            spawn_sparkle_3d(
                res,
                commands,
                pos,
                accent,
                offset,
                Vec2::new(offset.x * 0.42, 48.0 + offset.y.max(0.0)),
                0.72,
            );
        }
        return;
    }
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
        accent,
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
            Sprite::from_color(accent, Vec2::splat(7.0)),
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

pub(crate) fn spawn_floating_text(
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

pub(crate) fn spawn_hit_burst(
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

pub(crate) fn spawn_attack_streak(
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

pub(crate) fn damage_number_color(attack_type: AttackType, castle_hit: bool) -> Color {
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
