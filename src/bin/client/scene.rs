//! scene systems split out of the monolithic client (plan.md Phase 1 item 8).
#![allow(unused_imports)]
pub(crate) use super::audio::*;
pub(crate) use super::input::*;
pub(crate) use super::net::*;
pub(crate) use super::ui::*;
pub(crate) use super::vfx::*;
use super::*;

pub(crate) fn animate_grass(time: Res<Time>, mut query: Query<(&GrassBlade, &mut Transform)>) {
    let t = time.elapsed_secs();
    for (blade, mut transform) in &mut query {
        let wave = (t * 1.35 + blade.phase).sin();
        transform.translation.x = blade.base_pos.x + wave * blade.sway;
        transform.translation.y = blade.base_pos.y;
        transform.rotation = Quat::from_rotation_z(blade.base_rotation + wave * 0.055);
    }
}

pub(crate) fn update_fog_memory(
    mut fog: ResMut<FogMemory>,
    state: Res<SnapshotState>,
    net: Res<ClientNet>,
) {
    if fog.team != net.team {
        fog.team = net.team;
        fog.explored.fill(false);
        fog.revision += 1;
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

    let mut changed = false;
    for row in 0..FOG_ROWS {
        for col in 0..FOG_COLUMNS {
            let pos = fog_cell_center(col, row);
            if world_reveal_strength(snapshot, team, pos) > 0.05
                && !fog.explored[fog_index(col, row)]
            {
                fog.explored[fog_index(col, row)] = true;
                changed = true;
            }
        }
    }
    if changed {
        fog.revision += 1;
    }
}

pub(crate) fn static_scene_key(state: &SnapshotState, net: &ClientNet, fog: &FogMemory) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    net.team.hash(&mut hasher);
    fog.revision.hash(&mut hasher);
    if let Some(snapshot) = &state.snapshot {
        snapshot.phase.hash(&mut hasher);
        for castle in &snapshot.castles {
            castle.team.hash(&mut hasher);
            castle.health.hash(&mut hasher);
        }
        for player in &snapshot.players {
            player.race.hash(&mut hasher);
        }
        let mut building_rows: Vec<(u64, i32, u8)> = snapshot
            .buildings
            .iter()
            .map(|building| {
                let visibility = if is_building_visible(snapshot, net.team, building) {
                    2u8
                } else if is_enemy_building_scouted(snapshot, fog, net.team, building) {
                    1u8
                } else {
                    0u8
                };
                (building.id, building.health, visibility)
            })
            .collect();
        building_rows.sort_unstable();
        for row in building_rows {
            row.hash(&mut hasher);
        }
    }
    hasher.finish()
}

pub(crate) fn sync_static_scene(
    mut commands: Commands,
    mut registry: ResMut<SceneRegistry>,
    statics: Query<Entity, With<StaticScene>>,
    fog_tiles: Query<Entity, With<FogTile>>,
    state: Res<SnapshotState>,
    net: Res<ClientNet>,
    fog: Res<FogMemory>,
    building_icons: Res<BuildingIconAssets>,
) {
    let key = static_scene_key(&state, &net, &fog);
    if key != registry.statics_key {
        registry.statics_key = key;
        for entity in &statics {
            commands.entity(entity).despawn();
        }
        let lane_count = state.snapshot.as_ref().map(|s| s.players.len()).unwrap_or(2);
        let assigned_lane = net
            .player_id
            .and_then(|pid| {
                state.snapshot.as_ref().and_then(|s| {
                    let side_index = s
                        .players
                        .iter()
                        .filter(|p| Some(p.team) == net.team)
                        .position(|p| p.id == pid);
                    side_index.map(|idx| Lane::for_player(net.team.unwrap_or(Team::Left), idx))
                })
            });
        spawn_static_board(&mut commands, net.team, lane_count, assigned_lane);
        if let Some(snapshot) = &state.snapshot {
            let balance = active_balance(&state);
            for building in &snapshot.buildings {
                let side = side_of_player(snapshot, building.owner);
                if !is_building_visible(snapshot, net.team, building) {
                    if is_enemy_building_scouted(snapshot, &fog, net.team, building) {
                        spawn_building_silhouette(&mut commands, building, side);
                    }
                    continue;
                }
                spawn_building(&mut commands, building, side, &balance, &building_icons);
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
                );
            }
        }
    }

    let want_fog = state.snapshot.is_some() && net.team.is_some();
    if want_fog && registry.fog_tiles.is_empty() {
        let mut tiles = Vec::with_capacity(FOG_COLUMNS * FOG_ROWS);
        for row in 0..FOG_ROWS {
            for col in 0..FOG_COLUMNS {
                let pos = fog_cell_center(col, row);
                let tile = commands
                    .spawn((
                        Sprite::from_color(Color::srgba(0.0, 0.0, 0.0, 0.0), Vec2::ZERO),
                        Transform::from_xyz(pos.x, pos.y, 18.0),
                        FogTile { col, row },
                    ))
                    .id();
                tiles.push(tile);
            }
        }
        registry.fog_tiles = tiles;
    } else if !want_fog && !registry.fog_tiles.is_empty() {
        for entity in &fog_tiles {
            commands.entity(entity).despawn();
        }
        registry.fog_tiles.clear();
    }
}

pub(crate) fn sync_units(
    mut commands: Commands,
    mut registry: ResMut<SceneRegistry>,
    mut sprites: Query<&mut Sprite>,
    mut transforms: Query<&mut Transform>,
    state: Res<SnapshotState>,
    net: Res<ClientNet>,
    unit_assets: Res<UnitSpriteAssets>,
    mut sfx: ResMut<SfxQueue>,
    frame_sets: Res<UnitFrameSets>,
) {
    let Some(snapshot) = &state.snapshot else {
        despawn_all_units(&mut commands, &mut registry);
        return;
    };
    let balance = active_balance(&state);
    let mut alive: HashSet<u64> = HashSet::new();
    for unit in &snapshot.units {
        if !is_unit_visible(snapshot, net.team, unit) {
            continue;
        }
        alive.insert(unit.id);
        if let Some(visual) = registry.units.get_mut(&unit.id) {
            if visual.health != unit.health {
                visual.health = unit.health;
                let pct = (unit.health.max(0) as f32
                    / balance.unit(unit.kind).max_health.max(1) as f32)
                    .clamp(0.0, 1.0);
                if let Ok(mut fill) = transforms.get_mut(visual.health_fill) {
                    fill.translation.x = -(UNIT_HEALTH_BAR_W * (1.0 - pct)) / 2.0;
                }
                if let Ok(mut fill_sprite) = sprites.get_mut(visual.health_fill) {
                    fill_sprite.custom_size = Some(Vec2::new(UNIT_HEALTH_BAR_W * pct, 4.0));
                }
            }
        } else {
            let side = side_of_player(snapshot, unit.owner);
            let visual = spawn_unit_visual(&mut commands, unit, side, &balance, &unit_assets);
            registry.units.insert(unit.id, visual);
            sfx.push(Sfx::UnitSpawn);
        }
    }
    let stale: Vec<u64> = registry
        .units
        .keys()
        .copied()
        .filter(|id| !alive.contains(id))
        .collect();
    for id in stale {
        if let Some(visual) = registry.units.remove(&id) {
            // Death frame: leave a fading corpse where the unit stood.
            if let Some(frames) = frame_sets.frames.get(&visual.kind) {
                let (mut x, mut y) = (visual.last_pos.x, visual.last_pos.y);
                if let Ok(root) = transforms.get(visual.root) {
                    x = root.translation.x;
                    y = root.translation.y;
                }
                let mut corpse_sprite = Sprite::from_image(frames[4].clone());
                corpse_sprite.custom_size = Some(unit_sprite_size(visual.kind));
                corpse_sprite.flip_x = visual.side == Team::Right;
                commands.spawn((
                    corpse_sprite,
                    Transform::from_xyz(x, y + 12.0, UNIT_ROOT_Z - 0.5),
                    CombatVfx {
                        lifetime: 0.9,
                        max_lifetime: 0.9,
                        velocity: Vec2::ZERO,
                    },
                ));
            }
            commands.entity(visual.root).despawn();
        }
    }
}

pub(crate) fn despawn_all_units(commands: &mut Commands, registry: &mut SceneRegistry) {
    for (_, visual) in registry.units.drain() {
        commands.entity(visual.root).despawn();
    }
}

/// Render units every frame: interpolate between the last two snapshots and
/// animate the idle bob on the display clock, not the snapshot clock.
pub(crate) fn animate_units(
    time: Res<Time>,
    interp: Res<RenderInterp>,
    state: Res<SnapshotState>,
    registry: Res<SceneRegistry>,
    mut transforms: Query<&mut Transform>,
    mut sprites: Query<&mut Sprite>,
    frame_sets: Res<UnitFrameSets>,
) {
    if registry.units.is_empty() {
        return;
    }
    let Some(snapshot) = &state.snapshot else {
        return;
    };
    let mut render_t = 1.0_f32;
    let mut overshoot = 0.0_f32;
    if let (Some(prev), Some(latest)) = (&interp.prev, &interp.latest) {
        let interval = latest.at.saturating_duration_since(prev.at);
        let interval_secs = interval.as_secs_f32().max(1.0 / 60.0);
        let delay = (interval_secs * 1.25).clamp(0.08, 0.25);
        let raw = (prev.at.elapsed().as_secs_f32() - delay) / interval_secs;
        render_t = raw.clamp(0.0, 1.0);
        overshoot = (raw - 1.0).max(0.0) * interval_secs;
    }

    for unit in &snapshot.units {
        let Some(visual) = registry.units.get(&unit.id) else {
            continue;
        };
        let Ok(mut root) = transforms.get_mut(visual.root) else {
            continue;
        };
        let sim_pos = match interp
            .prev
            .as_ref()
            .and_then(|prev| prev.unit_pos.get(&unit.id))
        {
            Some(prev_pos) => {
                let x = prev_pos.x + (unit.pos.x - prev_pos.x) * render_t;
                let y = prev_pos.y + (unit.pos.y - prev_pos.y) * render_t;
                castle_lanes::sim::WorldPos::new(x, y)
            }
            None => unit.pos,
        };
        let sim_pos = castle_lanes::sim::WorldPos::new(
            (sim_pos.x + unit.velocity.x * overshoot).clamp(-18.0, LANE_LENGTH + 18.0),
            sim_pos.y + unit.velocity.y * overshoot,
        );
        let world = sim_pos_to_world(sim_pos);
        root.translation = Vec3::new(world.x, world.y, UNIT_ROOT_Z);

        // Animation pass (plan.md Phase 2 item 5): swap sprite frames for
        // units with a generated atlas. Attack frames fire right after the
        // server resets the attack timer; walk frames cycle while moving.
        if let Some(frames) = frame_sets.frames.get(&unit.kind) {
            let config = state.balance.unit(unit.kind);
            let moving = unit.velocity.x.abs() > 0.1;
            let frame = if unit.attack_timer > config.attack_interval - 0.35 {
                if unit.attack_timer > config.attack_interval - 0.18 {
                    3
                } else {
                    2
                }
            } else if moving {
                ((time.elapsed_secs() * 6.0) as usize + unit.id as usize) % 2
            } else {
                0
            };
            if let Ok(mut sprite) = sprites.get_mut(visual.sprite) {
                sprite.image = frames[frame].clone();
            }
        }

        let phase = time.elapsed_secs() * 4.0 + (unit.id % 97) as f32 * 1.37;
        let bob = phase.sin() * 2.0;
        if let Ok(mut sprite) = transforms.get_mut(visual.sprite) {
            sprite.translation.y = visual.sprite_base_y + bob;
            sprite.scale.y = 1.0 + phase.cos() * 0.018;
        }
        if let Ok(mut badge) = transforms.get_mut(visual.badge_attack) {
            badge.translation.y = visual.badge_base_y + bob;
        }
        if let Ok(mut badge) = transforms.get_mut(visual.badge_armor) {
            badge.translation.y = visual.badge_base_y + bob;
        }
    }
}

pub(crate) fn update_object_highlight(
    mut commands: Commands,
    mut registry: ResMut<SceneRegistry>,
    state: Res<SnapshotState>,
    world_selection: Res<WorldSelection>,
    world_hover: Res<WorldHover>,
    mut transforms: Query<&mut Transform>,
) {
    let Some(snapshot) = &state.snapshot else {
        despawn_highlight(&mut commands, &mut registry);
        return;
    };
    let mut source = None;
    let mut spec_size = Vec2::ZERO;
    let mut spec_color = Color::srgba(0.95, 0.76, 0.24, 0.48);
    let mut spec_z = 7.6;
    let mut spec_diamond = true;
    let mut follow_unit = false;
    let mut static_pos = Vec2::ZERO;

    if let Some(SelectedObject::Unit(id)) = world_selection.selected {
        if let Some(unit) = snapshot.units.iter().find(|unit| unit.id == id) {
            let size = unit_sprite_size(unit.kind);
            spec_size = Vec2::splat(size.x * 0.80);
            source = Some(HighlightSource::Unit(id));
            follow_unit = true;
        }
    }
    if source.is_none() {
        match world_selection
            .selected
            .or(world_hover.hovered)
            .filter(|selected| {
                matches!(
                    selected,
                    SelectedObject::Building(_) | SelectedObject::Castle(_)
                )
            }) {
            Some(SelectedObject::Building(id)) => {
                if let Some(building) = snapshot.buildings.iter().find(|b| b.id == id) {
                    static_pos = cell_to_world(
                        side_of_player(snapshot, building.owner),
                        building.lane,
                        building.zone,
                        building.cell,
                    );
                    static_pos.y -= 2.0;
                    spec_size = Vec2::splat(CELL + 4.0);
                    spec_color = Color::srgba(0.95, 0.76, 0.24, 0.42);
                    spec_z = 2.5;
                    spec_diamond = false;
                    source = Some(HighlightSource::Building(id));
                }
            }
            Some(SelectedObject::Castle(team)) => {
                if let Some(castle) = snapshot.castles.iter().find(|c| c.team == team) {
                    static_pos = castle_world_pos(castle.team);
                    static_pos.y -= 22.0;
                    spec_size = Vec2::new(126.0, 28.0);
                    spec_color = Color::srgba(0.95, 0.75, 0.26, 0.34);
                    spec_z = 5.4;
                    spec_diamond = false;
                    source = Some(HighlightSource::Castle(team));
                }
            }
            _ => {}
        }
    }

    let Some(source) = source else {
        despawn_highlight(&mut commands, &mut registry);
        return;
    };

    if registry.highlight_source != Some(source) || registry.highlight.is_none() {
        despawn_highlight(&mut commands, &mut registry);
        let entity = if spec_diamond {
            spawn_diamond(&mut commands, Vec2::ZERO, spec_size, spec_color, spec_z)
        } else {
            spawn_rect(&mut commands, Vec2::ZERO, spec_size, spec_color, spec_z)
        };
        registry.highlight = Some(entity);
        registry.highlight_source = Some(source);
    }

    if let Some(entity) = registry.highlight {
        if follow_unit {
            if let Some(HighlightSource::Unit(id)) = registry.highlight_source {
                if let Some(visual) = registry.units.get(&id) {
                    if let Ok(root) = transforms.get(visual.root) {
                        static_pos = Vec2::new(root.translation.x, root.translation.y - 5.0);
                    }
                }
            }
        }
        if let Ok(mut transform) = transforms.get_mut(entity) {
            transform.translation.x = static_pos.x;
            transform.translation.y = static_pos.y;
            transform.translation.z = spec_z;
        }
    }
}

pub(crate) fn despawn_highlight(commands: &mut Commands, registry: &mut SceneRegistry) {
    if let Some(entity) = registry.highlight.take() {
        commands.entity(entity).despawn();
    }
    registry.highlight_source = None;
}

/// Fog tile colors mutate in place on snapshot/fog changes; tiles themselves
/// are spawned once per match in `sync_static_scene`.
pub(crate) fn update_fog_tiles(
    state: Res<SnapshotState>,
    fog: Res<FogMemory>,
    net: Res<ClientNet>,
    registry: Res<SceneRegistry>,
    mut tiles: Query<(&FogTile, &mut Sprite)>,
) {
    if registry.fog_tiles.is_empty() {
        return;
    }
    let (Some(snapshot), Some(team)) = (&state.snapshot, net.team) else {
        return;
    };
    if !state.is_changed() && !fog.is_changed() {
        return;
    }
    let tile_w = MAP_W / FOG_COLUMNS as f32;
    let tile_h = MAP_H / FOG_ROWS as f32;
    for (tile, mut sprite) in &mut tiles {
        let pos = fog_cell_center(tile.col, tile.row);
        let reveal = world_reveal_strength(snapshot, team, pos);
        let explored = fog.explored[fog_index(tile.col, tile.row)];
        let alpha = if reveal > 0.0 {
            0.20 * (1.0 - reveal)
        } else if explored {
            0.53
        } else {
            0.88
        };
        let tint = if explored {
            Color::srgba(0.006, 0.009, 0.012, alpha)
        } else {
            Color::srgba(0.002, 0.002, 0.003, alpha)
        };
        sprite.color = tint;
        sprite.custom_size = Some(Vec2::new(tile_w + 1.0, tile_h + 1.0));
    }
}

pub(crate) fn building_icon_handle(
    assets: &BuildingIconAssets,
    kind: BuildingKind,
) -> Handle<Image> {
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
        // Branch upgrades reuse their base race's icons for now; a dedicated
        // icon pass lands with the Phase 2 art batch.
        BuildingKind::VanguardArbalestTower | BuildingKind::VanguardArcaneSpire => {
            assets.vanguard_range_tower.clone()
        }
        BuildingKind::GroveBrambleWarren | BuildingKind::GroveSpitefen => {
            assets.grove_root_den.clone()
        }
        BuildingKind::EmberMagmaForge | BuildingKind::EmberAshPack => {
            assets.ember_cinder_pit.clone()
        }
    }
}

pub(crate) fn castle_icon_handle(assets: &BuildingIconAssets, race: RaceKind) -> Handle<Image> {
    match race {
        RaceKind::Vanguard => assets.vanguard_barracks.clone(),
        RaceKind::Grove => assets.grove_root_den.clone(),
        RaceKind::Ember => assets.ember_cinder_pit.clone(),
    }
}

pub(crate) fn unit_sprite_handle(assets: &UnitSpriteAssets, kind: UnitKind) -> Handle<Image> {
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
        // Upgrade units reuse existing sprites (documented in assets.md).
        UnitKind::VanguardArbalester => assets.vanguard_archer.clone(),
        UnitKind::VanguardArcanist => assets.vanguard_battle_cleric.clone(),
        UnitKind::GroveBrambleguard => assets.grove_barkguard.clone(),
        UnitKind::GroveSpitefang => assets.grove_vine_stalker.clone(),
        UnitKind::EmberMagmaBrute => assets.ember_obsidian_guard.clone(),
        UnitKind::EmberAshStalker => assets.ember_runner.clone(),
    }
}

pub(crate) fn unit_sprite_size(kind: UnitKind) -> Vec2 {
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
        // Upgrade units reuse base-sprite dimensions.
        UnitKind::VanguardArbalester => Vec2::splat(62.0),
        UnitKind::VanguardArcanist => Vec2::splat(64.0),
        UnitKind::GroveBrambleguard => Vec2::splat(70.0),
        UnitKind::GroveSpitefang => Vec2::splat(66.0),
        UnitKind::EmberMagmaBrute => Vec2::splat(70.0),
        UnitKind::EmberAshStalker => Vec2::splat(61.0),
    }
}

pub(crate) fn unit_world_pos(unit: &Unit) -> Vec2 {
    sim_pos_to_world(unit.pos)
}

pub(crate) fn castle_world_pos(team: Team) -> Vec2 {
    Vec2::new(lane_to_world(team.castle_pos()), LANE_Y + 8.0)
}

pub(crate) fn sim_pos_to_world(pos: castle_lanes::sim::WorldPos) -> Vec2 {
    Vec2::new(lane_to_world(pos.x), pos.y * SIM_Y_TO_WORLD)
}

pub(crate) fn is_unit_visible(
    snapshot: &MatchSnapshot,
    viewer_team: Option<Team>,
    unit: &Unit,
) -> bool {
    if side_matches_viewer(snapshot, unit.owner, viewer_team) {
        return true;
    }
    is_world_revealed(snapshot, viewer_team, unit_world_pos(unit))
}

pub(crate) fn is_building_visible(
    snapshot: &MatchSnapshot,
    viewer_team: Option<Team>,
    building: &Building,
) -> bool {
    if side_matches_viewer(snapshot, building.owner, viewer_team) {
        return true;
    }
    is_world_revealed(
        snapshot,
        viewer_team,
        cell_to_world(
            side_of_player(snapshot, building.owner),
            building.lane,
            building.zone,
            building.cell,
        ),
    )
}

pub(crate) fn is_castle_visible(
    snapshot: &MatchSnapshot,
    viewer_team: Option<Team>,
    team: Team,
) -> bool {
    if Some(team) == viewer_team || viewer_team.is_none() {
        return true;
    }
    is_world_revealed(snapshot, viewer_team, castle_world_pos(team))
}

pub(crate) fn is_world_revealed(
    snapshot: &MatchSnapshot,
    viewer_team: Option<Team>,
    pos: Vec2,
) -> bool {
    let Some(team) = viewer_team else {
        return true;
    };
    world_reveal_strength(snapshot, team, pos) > 0.08
}

pub(crate) fn world_reveal_strength(snapshot: &MatchSnapshot, team: Team, pos: Vec2) -> f32 {
    let castle_strength =
        reveal_strength(pos.distance(castle_world_pos(team)), REVEAL_CASTLE_RADIUS);
    let building_strength = snapshot
        .buildings
        .iter()
        .filter(|building| side_of_player(snapshot, building.owner) == team)
        .map(|building| {
            reveal_strength(
                pos.distance(cell_to_world(
                    side_of_player(snapshot, building.owner),
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
        .filter(|unit| side_of_player(snapshot, unit.owner) == team)
        .map(|unit| reveal_strength(pos.distance(unit_world_pos(unit)), REVEAL_UNIT_RADIUS))
        .fold(0.0, f32::max);
    castle_strength.max(building_strength).max(unit_strength)
}

pub(crate) fn reveal_strength(distance: f32, radius: f32) -> f32 {
    if distance >= radius {
        0.0
    } else if distance <= radius - FOG_SOFT_EDGE {
        1.0
    } else {
        ((radius - distance) / FOG_SOFT_EDGE).clamp(0.0, 1.0)
    }
}

pub(crate) fn is_enemy_building_scouted(
    snapshot: &MatchSnapshot,
    fog: &FogMemory,
    viewer_team: Option<Team>,
    building: &Building,
) -> bool {
    let Some(team) = viewer_team else {
        return false;
    };
    if side_of_player(snapshot, building.owner) == team {
        return false;
    }
    let pos = cell_to_world(
        side_of_player(snapshot, building.owner),
        building.lane,
        building.zone,
        building.cell,
    );
    !is_world_revealed(snapshot, viewer_team, pos) && fog_is_explored(fog, viewer_team, pos)
}

pub(crate) fn fog_is_explored(fog: &FogMemory, viewer_team: Option<Team>, pos: Vec2) -> bool {
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

pub(crate) fn fog_index(col: usize, row: usize) -> usize {
    row * FOG_COLUMNS + col
}

pub(crate) fn fog_cell(pos: Vec2) -> Option<(usize, usize)> {
    let x = ((pos.x + MAP_W * 0.5) / MAP_W * FOG_COLUMNS as f32).floor() as isize;
    let y = ((pos.y + MAP_H * 0.5) / MAP_H * FOG_ROWS as f32).floor() as isize;
    if x < 0 || y < 0 || x >= FOG_COLUMNS as isize || y >= FOG_ROWS as isize {
        return None;
    }
    Some((x as usize, y as usize))
}

pub(crate) fn fog_cell_center(col: usize, row: usize) -> Vec2 {
    let tile_w = MAP_W / FOG_COLUMNS as f32;
    let tile_h = MAP_H / FOG_ROWS as f32;
    Vec2::new(
        -MAP_W * 0.5 + tile_w * (col as f32 + 0.5),
        -MAP_H * 0.5 + tile_h * (row as f32 + 0.5),
    )
}

pub(crate) fn lane_world_y(lane: Lane) -> f32 {
    match lane {
        Lane::Top => 192.0,
        Lane::UpperMid => 64.0,
        Lane::LowerMid => -64.0,
        Lane::Bottom => -192.0,
    }
}

pub(crate) fn visible_sides(team: Option<Team>) -> Vec<Team> {
    match team {
        Some(team) => vec![team],
        None => vec![Team::Left, Team::Right],
    }
}

pub(crate) fn spawn_grass_background(commands: &mut Commands) {
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

pub(crate) fn hash_range(index: u32, salt: u32, min: f32, max: f32) -> f32 {
    min + (max - min) * hash_unit(index, salt)
}

pub(crate) fn hash_unit(index: u32, salt: u32) -> f32 {
    let mut value = index
        .wrapping_mul(747_796_405)
        .wrapping_add(salt.wrapping_mul(2_891_336_453));
    value ^= value >> 16;
    value = value.wrapping_mul(2_246_822_519);
    value ^= value >> 13;
    (value as f32) / (u32::MAX as f32)
}

pub(crate) fn spawn_static_board(
    commands: &mut Commands,
    team: Option<Team>,
    lane_count: usize,
    assigned_lane: Option<Lane>,
) {
    let mut spawned = Vec::new();
    let active_lanes = &Lane::ALL[..lane_count.min(Lane::ALL.len())];
    for lane in active_lanes {
        let y = lane_world_y(*lane);
        spawned.push(spawn_rect(
            commands,
            Vec2::new(0.0, y),
            Vec2::new(WORLD_W - 120.0, 30.0),
            Color::srgba(0.12, 0.10, 0.07, 0.34),
            -1.0,
        ));
        spawned.push(spawn_rect(
            commands,
            Vec2::new(0.0, y),
            Vec2::new(WORLD_W - 160.0, 3.0),
            Color::srgba(0.80, 0.64, 0.34, 0.42),
            0.0,
        ));
    }

    for side in visible_sides(team) {
        for lane in Lane::ALL {
            for zone in BuildZone::ALL {
                for x in 0..GRID_W {
                    for y in 0..GRID_H {
                        let pos = cell_to_world(side, lane, zone, GridCell { x, y });
                        // Team play: dim lanes that aren't this player's assignment
                        let wrong_lane = assigned_lane.is_some() && Some(side) == team && lane != assigned_lane.unwrap();
                        let color = if Some(side) != team {
                            Color::srgba(0.07, 0.06, 0.05, 0.20)
                        } else if wrong_lane {
                            Color::srgba(0.12, 0.10, 0.08, 0.10)
                        } else {
                            match zone {
                                BuildZone::Front => Color::srgba(0.26, 0.56, 0.72, 0.32),
                                BuildZone::Back => Color::srgba(0.30, 0.42, 0.68, 0.22),
                            }
                        };
                        spawned.push(spawn_rect(
                            commands,
                            pos,
                            Vec2::splat(CELL - 3.0),
                            color,
                            0.5,
                        ));
                        spawned.push(spawn_rect(
                            commands,
                            pos,
                            Vec2::new(CELL - 3.0, 2.0),
                            Color::srgba(0.67, 0.54, 0.31, 0.18),
                            0.7,
                        ));
                    }
                }
            }
        }
    }
    for entity in spawned {
        commands.entity(entity).insert(StaticScene);
    }
}

pub(crate) fn spawn_building(
    commands: &mut Commands,
    building: &Building,
    side: Team,
    balance: &BalanceConfig,
    building_icons: &BuildingIconAssets,
) {
    let pos = cell_to_world(side, building.lane, building.zone, building.cell);
    let config = balance.building(building.kind);
    let color = Color::srgb(config.color[0], config.color[1], config.color[2]);
    let mut spawned = Vec::new();
    spawned.push(spawn_rect(
        commands,
        Vec2::new(pos.x, pos.y - 8.0),
        Vec2::new(CELL - 8.0, CELL - 8.0),
        Color::srgba(0.04, 0.035, 0.025, 0.58),
        2.7,
    ));
    spawned.push(spawn_rect(
        commands,
        Vec2::new(pos.x, pos.y + 3.0),
        Vec2::new(CELL - 12.0, CELL - 12.0),
        color.with_alpha(0.50),
        3.0,
    ));
    spawned.push(spawn_rect(
        commands,
        Vec2::new(pos.x, pos.y - 8.0 - (CELL - 8.0) * 0.5 + 2.5),
        Vec2::new(CELL - 8.0, 3.5),
        team_color(side).with_alpha(0.9),
        2.9,
    ));
    let mut sprite = Sprite::from_image(building_icon_handle(building_icons, building.kind));
    sprite.custom_size = Some(Vec2::splat(48.0));
    spawned.push(
        commands
            .spawn((
                sprite,
                Transform::from_xyz(pos.x, pos.y + 4.0, 4.2),
                SceneEntity,
            ))
            .id(),
    );
    // Branch-upgraded buildings show a gold star marker (plan.md item 3).
    if config.upgraded_from.is_some() {
        let star = spawn_diamond(
            commands,
            Vec2::new(pos.x + CELL * 0.5 - 6.0, pos.y - CELL * 0.5 + 6.0),
            Vec2::splat(9.0),
            Color::srgb(1.0, 0.85, 0.25),
            3.6,
        );
        spawned.push(star);
    }
    for entity in spawned {
        commands.entity(entity).insert(StaticScene);
    }
}

pub(crate) fn spawn_building_silhouette(commands: &mut Commands, building: &Building, side: Team) {
    let pos = cell_to_world(side, building.lane, building.zone, building.cell);
    let mut spawned = Vec::new();
    spawned.push(spawn_rect(
        commands,
        Vec2::new(pos.x, pos.y - 7.0),
        Vec2::new(CELL - 7.0, CELL - 7.0),
        Color::srgba(0.015, 0.014, 0.013, 0.50),
        2.7,
    ));
    spawned.push(spawn_rect(
        commands,
        Vec2::new(pos.x, pos.y + 2.0),
        Vec2::new(CELL - 13.0, CELL - 13.0),
        Color::srgba(0.42, 0.38, 0.30, 0.24),
        3.0,
    ));
    spawned.push(spawn_rect(
        commands,
        Vec2::new(pos.x, pos.y + 14.0),
        Vec2::new(CELL - 18.0, 3.0),
        Color::srgba(0.72, 0.66, 0.48, 0.18),
        3.2,
    ));
    for entity in spawned {
        commands.entity(entity).insert(StaticScene);
    }
}

pub(crate) fn spawn_castle(
    commands: &mut Commands,
    castle: &SimCastle,
    race: RaceKind,
    building_icons: &BuildingIconAssets,
) {
    let pos = castle_world_pos(castle.team);
    let mut spawned = Vec::new();
    spawned.push(spawn_rect(
        commands,
        Vec2::new(pos.x, pos.y - 24.0),
        Vec2::new(104.0, 20.0),
        Color::srgba(0.02, 0.018, 0.012, 0.48),
        5.2,
    ));
    let mut sprite = Sprite::from_image(castle_icon_handle(building_icons, race));
    sprite.custom_size = Some(Vec2::splat(108.0));
    sprite.flip_x = castle.team == Team::Right;
    spawned.push(
        commands
            .spawn((
                sprite,
                Transform::from_xyz(pos.x, pos.y + 26.0, 7.2),
                SceneEntity,
            ))
            .id(),
    );

    let health_pct = castle.health.max(0) as f32 / castle.max_health.max(1) as f32;
    spawned.push(spawn_rect(
        commands,
        Vec2::new(pos.x, pos.y + 94.0),
        Vec2::new(104.0, 11.0),
        Color::srgba(0.05, 0.04, 0.03, 0.88),
        9.0,
    ));
    spawned.push(spawn_rect(
        commands,
        Vec2::new(pos.x - 104.0 * (1.0 - health_pct) * 0.5, pos.y + 94.0),
        Vec2::new(104.0 * health_pct, 11.0),
        team_color(castle.team),
        10.0,
    ));
    spawned.push(spawn_rect(
        commands,
        Vec2::new(pos.x, pos.y - 12.0),
        Vec2::new(108.0, 5.0),
        team_color(castle.team).with_alpha(0.85),
        5.3,
    ));
    for entity in spawned {
        commands.entity(entity).insert(StaticScene);
    }
}

/// Spawn a unit as a persistent hierarchy: one root that `animate_units`
/// moves every frame, with shadow/sprite/badges/health bar as children.
pub(crate) fn spawn_unit_visual(
    commands: &mut Commands,
    unit: &Unit,
    side: Team,
    balance: &BalanceConfig,
    unit_assets: &UnitSpriteAssets,
) -> UnitVisual {
    let config = balance.unit(unit.kind);
    let sprite_size = unit_sprite_size(unit.kind);
    let world = sim_pos_to_world(unit.pos);
    let root = commands
        .spawn((
            Transform::from_xyz(world.x, world.y, UNIT_ROOT_Z),
            SceneEntity,
        ))
        .id();
    let shadow = spawn_rect(
        commands,
        Vec2::new(0.0, -15.0),
        Vec2::new(sprite_size.x * 0.50, 8.0),
        Color::srgba(0.02, 0.018, 0.014, 0.42),
        -0.6,
    );
    let mut sprite = Sprite::from_image(unit_sprite_handle(unit_assets, unit.kind));
    sprite.custom_size = Some(sprite_size);
    sprite.flip_x = side == Team::Right;
    sprite.color = match side {
        Team::Left => Color::srgb(0.86, 0.92, 1.0),
        Team::Right => Color::srgb(1.0, 0.90, 0.88),
    };
    let sprite_entity = commands
        .spawn((sprite, Transform::from_xyz(0.0, 12.0, 0.0)))
        .id();
    let team_stripe = spawn_rect(
        commands,
        Vec2::new(0.0, -20.0),
        Vec2::new(sprite_size.x * 0.55, 4.5),
        team_color(side).with_alpha(0.85),
        -0.45,
    );
    let badge_attack = spawn_rect(
        commands,
        Vec2::new(-9.0, 27.0),
        Vec2::new(15.0, 4.0),
        attack_type_color(config.attack_type),
        2.6,
    );
    let badge_armor = spawn_rect(
        commands,
        Vec2::new(9.0, 27.0),
        Vec2::new(15.0, 4.0),
        armor_type_color(config.armor_type),
        2.6,
    );
    spawn_rect(
        commands,
        Vec2::new(0.0, -24.0),
        Vec2::new(UNIT_HEALTH_BAR_W, 4.0),
        Color::srgb(0.10, 0.11, 0.11),
        0.6,
    );
    let health_pct = (unit.health.max(0) as f32 / config.max_health.max(1) as f32).clamp(0.0, 1.0);
    let health_fill = spawn_rect(
        commands,
        Vec2::new(-(UNIT_HEALTH_BAR_W * (1.0 - health_pct)) / 2.0, -24.0),
        Vec2::new(UNIT_HEALTH_BAR_W * health_pct, 4.0),
        team_color(side),
        1.6,
    );
    commands.entity(root).add_children(&[
        shadow,
        team_stripe,
        sprite_entity,
        badge_attack,
        badge_armor,
        health_fill,
    ]);
    UnitVisual {
        root,
        sprite: sprite_entity,
        badge_attack,
        badge_armor,
        health_fill,
        health: unit.health,
        sprite_base_y: 12.0,
        badge_base_y: 27.0,
        kind: unit.kind,
        side,
        last_pos: world,
    }
}

pub(crate) fn spawn_rect(
    commands: &mut Commands,
    pos: Vec2,
    size: Vec2,
    color: Color,
    z: f32,
) -> Entity {
    commands
        .spawn((
            Sprite::from_color(color, size),
            Transform::from_xyz(pos.x, pos.y, z),
            SceneEntity,
        ))
        .id()
}

pub(crate) fn spawn_diamond(
    commands: &mut Commands,
    pos: Vec2,
    size: Vec2,
    color: Color,
    z: f32,
) -> Entity {
    commands
        .spawn((
            Sprite::from_color(color, size),
            Transform::from_xyz(pos.x, pos.y, z)
                .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_4)),
            SceneEntity,
        ))
        .id()
}

pub(crate) fn attack_type_color(kind: AttackType) -> Color {
    match kind {
        AttackType::Normal => Color::srgb(0.86, 0.82, 0.68),
        AttackType::Pierce => Color::srgb(0.45, 0.78, 0.95),
        AttackType::Magic => Color::srgb(0.70, 0.48, 0.95),
        AttackType::Siege => Color::srgb(0.91, 0.62, 0.28),
        AttackType::Chaos => Color::srgb(0.94, 0.24, 0.20),
    }
}

pub(crate) fn armor_type_color(kind: ArmorType) -> Color {
    match kind {
        ArmorType::Normal => Color::srgb(0.72, 0.70, 0.62),
        ArmorType::Light => Color::srgb(0.58, 0.88, 0.52),
        ArmorType::Heavy => Color::srgb(0.49, 0.60, 0.72),
        ArmorType::Fortified => Color::srgb(0.72, 0.54, 0.34),
        ArmorType::Unarmored => Color::srgb(0.86, 0.75, 0.52),
    }
}

pub(crate) fn lane_to_world(pos: f32) -> f32 {
    (pos / LANE_LENGTH - 0.5) * (WORLD_W - 120.0)
}

pub(crate) fn cell_to_world(team: Team, lane: Lane, zone: BuildZone, cell: GridCell) -> Vec2 {
    let x = lane_to_world(building_lane_pos(team, zone, cell));
    let y = lane_world_y(lane) + (cell.y as f32 - (GRID_H - 1) as f32 * 0.5) * CELL;
    Vec2::new(x, y)
}
