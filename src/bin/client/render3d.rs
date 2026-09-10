//! 3D world renderer (plan-0.2.md M0): WC3-style low-poly foundations.
//!
//! Renders the same authoritative snapshots as the 2D scene, but in a
//! perspective world: heightfield terrain (castle highlands, sunken lanes,
//! central water channel with bridges), one shared sun with shadow mapping,
//! and the existing 2D sprites as camera-facing billboard quads. The HUD,
//! sim, and protocol are untouched; `--renderer2d` switches back to the v0.1
//! renderer via `is_3d()`.
#![allow(unused_imports)]
pub(crate) use super::audio::*;
pub(crate) use super::input::*;
pub(crate) use super::net::*;
pub(crate) use super::scene::*;
pub(crate) use super::ui::*;
pub(crate) use super::vfx::*;
use super::*;
use bevy::asset::RenderAssetUsages;
use bevy::camera::Camera3d;
use bevy::color::Mix;
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::light::{
    AmbientLight, CascadeShadowConfigBuilder, DirectionalLight, DirectionalLightShadowMap,
    NotShadowCaster,
};
use bevy::mesh::{Mesh3d, PrimitiveTopology};
use bevy::pbr::{MeshMaterial3d, StandardMaterial};
use bevy::render::alpha::AlphaMode;
use bevy::scene::SceneRoot;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicBool, Ordering};

static RENDER_3D: AtomicBool = AtomicBool::new(true);

pub(crate) fn set_renderer_3d(enabled: bool) {
    RENDER_3D.store(enabled, Ordering::Relaxed);
}

pub(crate) fn is_3d() -> bool {
    RENDER_3D.load(Ordering::Relaxed)
}

/// Camera pitch (plan-0.2.md §6) and the base distance that reproduces the
/// v0.1 ortho scale 1.0 vertical coverage at fov 40°.
const CAM_PITCH_DEG: f32 = 50.0;
const CAM_BASE_DIST: f32 = 980.0;
const CAM_ZOOM_MIN: f32 = 0.35;
const CAM_ZOOM_MAX: f32 = 2.4;
const CAM_FOV_DEG: f32 = 40.0;

/// Terrain footprint and height profile (plan-0.2.md §4): castle highlands
/// rise toward the map ends, lanes run as gentle fields, the middle is a
/// sunken water channel spanned by stone bridges on each lane.
const TER_X: (f32, f32) = (-1900.0, 1900.0);
const TER_Z: (f32, f32) = (-340.0, 340.0);
const TER_STEP_X: f32 = 50.0;
const TER_STEP_Z: f32 = 17.0;

const WATER_Y: f32 = -11.0;
const CHANNEL_DEPTH: f32 = -34.0;

const LANE_ZS_2D: [f32; 4] = [192.0, 64.0, -64.0, -192.0];

/// Deterministic terrain height for a point in 2D world coordinates.
pub(crate) fn ground_height(x2d: f32, y2d: f32) -> f32 {
    let ax = x2d.abs();
    let rise = smoothstep(560.0, 1520.0, ax);
    let mut h = 64.0 * rise
        + 26.0 * smoothstep(1250.0, 1750.0, ax)
        + CHANNEL_DEPTH * smoothstep(150.0, 40.0, ax)
        + value_noise(x2d, y2d) * 3.4 * (1.0 - rise);
    // Stone bridge decks flatten the channel crossing on every lane.
    let mut bridge = 0.0_f32;
    for &lane_y in &LANE_ZS_2D {
        let across = 1.0 - smoothstep(44.0, 64.0, (y2d - lane_y).abs());
        let along = 1.0 - smoothstep(110.0, 165.0, ax);
        bridge = bridge.max(across * along);
    }
    h = f32_lerp(h, 6.0, bridge);
    h
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn f32_lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn rise_at(x2d: f32) -> f32 {
    smoothstep(560.0, 1520.0, x2d.abs())
}

fn value_noise(x: f32, y: f32) -> f32 {
    const CELL: f32 = 90.0;
    let fx = x / CELL;
    let fy = y / CELL;
    let ix = fx.floor() as i32;
    let iy = fy.floor() as i32;
    let tx = fx - ix as f32;
    let ty = fy - iy as f32;
    let sx = tx * tx * (3.0 - 2.0 * tx);
    let sy = ty * ty * (3.0 - 2.0 * ty);
    let corner = |gx: i32, gy: i32| hash_unit((gx as u32).wrapping_mul(73) ^ (gy as u32), 911);
    let top = f32_lerp(corner(ix, iy), corner(ix + 1, iy), sx);
    let bottom = f32_lerp(corner(ix, iy + 1), corner(ix + 1, iy + 1), sx);
    f32_lerp(top, bottom, sy)
}

/// Convert a 2D world point (v0.1 map space) to 3D space on the ground.
/// The v0.1 map vertical becomes -Z so screen-right stays map-down.
pub(crate) fn world2_to_3d(pos: Vec2) -> Vec3 {
    Vec3::new(pos.x, ground_height(pos.x, pos.y), -pos.y)
}

/// glTF scenes built by tools/blender (plan-0.2.md §5). Missing or
/// still-loading models fall back to the sprite billboards.
#[derive(Resource, Default)]
pub(crate) struct ModelAssets {
    pub buildings: HashMap<String, Handle<Scene>>,
    pub units: HashMap<String, Handle<Scene>>,
    pub castles: HashMap<String, Handle<Scene>>,
}

pub(crate) const VANGUARD_BUILDING_MODELS: [&str; 8] = [
    "vanguard_barracks",
    "vanguard_range_tower",
    "vanguard_forge",
    "vanguard_pike_yard",
    "vanguard_bulwark_hall",
    "vanguard_chapel",
    "vanguard_stables",
    "vanguard_siege_workshop",
];

pub(crate) const VANGUARD_UNIT_MODELS: [&str; 7] = [
    "vanguard_guard",
    "vanguard_archer",
    "vanguard_pikeman",
    "vanguard_shieldbearer",
    "vanguard_battle_cleric",
    "vanguard_lancer",
    "vanguard_ballista",
];

pub(crate) fn setup_models(mut commands: Commands, assets: AssetServer) {
    let mut models = ModelAssets::default();
    for name in VANGUARD_BUILDING_MODELS {
        models.buildings.insert(
            name.to_string(),
            assets.load(format!("models/vanguard/{name}.glb#Scene0")),
        );
    }
    for name in VANGUARD_UNIT_MODELS {
        models.units.insert(
            name.to_string(),
            assets.load(format!("models/vanguard/{name}.glb#Scene0")),
        );
    }
    models.castles.insert(
        "vanguard_castle".to_string(),
        assets.load("models/vanguard/vanguard_castle.glb#Scene0"),
    );
    commands.insert_resource(models);
}

/// `VanguardBarracks` -> `vanguard_barracks` (Blender manifest naming).
fn snake_case(debug_name: &str) -> String {
    let mut out = String::with_capacity(debug_name.len() + 4);
    for (i, ch) in debug_name.chars().enumerate() {
        if ch.is_ascii_uppercase() {
            if i > 0 {
                out.push('_');
            }
            out.extend(ch.to_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

/// Model name for a building kind (upgrade branches reuse their base).
fn building_model_name(kind: BuildingKind) -> String {
    let base = match kind {
        BuildingKind::VanguardArbalestTower => "vanguard_range_tower",
        BuildingKind::VanguardArcaneSpire => "vanguard_chapel",
        BuildingKind::GroveBrambleWarren => "grove_root_den",
        BuildingKind::GroveSpitefen => "grove_thorn_spire",
        BuildingKind::EmberMagmaForge => "ember_cinder_pit",
        BuildingKind::EmberAshPack => "ember_flame_spire",
        _ => return snake_case(&format!("{kind:?}")),
    };
    base.to_string()
}

fn unit_model_name(kind: UnitKind) -> Option<String> {
    // Upgrade units reuse their base model (documented in assets.md).
    let base = match kind {
        UnitKind::VanguardArbalester => UnitKind::VanguardArcher,
        UnitKind::VanguardArcanist => UnitKind::VanguardBattleCleric,
        _ => kind,
    };
    let name = snake_case(&format!("{base:?}"));
    (VANGUARD_UNIT_MODELS.contains(&name.as_str())).then_some(name)
}

/// Client-side placement effects: recent placement cells drive both the
/// animated build circle and the scaffold window on fresh buildings
/// (plan-0.2.md §7.3).
#[derive(Resource, Default)]
pub(crate) struct BuildFx {
    pub placements: Vec<(Team, Lane, BuildZone, GridCell, f32)>,
}

impl BuildFx {
    /// True while the given cell's building should show its scaffold.
    pub fn under_construction(
        &self,
        team: Team,
        lane: Lane,
        zone: BuildZone,
        cell: GridCell,
        now: f32,
    ) -> bool {
        self.placements.iter().any(|(t, l, z, c, at)| {
            *t == team && *l == lane && *z == zone && *c == cell && now - *at < 3.0
        })
    }
}

/// Records a placement intent for the animated build circle; called from
/// `placement_input` when the server-bound intent is sent.
pub(crate) fn note_placement(
    fx: &mut BuildFx,
    time: f32,
    team: Team,
    lane: Lane,
    zone: BuildZone,
    cell: GridCell,
) {
    fx.placements.push((team, lane, zone, cell, time));
}

/// Animated WC3-style build circle: an expanding, fading ring on the ground.
pub(crate) fn update_build_fx(
    mut commands: Commands,
    mut fx: ResMut<BuildFx>,
    time: Res<Time>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut world_assets: ResMut<World3dAssets>,
) {
    let now = time.elapsed_secs();
    // Circles live 0.6 s; scaffold lookup keeps 3 s of placement history.
    fx.placements.retain(|(_, _, _, _, at)| now - *at < 3.0);
    let fresh: Vec<_> = fx
        .placements
        .iter()
        .filter(|(_, _, _, _, at)| now - *at < 0.6)
        .copied()
        .collect();
    if fresh.is_empty() {
        return;
    }
    let mut res = Res3d {
        meshes: &mut meshes,
        materials: &mut materials,
        assets: &mut world_assets,
    };
    for (team, lane, zone, cell, at) in fresh {
        let t = ((now - at) / 0.6).clamp(0.0, 1.0);
        let pos = cell_to_world(team, lane, zone, cell);
        let base = world2_to_3d(pos);
        let alpha = (1.0 - t) * 0.9;
        let ring = res.ring_texture();
        let mat = res.materials.add(StandardMaterial {
            base_color: team_color(team).with_alpha(alpha),
            base_color_texture: Some(ring),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            cull_mode: None,
            ..default()
        });
        let size = 10.0 + t * (CELL * 1.2);
        commands.spawn((
            Mesh3d(res.flat_quad(size, size)),
            MeshMaterial3d(mat),
            Transform::from_translation(base + Vec3::Y * 0.8),
            NotShadowCaster,
        ));
    }
}

/// Shared meshes/materials for everything billboard-shaped, cached by size
/// (quantized to 0.25) and color so batching stays effective.
#[derive(Resource, Default)]
pub(crate) struct World3dAssets {
    flat_quads: HashMap<(u32, u32), Handle<Mesh>>,
    stand_quads: HashMap<(u32, u32), Handle<Mesh>>,
    flat_mats: HashMap<u64, Handle<StandardMaterial>>,
    tex_mats: HashMap<u64, Handle<StandardMaterial>>,
    tile_mesh: Option<Handle<Mesh>>,
    tile_texture: Option<Handle<Image>>,
    tile_mats: HashMap<u64, Handle<StandardMaterial>>,
    ring_texture: Option<Handle<Image>>,
}

/// Bundle of mutable asset accesses shared by the spawn helpers.
pub(crate) struct Res3d<'a> {
    pub meshes: &'a mut Assets<Mesh>,
    pub materials: &'a mut Assets<StandardMaterial>,
    pub assets: &'a mut World3dAssets,
}

fn color_key(color: Color) -> u64 {
    let s = color.to_srgba();
    (s.red.to_bits() as u64)
        | ((s.green.to_bits() as u64) << 16)
        | ((s.blue.to_bits() as u64) << 32)
        | ((s.alpha.to_bits() as u64) << 48)
}

fn cached_flat_mat(
    materials: &mut Assets<StandardMaterial>,
    assets: &mut World3dAssets,
    color: Color,
) -> Handle<StandardMaterial> {
    let key = color_key(color);
    assets
        .flat_mats
        .entry(key)
        .or_insert_with(|| {
            materials.add(StandardMaterial {
                base_color: color,
                unlit: true,
                alpha_mode: if color.to_srgba().alpha < 0.999 {
                    AlphaMode::Blend
                } else {
                    AlphaMode::Opaque
                },
                cull_mode: None,
                ..default()
            })
        })
        .clone()
}

impl<'a> Res3d<'a> {
    pub(crate) fn flat_quad(&mut self, w: f32, h: f32) -> Handle<Mesh> {
        let key = ((w * 4.0).round() as u32, (h * 4.0).round() as u32);
        self.assets
            .flat_quads
            .entry(key)
            .or_insert_with(|| self.meshes.add(quad_mesh(w, h, QuadPlane::Ground)))
            .clone()
    }

    pub(crate) fn stand_quad(&mut self, w: f32, h: f32) -> Handle<Mesh> {
        let key = ((w * 4.0).round() as u32, (h * 4.0).round() as u32);
        self.assets
            .stand_quads
            .entry(key)
            .or_insert_with(|| self.meshes.add(quad_mesh(w, h, QuadPlane::Standing)))
            .clone()
    }

    pub(crate) fn flat_mat(&mut self, color: Color) -> Handle<StandardMaterial> {
        cached_flat_mat(self.materials, self.assets, color)
    }

    /// Beveled build-footprint tile (shared mesh created at startup,
    /// per-tint material).
    pub(crate) fn tile_quad(&mut self) -> Handle<Mesh> {
        self.assets.tile_mesh.as_ref().expect("tile mesh").clone()
    }

    pub(crate) fn ring_texture(&mut self) -> Handle<Image> {
        self.assets
            .ring_texture
            .as_ref()
            .expect("ring texture")
            .clone()
    }

    pub(crate) fn tile_mat(&mut self, tint: Color) -> Handle<StandardMaterial> {
        let key = color_key(tint);
        let texture = self
            .assets
            .tile_texture
            .as_ref()
            .expect("tile texture")
            .clone();
        self.assets
            .tile_mats
            .entry(key)
            .or_insert_with(|| {
                self.materials.add(StandardMaterial {
                    base_color: tint,
                    base_color_texture: Some(texture),
                    unlit: true,
                    alpha_mode: AlphaMode::Blend,
                    cull_mode: None,
                    ..default()
                })
            })
            .clone()
    }

    pub(crate) fn tex_mat(
        &mut self,
        texture: &Handle<Image>,
        tint: Color,
    ) -> Handle<StandardMaterial> {
        let mut hasher = DefaultHasher::new();
        texture.id().hash(&mut hasher);
        color_key(tint).hash(&mut hasher);
        let key = hasher.finish();
        self.assets
            .tex_mats
            .entry(key)
            .or_insert_with(|| {
                self.materials.add(StandardMaterial {
                    base_color: tint,
                    base_color_texture: Some(texture.clone()),
                    unlit: true,
                    alpha_mode: AlphaMode::Blend,
                    cull_mode: None,
                    ..default()
                })
            })
            .clone()
    }
}

enum QuadPlane {
    Ground,
    Standing,
}

fn quad_mesh(w: f32, h: f32, plane: QuadPlane) -> Mesh {
    let (hw, hh) = (w * 0.5, h * 0.5);
    let (positions, normals, uvs): (Vec<[f32; 3]>, Vec<[f32; 3]>, Vec<[f32; 2]>) = match plane {
        QuadPlane::Ground => (
            vec![
                [-hw, 0.0, -hh],
                [hw, 0.0, hh],
                [hw, 0.0, -hh],
                [-hw, 0.0, -hh],
                [-hw, 0.0, hh],
                [hw, 0.0, hh],
            ],
            vec![[0.0, 1.0, 0.0]; 6],
            vec![
                [0.0, 1.0],
                [1.0, 1.0],
                [1.0, 0.0],
                [0.0, 1.0],
                [1.0, 0.0],
                [0.0, 0.0],
            ],
        ),
        QuadPlane::Standing => (
            vec![
                [-hw, -hh, 0.0],
                [hw, -hh, 0.0],
                [hw, hh, 0.0],
                [-hw, -hh, 0.0],
                [hw, hh, 0.0],
                [-hw, hh, 0.0],
            ],
            vec![[0.0, 0.0, 1.0]; 6],
            vec![
                [0.0, 1.0],
                [1.0, 1.0],
                [1.0, 0.0],
                [0.0, 1.0],
                [1.0, 0.0],
                [0.0, 0.0],
            ],
        ),
    };
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
}

/// Y-axis billboard: reorients around world up to face the 3D camera.
#[derive(Component)]
pub(crate) struct Billboard;

/// Marker for short-lived 3D combat effects (drives the 3D vfx updater).
#[derive(Component)]
pub(crate) struct VfxBillboard;

#[derive(Resource)]
pub(crate) struct CameraRig {
    /// Look-at target in 2D world coordinates.
    pub target: Vec2,
    /// Zoom factor; 1.0 matches the v0.1 default view height.
    pub zoom: f32,
    /// +1 when the viewer looks toward +X (left side), -1 otherwise.
    pub look_sign: f32,
    pub initialized_for: Option<Team>,
}

impl Default for CameraRig {
    fn default() -> Self {
        Self {
            target: Vec2::ZERO,
            zoom: 1.0,
            look_sign: 1.0,
            initialized_for: None,
        }
    }
}

impl CameraRig {
    fn dist(&self) -> f32 {
        (CAM_BASE_DIST * self.zoom).clamp(420.0, 3300.0)
    }

    pub(crate) fn home_target(team: Team) -> Vec2 {
        let castle_x = lane_to_world(team.castle_pos());
        Vec2::new(castle_x + team.direction() * 290.0, 0.0)
    }
}

/// Startup: 3D camera, sun, terrain, water. The UI Camera2d is spawned by
/// `setup()` in main.rs with IsDefaultUiCamera + higher render order here.
pub(crate) fn setup_3d_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut world_assets: ResMut<World3dAssets>,
) {
    if !is_3d() {
        return;
    }
    // Shared build-footprint tile (beveled, textured) for the placement UI.
    world_assets.tile_mesh = Some(meshes.add(tile_mesh()));
    world_assets.tile_texture = Some(images.add(footprint_tile_texture()));
    world_assets.ring_texture = Some(images.add(ring_texture()));

    let mut res = Res3d {
        meshes: &mut meshes,
        materials: &mut materials,
        assets: &mut world_assets,
    };

    commands.spawn((
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: CAM_FOV_DEG.to_radians(),
            near: 2.0,
            far: 7000.0,
            ..default()
        }),
        Camera {
            order: 0,
            ..default()
        },
        Msaa::Off,
        Transform::from_xyz(-1600.0, 1100.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
        // Camera-attached ambient override (bevy 0.18: AmbientLight is a
        // component on a camera), tuned warm for the outdoor scene.
        AmbientLight {
            color: Color::srgb(0.80, 0.86, 1.00),
            brightness: 700.0,
            affects_lightmapped_meshes: true,
        },
    ));

    // Warm afternoon sun from the viewer's back-left; the single shared
    // light is the visual-consistency anchor (plan-0.2.md §6).
    commands.spawn((
        DirectionalLight {
            illuminance: 15000.0,
            shadows_enabled: true,
            ..default()
        },
        CascadeShadowConfigBuilder {
            first_cascade_far_bound: 500.0,
            maximum_distance: 3200.0,
            ..default()
        }
        .build(),
        Transform::from_xyz(-1400.0, 2100.0, 900.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.insert_resource(DirectionalLightShadowMap { size: 2048 });

    spawn_terrain(&mut commands, &mut res, &mut images);
}

/// Flat XZ quad with UVs for the footprint tile texture.
fn tile_mesh() -> Mesh {
    let hw = (CELL - 2.0) * 0.5;
    let mut mesh = quad_mesh(CELL - 2.0, CELL - 2.0, QuadPlane::Ground);
    // quad_mesh already emits UVs; texture spacing handles the bevel.
    let _ = (hw, &mut mesh);
    mesh
}

/// Bright ring on transparent ground, used by the build circle.
fn ring_texture() -> Image {
    const S: usize = 64;
    let mut data = Vec::with_capacity(S * S * 4);
    for py in 0..S {
        for px in 0..S {
            let dx = px as f32 / (S - 1) as f32 - 0.5;
            let dy = py as f32 / (S - 1) as f32 - 0.5;
            let d = (dx * dx + dy * dy).sqrt() * 2.0; // 0 center, 1 edge
            let ring = 1.0 - smoothstep(0.72, 0.95, d);
            let alpha = smoothstep(0.55, 0.8, d) * ring;
            data.extend([255, 235, 170, (alpha * 255.0) as u8]);
        }
    }
    Image::new(
        bevy::render::render_resource::Extent3d {
            width: S as u32,
            height: S as u32,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        data,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
}

/// Soft beveled square with a bright inner inset, alpha outside.
fn footprint_tile_texture() -> Image {
    const S: usize = 64;
    let mut data = Vec::with_capacity(S * S * 4);
    for py in 0..S {
        for px in 0..S {
            let x = px as f32 / (S - 1) as f32;
            let y = py as f32 / (S - 1) as f32;
            let edge = x.min(y).min(1.0 - x).min(1.0 - y); // 0 at border
            let border = smoothstep(0.0, 0.08, edge); // frame
            let inset = smoothstep(0.12, 0.30, edge); // inner fill window
            let alpha = 0.95 * border.max(0.55 * inset);
            let bright = 1.0 + 0.35 * (1.0 - inset);
            data.extend([
                ((1.0 * bright).min(1.0) * 255.0) as u8,
                ((1.0 * bright).min(1.0) * 255.0) as u8,
                (1.0 * 255.0) as u8,
                (alpha * 255.0) as u8,
            ]);
        }
    }
    Image::new(
        bevy::render::render_resource::Extent3d {
            width: S as u32,
            height: S as u32,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        data,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
}

fn spawn_terrain(commands: &mut Commands, res: &mut Res3d, images: &mut Assets<Image>) {
    let x0 = TER_X.0;
    let z0 = TER_Z.0;
    let cols = ((TER_X.1 - TER_X.0) / TER_STEP_X).round() as usize;
    let rows = ((TER_Z.1 - TER_Z.0) / TER_STEP_Z).round() as usize;

    let height_at = |gx: usize, gz: usize| -> f32 {
        let x = x0 + gx as f32 * TER_STEP_X;
        let z = z0 + gz as f32 * TER_STEP_Z;
        ground_height(x, -z)
    };

    // Positions/normals for the flat-shaded heightfield; albedo comes from a
    // generated texture (plan-0.2.md §4) so the terrain reads as one painted
    // surface instead of per-facet blocks.
    let quads = cols * rows;
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(quads * 6);
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(quads * 6);
    let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(quads * 6);

    for gz in 0..rows {
        for gx in 0..cols {
            let x_a = x0 + gx as f32 * TER_STEP_X;
            let z_a = z0 + gz as f32 * TER_STEP_Z;
            let x_b = x_a + TER_STEP_X;
            let z_b = z_a + TER_STEP_Z;
            let h_aa = height_at(gx, gz);
            let h_ba = height_at(gx + 1, gz);
            let h_bb = height_at(gx + 1, gz + 1);
            let h_ab = height_at(gx, gz + 1);

            // +Y facet normal (winding below is CCW seen from above).
            let e1 = Vec3::new(TER_STEP_X, h_ba - h_aa, 0.0);
            let e2 = Vec3::new(0.0, h_ab - h_aa, TER_STEP_Z);
            let n = e2.cross(e1).normalize_or_zero();

            let quad_pos = [
                [x_a, h_aa, z_a],
                [x_b, h_bb, z_b],
                [x_b, h_ba, z_a],
                [x_a, h_aa, z_a],
                [x_a, h_ab, z_b],
                [x_b, h_bb, z_b],
            ];
            let u_a = gx as f32 / cols as f32;
            let v_a = gz as f32 / rows as f32;
            let u_b = (gx + 1) as f32 / cols as f32;
            let v_b = (gz + 1) as f32 / rows as f32;
            let quad_uv = [
                [u_a, v_a],
                [u_b, v_a],
                [u_b, v_b],
                [u_a, v_a],
                [u_b, v_b],
                [u_a, v_b],
            ];
            positions.extend(quad_pos);
            normals.extend(std::iter::repeat([n.x, n.y, n.z]).take(6));
            uvs.extend(quad_uv);
        }
    }

    let mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs);

    let terrain_texture = images.add(terrain_albedo_texture());
    let terrain_material = res.materials.add(StandardMaterial {
        base_color_texture: Some(terrain_texture),
        cull_mode: None,
        perceptual_roughness: 0.95,
        reflectance: 0.05,
        ..default()
    });
    commands.spawn((
        Mesh3d(res.meshes.add(mesh)),
        MeshMaterial3d(terrain_material),
        Transform::IDENTITY,
    ));

    // Shallow water surface over the channel.
    let water_w = 330.0;
    let water_d = TER_Z.1 - TER_Z.0 + 40.0;
    commands.spawn((
        Mesh3d(res.flat_quad(water_w, water_d)),
        MeshMaterial3d(res.materials.add(StandardMaterial {
            base_color: Color::srgba(0.18, 0.40, 0.52, 0.66),
            metallic: 0.25,
            perceptual_roughness: 0.12,
            alpha_mode: AlphaMode::Blend,
            cull_mode: None,
            ..default()
        })),
        Transform::from_xyz(0.0, WATER_Y, 0.0),
        NotShadowCaster,
    ));
}

/// Paint the terrain albedo (grass, lane roads, highland stone, channel bed,
/// bridge decks) into a texture — same rules the 2D renderer's palette used.
fn terrain_albedo_texture() -> Image {
    const W: usize = 512;
    const H: usize = 256;
    let mut data = Vec::with_capacity(W * H * 4);
    for py in 0..H {
        for px in 0..W {
            let u = px as f32 / (W - 1) as f32;
            let v = py as f32 / (H - 1) as f32;
            let x2d = TER_X.0 + u * (TER_X.1 - TER_X.0);
            let z3d = TER_Z.0 + v * (TER_Z.1 - TER_Z.0);
            let y2d = -z3d;
            let h = ground_height(x2d, y2d);
            let c = terrain_color(x2d, y2d, h).to_srgba();
            data.extend([
                (c.red * 255.0).round() as u8,
                (c.green * 255.0).round() as u8,
                (c.blue * 255.0).round() as u8,
                255,
            ]);
        }
    }
    Image::new(
        bevy::render::render_resource::Extent3d {
            width: W as u32,
            height: H as u32,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        data,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
}

pub(crate) fn terrain_color(x2d: f32, y2d: f32, h: f32) -> Color {
    let grass_a = Color::srgb(0.34, 0.47, 0.22);
    let grass_b = Color::srgb(0.29, 0.42, 0.19);
    let dirt = Color::srgb(0.50, 0.41, 0.27);
    let stone = Color::srgb(0.53, 0.50, 0.44);
    let sand = Color::srgb(0.58, 0.53, 0.40);
    let bridge_stone = Color::srgb(0.46, 0.44, 0.40);

    let mut color = grass_a.mix(&grass_b, value_noise(x2d, y2d));
    let mut road = 0.0_f32;
    for &lane_y in &LANE_ZS_2D {
        road = road.max(1.0 - smoothstep(24.0, 38.0, (y2d - lane_y).abs()));
    }
    color = color.mix(&dirt, road * 0.85);
    color = color.mix(&stone, smoothstep(0.35, 0.8, rise_at(x2d)));
    if h < WATER_Y + 4.0 {
        color = color.mix(&sand, 0.8);
    }
    let mut deck = 0.0_f32;
    for &lane_y in &LANE_ZS_2D {
        let across = 1.0 - smoothstep(46.0, 60.0, (y2d - lane_y).abs());
        let along = 1.0 - smoothstep(112.0, 160.0, x2d.abs());
        deck = deck.max(across * along);
    }
    color.mix(&bridge_stone, deck)
}

// ---- Static scene (buildings, castles, zone tiles) ----

pub(crate) fn sync_static_3d(
    mut commands: Commands,
    mut registry: ResMut<SceneRegistry>,
    statics: Query<Entity, With<StaticScene>>,
    models: Res<ModelAssets>,
    scenes: Res<Assets<Scene>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut world_assets: ResMut<World3dAssets>,
    building_icons: Res<BuildingIconAssets>,
    selection: Res<BuildSelection>,
    build_fx: Res<BuildFx>,
    time: Res<Time>,
    state: Res<SnapshotState>,
    net: Res<ClientNet>,
    fog: Res<FogMemory>,
) {
    let key = static_scene_key(&state, &net, &fog);
    if key != registry.statics_key {
        registry.statics_key = key;
        for entity in &statics {
            commands.entity(entity).despawn();
        }
        let mut res = Res3d {
            meshes: &mut meshes,
            materials: &mut materials,
            assets: &mut world_assets,
        };
        let lane_count = state
            .snapshot
            .as_ref()
            .map(|s| s.players.len())
            .unwrap_or(2);
        let assigned_lane = net.player_id.and_then(|pid| {
            state.snapshot.as_ref().and_then(|s| {
                let side_index = s
                    .players
                    .iter()
                    .filter(|p| Some(p.team) == net.team)
                    .position(|p| p.id == pid);
                side_index.map(|idx| Lane::for_player(net.team.unwrap_or(Team::Left), idx))
            })
        });
        // M1 §7: the grid only exists while placing; the idle map reads via
        // roads and terrace trim alone.
        if selection.kind.is_some() {
            spawn_zone_tiles_3d(&mut commands, &mut res, net.team, lane_count, assigned_lane);
        }
        if let Some(snapshot) = &state.snapshot {
            for building in &snapshot.buildings {
                let side = side_of_player(snapshot, building.owner);
                if !is_building_visible(snapshot, net.team, building) {
                    if is_enemy_building_scouted(snapshot, &fog, net.team, building) {
                        spawn_building_silhouette_3d(&mut commands, &mut res, building, side);
                    }
                    continue;
                }
                spawn_building_3d(
                    &mut commands,
                    &mut res,
                    &models,
                    &scenes,
                    building,
                    side,
                    &building_icons,
                    build_fx.under_construction(
                        side,
                        building.lane,
                        building.zone,
                        building.cell,
                        time.elapsed_secs(),
                    ),
                );
            }
            for castle in &snapshot.castles {
                if !is_castle_visible(snapshot, net.team, castle.team) {
                    continue;
                }
                spawn_castle_3d(
                    &mut commands,
                    &mut res,
                    &models,
                    &scenes,
                    castle,
                    snapshot,
                    &building_icons,
                );
            }
        }
    }

    // Fog tiles live outside the statics key: spawned once per match,
    // mutated in place by `update_fog_tiles_3d`.
    let want_fog = state.snapshot.is_some() && net.team.is_some();
    if want_fog && registry.fog_tiles.is_empty() {
        let mut res = Res3d {
            meshes: &mut meshes,
            materials: &mut materials,
            assets: &mut world_assets,
        };
        let tile_w = MAP_W / FOG_COLUMNS as f32;
        let tile_h = MAP_H / FOG_ROWS as f32;
        let mesh = res.flat_quad(tile_w + 1.0, tile_h + 1.0);
        for row in 0..FOG_ROWS {
            for col in 0..FOG_COLUMNS {
                let pos = fog_cell_center(col, row);
                let tile = commands
                    .spawn((
                        Mesh3d(mesh.clone()),
                        MeshMaterial3d(res.flat_mat(Color::srgba(0.0, 0.0, 0.0, 0.88))),
                        Transform::from_translation(world2_to_3d(pos) + Vec3::Y * 0.9),
                        Visibility::Hidden,
                        FogTile { col, row },
                    ))
                    .id();
                registry.fog_tiles.push(tile);
            }
        }
    }
}

fn spawn_zone_tiles_3d(
    commands: &mut Commands,
    res: &mut Res3d,
    team: Option<Team>,
    lane_count: usize,
    assigned_lane: Option<Lane>,
) {
    let tint = team
        .map(team_color)
        .unwrap_or(Color::srgb(0.30, 0.50, 0.80));
    for side in visible_sides(team) {
        for lane in &Lane::ALL[..lane_count.min(Lane::ALL.len())] {
            for zone in BuildZone::ALL {
                for x in 0..GRID_W {
                    for y in 0..GRID_H {
                        let pos = cell_to_world(side, *lane, zone, GridCell { x, y });
                        let wrong_lane = assigned_lane.is_some()
                            && Some(side) == team
                            && *lane != assigned_lane.unwrap();
                        let color = if wrong_lane {
                            tint.with_alpha(0.05)
                        } else {
                            tint.with_alpha(0.42)
                        };
                        commands.spawn((
                            Mesh3d(res.tile_quad()),
                            MeshMaterial3d(res.tile_mat(color)),
                            Transform::from_translation(world2_to_3d(pos) + Vec3::Y * 0.35),
                            NotShadowCaster,
                            StaticScene,
                        ));
                    }
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_arguments)]
fn spawn_building_3d(
    commands: &mut Commands,
    res: &mut Res3d,
    models: &ModelAssets,
    scenes: &Assets<Scene>,
    building: &Building,
    side: Team,
    icon_assets: &BuildingIconAssets,
    scaffold: bool,
) {
    let pos = cell_to_world(side, building.lane, building.zone, building.cell);
    let base = world2_to_3d(pos);
    commands.spawn((
        Mesh3d(res.flat_quad(CELL - 8.0, CELL - 8.0)),
        MeshMaterial3d(res.flat_mat(Color::srgba(0.02, 0.018, 0.014, 0.45))),
        Transform::from_translation(base + Vec3::Y * 0.55),
        NotShadowCaster,
        StaticScene,
    ));
    commands.spawn((
        Mesh3d(res.flat_quad(CELL - 10.0, 4.0)),
        MeshMaterial3d(res.flat_mat(team_color(side).with_alpha(0.8))),
        Transform::from_translation(base + Vec3::Y * 0.75 - Vec3::Z * (CELL * 0.34)),
        NotShadowCaster,
        StaticScene,
    ));
    let model_name = building_model_name(building.kind);
    let model = models
        .buildings
        .get(&model_name)
        .filter(|h| scenes.contains(*h));
    if let Some(handle) = model {
        // Real mesh (M2+): modelled in Blender, casts shadows.
        commands.spawn((
            SceneRoot(handle.clone()),
            Transform::from_translation(base + Vec3::Y * 4.0)
                .with_scale(Vec3::splat(if scaffold { 0.62 } else { 1.0 })),
            StaticScene,
        ));
    } else {
        let (icon_h, icon_alpha) = if scaffold { (30.0, 0.7) } else { (52.0, 1.0) };
        commands.spawn((
            Mesh3d(res.stand_quad(icon_h, icon_h)),
            MeshMaterial3d(res.tex_mat(
                &building_icon_handle(icon_assets, building.kind),
                Color::srgba(1.0, 0.0, 1.0, 1.0), // TEST fallback
            )),
            Transform::from_translation(base + Vec3::Y * icon_h * 0.5),
            Billboard,
            NotShadowCaster,
            StaticScene,
        ));
    }
    if scaffold {
        // Construction scaffold: crossed timber posts + top beam.
        let timber = Color::srgb(0.55, 0.40, 0.24);
        for angle in [0.35_f32, -0.35] {
            commands.spawn((
                Mesh3d(res.stand_quad(4.0, 40.0)),
                MeshMaterial3d(res.flat_mat(timber)),
                Transform::from_translation(base + Vec3::Y * 20.0)
                    .with_rotation(Quat::from_rotation_z(angle)),
                NotShadowCaster,
                StaticScene,
            ));
        }
        commands.spawn((
            Mesh3d(res.stand_quad(46.0, 4.0)),
            MeshMaterial3d(res.flat_mat(timber)),
            Transform::from_translation(base + Vec3::Y * 38.0),
            NotShadowCaster,
            StaticScene,
        ));
    }
}

fn spawn_building_silhouette_3d(
    commands: &mut Commands,
    res: &mut Res3d,
    building: &Building,
    side: Team,
) {
    let pos = cell_to_world(side, building.lane, building.zone, building.cell);
    let base = world2_to_3d(pos);
    commands.spawn((
        Mesh3d(res.stand_quad(40.0, 44.0)),
        MeshMaterial3d(res.flat_mat(Color::srgba(0.05, 0.05, 0.06, 0.55))),
        Transform::from_translation(base + Vec3::Y * 22.0),
        Billboard,
        NotShadowCaster,
        StaticScene,
    ));
}

fn spawn_castle_3d(
    commands: &mut Commands,
    res: &mut Res3d,
    models: &ModelAssets,
    scenes: &Assets<Scene>,
    castle: &SimCastle,
    snapshot: &MatchSnapshot,
    icon_assets: &BuildingIconAssets,
) {
    let race = team_race(snapshot, castle.team).unwrap_or(RaceKind::Vanguard);
    let pos = castle_world_pos(castle.team);
    let base = world2_to_3d(pos);
    commands.spawn((
        Mesh3d(res.flat_quad(150.0, 90.0)),
        MeshMaterial3d(res.flat_mat(Color::srgba(0.02, 0.018, 0.014, 0.40))),
        Transform::from_translation(base + Vec3::Y * 0.6),
        NotShadowCaster,
        StaticScene,
    ));
    let castle_name = format!("{race:?}_castle").to_lowercase();
    let model = models
        .castles
        .get(&castle_name)
        .filter(|h| scenes.contains(*h));
    if let Some(handle) = model {
        commands.spawn((
            SceneRoot(handle.clone()),
            Transform::from_translation(base)
                .with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
            StaticScene,
        ));
    } else {
        commands.spawn((
            Mesh3d(res.stand_quad(118.0, 118.0)),
            MeshMaterial3d(res.tex_mat(&castle_icon_handle(icon_assets, race), Color::WHITE)),
            Transform::from_translation(base + Vec3::Y * 60.0),
            Billboard,
            NotShadowCaster,
            StaticScene,
        ));
    }

    // Castle health bar floats above the keep; statics rebuild on damage.
    let health_pct = castle.health.max(0) as f32 / castle.max_health.max(1) as f32;
    commands.spawn((
        Mesh3d(res.flat_quad(116.0, 12.0)),
        MeshMaterial3d(res.flat_mat(Color::srgba(0.05, 0.04, 0.03, 0.9))),
        Transform::from_translation(base + Vec3::Y * 128.0),
        NotShadowCaster,
        StaticScene,
    ));
    commands.spawn((
        Mesh3d(res.flat_quad((116.0 * health_pct).max(1.0), 9.0)),
        MeshMaterial3d(res.flat_mat(team_color(castle.team))),
        Transform::from_translation(
            base + Vec3::Y * 129.0 - Vec3::new(116.0 * (1.0 - health_pct) * 0.5, 0.0, 0.0),
        ),
        NotShadowCaster,
        StaticScene,
    ));
}

// ---- Units ----

#[allow(clippy::too_many_arguments)]
pub(crate) fn spawn_unit_visual_3d(
    commands: &mut Commands,
    res: &mut Res3d,
    models: &ModelAssets,
    scenes: &Assets<Scene>,
    unit: &Unit,
    side: Team,
    balance: &BalanceConfig,
    unit_assets: &UnitSpriteAssets,
) -> UnitVisual {
    let config = balance.unit(unit.kind);
    let s = unit_sprite_size(unit.kind);
    let world = sim_pos_to_world(unit.pos);
    let model_name = unit_model_name(unit.kind);
    let model_handle = model_name
        .as_ref()
        .and_then(|name| models.units.get(name))
        .filter(|h| scenes.contains(*h));
    let is_model = model_handle.is_some();
    let root = commands
        .spawn(Transform::from_translation(
            world2_to_3d(world) + Vec3::Y * 0.45,
        ))
        .id();

    let shadow = commands
        .spawn((
            Mesh3d(res.flat_quad(s.x * 0.55, s.x * 0.4)),
            MeshMaterial3d(res.flat_mat(Color::srgba(0.01, 0.01, 0.012, 0.45))),
            Transform::from_xyz(0.0, 0.05, 0.0),
            NotShadowCaster,
        ))
        .id();
    let stripe = commands
        .spawn((
            Mesh3d(res.flat_quad(s.x * 0.6, 5.0)),
            MeshMaterial3d(res.flat_mat(team_color(side).with_alpha(0.85))),
            Transform::from_xyz(0.0, 0.18, 0.0),
            NotShadowCaster,
        ))
        .id();
    let sprite = commands
        .spawn((
            Mesh3d(res.stand_quad(s.x * 0.92, s.y * 0.92)),
            MeshMaterial3d(
                res.tex_mat(&unit_sprite_handle(unit_assets, unit.kind), team_tint(side)),
            ),
            Transform::from_xyz(0.0, s.y * 0.5 + 2.0, 0.0),
            NotShadowCaster,
        ))
        .id();
    let bar_y = s.y + 8.0;
    let badge_attack = commands
        .spawn((
            Mesh3d(res.flat_quad(9.0, 4.0)),
            MeshMaterial3d(res.flat_mat(attack_type_color(config.attack_type))),
            Transform::from_xyz(-8.0, bar_y + 2.5, 0.0),
            NotShadowCaster,
        ))
        .id();
    let badge_armor = commands
        .spawn((
            Mesh3d(res.flat_quad(9.0, 4.0)),
            MeshMaterial3d(res.flat_mat(armor_type_color(config.armor_type))),
            Transform::from_xyz(8.0, bar_y + 2.5, 0.0),
            NotShadowCaster,
        ))
        .id();
    commands.spawn((
        Mesh3d(res.flat_quad(UNIT_HEALTH_BAR_W + 2.0, 5.0)),
        MeshMaterial3d(res.flat_mat(Color::srgb(0.08, 0.09, 0.09))),
        Transform::from_xyz(0.0, bar_y, 0.0),
        NotShadowCaster,
    ));
    let health_pct = (unit.health.max(0) as f32 / config.max_health.max(1) as f32).clamp(0.0, 1.0);
    let health_fill = commands
        .spawn((
            Mesh3d(res.flat_quad(UNIT_HEALTH_BAR_W, 3.6)),
            MeshMaterial3d(res.flat_mat(team_color(side))),
            Transform::from_xyz(
                -(UNIT_HEALTH_BAR_W * (1.0 - health_pct)) * 0.5,
                bar_y + 0.8,
                0.0,
            ),
            NotShadowCaster,
        ))
        .id();
    commands.entity(root).add_children(&[
        shadow,
        stripe,
        sprite,
        badge_attack,
        badge_armor,
        health_fill,
    ]);
    UnitVisual {
        root,
        sprite,
        badge_attack,
        badge_armor,
        health_fill,
        health: unit.health,
        sprite_base_y: s.y * 0.5 + 2.0,
        badge_base_y: bar_y + 2.5,
        kind: unit.kind,
        side,
        last_pos: world,
        is_model,
    }
}

fn team_tint(side: Team) -> Color {
    match side {
        Team::Left => Color::srgb(0.86, 0.92, 1.0),
        Team::Right => Color::srgb(1.0, 0.90, 0.88),
    }
}

pub(crate) fn sync_units_3d(
    mut commands: Commands,
    mut registry: ResMut<SceneRegistry>,
    models: Res<ModelAssets>,
    scenes: Res<Assets<Scene>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut world_assets: ResMut<World3dAssets>,
    state: Res<SnapshotState>,
    net: Res<ClientNet>,
    unit_assets: Res<UnitSpriteAssets>,
    mut sfx: ResMut<SfxQueue>,
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
        if registry.units.contains_key(&unit.id) {
            continue;
        }
        let side = side_of_player(snapshot, unit.owner);
        let mut res = Res3d {
            meshes: &mut meshes,
            materials: &mut materials,
            assets: &mut world_assets,
        };
        let visual = spawn_unit_visual_3d(
            &mut commands,
            &mut res,
            &models,
            &scenes,
            unit,
            side,
            &balance,
            &unit_assets,
        );
        registry.units.insert(unit.id, visual);
        sfx.push(Sfx::UnitSpawn);
    }
    let stale: Vec<u64> = registry
        .units
        .keys()
        .copied()
        .filter(|id| !alive.contains(id))
        .collect();
    for id in stale {
        if let Some(visual) = registry.units.remove(&id) {
            spawn_corpse_3d(
                &mut commands,
                &mut meshes,
                &mut materials,
                &mut world_assets,
                &unit_assets,
                &visual,
            );
            commands.entity(visual.root).despawn();
        }
    }
}

fn spawn_corpse_3d(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    world_assets: &mut World3dAssets,
    unit_assets: &UnitSpriteAssets,
    visual: &UnitVisual,
) {
    let mut res = Res3d {
        meshes,
        materials,
        assets: world_assets,
    };
    let size = unit_sprite_size(visual.kind);
    let pos = world2_to_3d(visual.last_pos);
    // Unique material per corpse so the fade does not touch live units.
    let material = res.materials.add(StandardMaterial {
        base_color: team_tint(visual.side).with_alpha(0.8),
        base_color_texture: Some(unit_sprite_handle(unit_assets, visual.kind)),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        ..default()
    });
    commands.spawn((
        Mesh3d(res.stand_quad(size.x * 0.92, size.y * 0.92)),
        MeshMaterial3d(material),
        Transform::from_translation(pos + Vec3::Y * 0.5),
        Billboard,
        VfxBillboard,
        CombatVfx {
            lifetime: 0.9,
            max_lifetime: 0.9,
            velocity: Vec2::ZERO,
        },
    ));
}

/// Interpolate unit transforms between snapshots exactly like the 2D path,
/// then place on the terrain and yaw to face the camera.
pub(crate) fn animate_units_3d(
    time: Res<Time>,
    interp: Res<RenderInterp>,
    state: Res<SnapshotState>,
    mut registry: ResMut<SceneRegistry>,
    cam: Query<&GlobalTransform, With<Camera3d>>,
    mut transforms: Query<&mut Transform>,
) {
    if registry.units.is_empty() {
        return;
    }
    let Some(snapshot) = &state.snapshot else {
        return;
    };
    let balance = active_balance(&state);
    let cam_pos = cam.single().map(|t| t.translation()).unwrap_or(Vec3::ZERO);

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
        let Some(visual) = registry.units.get_mut(&unit.id) else {
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
        let pos3 = world2_to_3d(world) + Vec3::Y * 0.45;
        root.translation = pos3;
        if visual.is_model {
            root.rotation = Quat::IDENTITY;
            if let Ok(mut sprite) = transforms.get_mut(visual.sprite) {
                // Face the march direction (models are authored facing +X).
                if unit.velocity.x.abs() > 0.05 {
                    let yaw = if unit.velocity.x > 0.0 {
                        0.0
                    } else {
                        std::f32::consts::PI
                    };
                    sprite.rotation = Quat::from_rotation_y(yaw);
                }
            }
        } else {
            let to_cam = cam_pos - pos3;
            root.rotation = Quat::from_rotation_y(to_cam.x.atan2(to_cam.z));
        }

        // Idle bob on the display clock (same contract as the 2D renderer).
        let phase = time.elapsed_secs() * 4.0 + (unit.id % 97) as f32 * 1.37;
        let bob = phase.sin() * 1.6;
        if let Ok(mut sprite) = transforms.get_mut(visual.sprite) {
            sprite.translation.y = visual.sprite_base_y + bob;
            sprite.scale.y = 1.0 + phase.cos() * 0.02;
        }
        if let Ok(mut badge) = transforms.get_mut(visual.badge_attack) {
            badge.translation.y = visual.badge_base_y + bob * 0.6;
        }
        if let Ok(mut badge) = transforms.get_mut(visual.badge_armor) {
            badge.translation.y = visual.badge_base_y + bob * 0.6;
        }
        if visual.health != unit.health {
            visual.health = unit.health;
            let pct = (unit.health.max(0) as f32
                / balance.unit(unit.kind).max_health.max(1) as f32)
                .clamp(0.0, 1.0);
            if let Ok(mut fill) = transforms.get_mut(visual.health_fill) {
                fill.translation.x = -(UNIT_HEALTH_BAR_W * (1.0 - pct)) * 0.5;
                fill.scale.x = pct.max(0.001);
            }
        }
    }
}

// ---- Object highlight ----

pub(crate) fn update_object_highlight_3d(
    mut commands: Commands,
    mut registry: ResMut<SceneRegistry>,
    state: Res<SnapshotState>,
    world_selection: Res<WorldSelection>,
    world_hover: Res<WorldHover>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut world_assets: ResMut<World3dAssets>,
    mut transforms: Query<&mut Transform>,
) {
    let Some(snapshot) = &state.snapshot else {
        despawn_highlight(&mut commands, &mut registry);
        return;
    };
    let mut source = None;
    let mut spec = Vec2::splat(44.0);
    let mut color = Color::srgba(0.95, 0.76, 0.24, 0.5);
    let mut pos = Vec2::ZERO;

    if let Some(SelectedObject::Unit(id)) = world_selection.selected {
        if snapshot.units.iter().any(|unit| unit.id == id) {
            if let Some(visual) = registry.units.get(&id) {
                if let Ok(t) = transforms.get(visual.root) {
                    spec = Vec2::splat(
                        unit_sprite_size(
                            snapshot
                                .units
                                .iter()
                                .find(|unit| unit.id == id)
                                .map(|unit| unit.kind)
                                .unwrap_or(UnitKind::VanguardGuard),
                        )
                        .x * 0.85,
                    );
                    pos = Vec2::new(t.translation.x, -t.translation.z);
                }
            }
            source = Some(HighlightSource::Unit(id));
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
                    pos = cell_to_world(
                        side_of_player(snapshot, building.owner),
                        building.lane,
                        building.zone,
                        building.cell,
                    );
                    spec = Vec2::splat(CELL + 6.0);
                    source = Some(HighlightSource::Building(id));
                }
            }
            Some(SelectedObject::Castle(team)) => {
                if snapshot.castles.iter().any(|c| c.team == team) {
                    pos = castle_world_pos(team);
                    spec = Vec2::new(170.0, 110.0);
                    color = Color::srgba(0.95, 0.75, 0.26, 0.4);
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
        let mut res = Res3d {
            meshes: &mut meshes,
            materials: &mut materials,
            assets: &mut world_assets,
        };
        let entity = commands
            .spawn((
                Mesh3d(res.flat_quad(spec.x, spec.y)),
                MeshMaterial3d(res.flat_mat(color)),
                Transform::from_translation(world2_to_3d(pos) + Vec3::Y * 0.75),
                NotShadowCaster,
            ))
            .id();
        registry.highlight = Some(entity);
        registry.highlight_source = Some(source);
    } else if let Some(entity) = registry.highlight {
        if let Ok(mut t) = transforms.get_mut(entity) {
            let p3 = world2_to_3d(pos);
            t.translation.x = p3.x;
            t.translation.z = p3.z;
        }
    }
}

// ---- Fog tiles ----

pub(crate) fn update_fog_tiles_3d(
    state: Res<SnapshotState>,
    fog: Res<FogMemory>,
    net: Res<ClientNet>,
    registry: Res<SceneRegistry>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut world_assets: ResMut<World3dAssets>,
    mut tiles: Query<(
        &FogTile,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut Visibility,
    )>,
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
    for (tile, mut mat, mut visibility) in &mut tiles {
        let pos = fog_cell_center(tile.col, tile.row);
        let reveal = world_reveal_strength(snapshot, team, pos);
        let explored = fog.explored[fog_index(tile.col, tile.row)];
        let (alpha, hidden) = if reveal > 0.96 {
            (0.0, true)
        } else if reveal > 0.0 {
            (0.20 * (1.0 - reveal), false)
        } else if explored {
            (0.53, false)
        } else {
            (0.88, false)
        };
        if hidden {
            *visibility = Visibility::Hidden;
            continue;
        }
        *visibility = Visibility::Inherited;
        // Alpha is quantized so the shared-material cache stays small.
        let alpha = (alpha * 10.0).round() / 10.0;
        **mat = cached_flat_mat(
            &mut materials,
            &mut world_assets,
            Color::srgba(0.004, 0.006, 0.009, alpha),
        );
    }
}

// ---- Placement preview ----

pub(crate) fn update_placement_preview_3d(
    mut commands: Commands,
    preview_query: Query<Entity, With<PreviewEntity>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cam3d: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    layout: Res<UiLayout>,
    net: Res<ClientNet>,
    selection: Res<BuildSelection>,
    state: Res<SnapshotState>,
    building_icons: Res<BuildingIconAssets>,
    time: Res<Time>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut world_assets: ResMut<World3dAssets>,
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
    let Some(screen) = cursor_screen_pos(&windows) else {
        return;
    };
    if screen.y <= layout.panel_top_y() {
        return;
    }
    let Some(kind) = selection.kind else {
        return;
    };
    let Some(world) = cursor_ground_2d(&windows, &cam3d) else {
        return;
    };
    let balance = active_balance(&state);
    let slot = world_to_build_slot(snapshot, player_id, team, world);
    let valid = slot.is_some_and(|(lane, zone, cell)| {
        can_place_building(&balance, snapshot, player_id, team, kind, lane, zone, cell)
    });
    let pos = slot
        .map(|(lane, zone, cell)| cell_to_world(team, lane, zone, cell))
        .unwrap_or(world);
    let color = if valid {
        Color::srgba(0.30, 0.85, 0.46, 0.5)
    } else {
        Color::srgba(0.95, 0.20, 0.18, 0.5)
    };
    let mut res = Res3d {
        meshes: &mut meshes,
        materials: &mut materials,
        assets: &mut world_assets,
    };
    let base = world2_to_3d(pos);
    // Hover pulse (§7.2): the footprint tile breathes under the ghost.
    let pulse = 1.0 + (time.elapsed_secs() * 4.5).sin() * 0.05;
    let bob = (time.elapsed_secs() * 2.5).sin() * 1.5;
    commands.spawn((
        Mesh3d(res.tile_quad()),
        MeshMaterial3d(res.tile_mat(color)),
        Transform::from_translation(base + Vec3::Y * 1.0).with_scale(Vec3::splat(pulse)),
        NotShadowCaster,
        PreviewEntity,
    ));
    commands.spawn((
        Mesh3d(res.stand_quad(CELL + 6.0, CELL + 6.0)),
        MeshMaterial3d(res.tex_mat(
            &building_icon_handle(&building_icons, kind),
            Color::srgba(1.0, 1.0, 1.0, 0.55),
        )),
        Transform::from_translation(base + Vec3::Y * ((CELL + 6.0) * 0.5 + bob)),
        Billboard,
        NotShadowCaster,
        PreviewEntity,
    ));
}

// ---- Camera ----

pub(crate) fn camera_rig_input(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut wheel_events: MessageReader<MouseWheel>,
    windows: Query<&Window, With<PrimaryWindow>>,
    layout: Res<UiLayout>,
    net: Res<ClientNet>,
    mut rig: ResMut<CameraRig>,
) {
    for wheel_event in wheel_events.read() {
        let delta = if wheel_event.unit == MouseScrollUnit::Line {
            wheel_event.y * 0.1
        } else {
            wheel_event.y * 0.01
        };
        rig.zoom = (rig.zoom - delta).clamp(CAM_ZOOM_MIN, CAM_ZOOM_MAX);
    }
    if keys.just_pressed(KeyCode::Equal) || keys.just_pressed(KeyCode::NumpadAdd) {
        rig.zoom = (rig.zoom * 0.88).clamp(CAM_ZOOM_MIN, CAM_ZOOM_MAX);
    }
    if keys.just_pressed(KeyCode::Minus) || keys.just_pressed(KeyCode::NumpadSubtract) {
        rig.zoom = (rig.zoom * 1.12).clamp(CAM_ZOOM_MIN, CAM_ZOOM_MAX);
    }
    if keys.just_pressed(KeyCode::Home) {
        rig.zoom = 1.0;
        rig.target = net.team.map(CameraRig::home_target).unwrap_or(Vec2::ZERO);
    }

    let mut forward = 0.0_f32;
    let mut strafe = 0.0_f32;
    if keys.pressed(KeyCode::ArrowLeft) || keys.pressed(KeyCode::KeyA) {
        strafe -= 1.0;
    }
    if keys.pressed(KeyCode::ArrowRight) || keys.pressed(KeyCode::KeyD) {
        strafe += 1.0;
    }
    if keys.pressed(KeyCode::ArrowUp) || keys.pressed(KeyCode::KeyW) {
        forward += 1.0;
    }
    if keys.pressed(KeyCode::ArrowDown) || keys.pressed(KeyCode::KeyS) {
        forward -= 1.0;
    }
    if let Ok(window) = windows.single() {
        if let Some(cursor) = window.cursor_position() {
            let edge = 18.0;
            if cursor.x <= edge {
                strafe -= 1.0;
            } else if cursor.x >= window.width() - edge {
                strafe += 1.0;
            }
            if cursor.y <= edge {
                forward += 1.0;
            } else if cursor.y >= window.height() - edge
                && cursor_screen_pos(&windows).is_some_and(|screen| screen.y > layout.panel_top_y())
            {
                forward -= 1.0;
            }
        }
    }
    if forward != 0.0 || strafe != 0.0 {
        // Screen-up is toward the enemy (camera forward); screen-right maps
        // to -Y2D because the 3D Z axis flips the v0.1 map vertical.
        let speed = 620.0 * rig.zoom * time.delta_secs();
        rig.target.x += forward * rig.look_sign * speed;
        rig.target.y -= strafe * speed;
        rig.target.x = rig.target.x.clamp(-1520.0, 1520.0);
        rig.target.y = rig.target.y.clamp(-330.0, 330.0);
    }
}

pub(crate) fn apply_camera_rig(
    windows: Query<&Window, With<PrimaryWindow>>,
    net: Res<ClientNet>,
    camera_home: Res<CameraHome>,
    mut rig: ResMut<CameraRig>,
    mut cam: Query<&mut Transform, With<Camera3d>>,
) {
    if let Some(team) = net.team {
        if rig.initialized_for != Some(team) {
            rig.target = camera_home
                .home_override
                .map(|fraction| Vec2::new((fraction - 0.5) * (WORLD_W - 120.0), 0.0))
                .unwrap_or_else(|| CameraRig::home_target(team));
            rig.look_sign = if team == Team::Left { 1.0 } else { -1.0 };
            rig.initialized_for = Some(team);
        }
    }
    let Ok(mut transform) = cam.single_mut() else {
        return;
    };
    let pitch = CAM_PITCH_DEG.to_radians();
    let dist = rig.dist();
    let target3 = world2_to_3d(rig.target);
    let look = Vec3::new(rig.look_sign, 0.0, 0.0);
    transform.translation = target3 - look * (dist * pitch.cos()) + Vec3::Y * (dist * pitch.sin());
    transform.look_at(target3, Vec3::Y);
}

// ---- Picking ----

/// Screen cursor → 2D world point via ray-terrain intersection (plan §3.2).
pub(crate) fn cursor_ground_2d(
    windows: &Query<&Window, With<PrimaryWindow>>,
    cam3d: &Query<(&Camera, &GlobalTransform), With<Camera3d>>,
) -> Option<Vec2> {
    let window = windows.single().ok()?;
    let cursor = window.cursor_position()?;
    let (camera, transform) = cam3d.single().ok()?;
    let ray = camera.viewport_to_world(transform, cursor).ok()?;
    if ray.direction.y.abs() < 1e-4 {
        return None;
    }
    let mut t = (0.0 - ray.origin.y) / ray.direction.y;
    for _ in 0..4 {
        let p = ray.origin + ray.direction * t;
        let h = ground_height(p.x, -p.z);
        t += (h - p.y) / ray.direction.y;
        if !t.is_finite() {
            return None;
        }
    }
    let p = ray.origin + ray.direction * t;
    Some(Vec2::new(p.x, -p.z))
}

// ---- Combat VFX (3D variants; floating text lands in M5) ----

pub(crate) fn spawn_impact_3d(
    res: &mut Res3d,
    commands: &mut Commands,
    target: Vec2,
    attack_type: AttackType,
    castle_hit: bool,
) {
    let color = damage_number_color(attack_type, castle_hit);
    let size = if castle_hit { 40.0 } else { 26.0 };
    spawn_vfx_quad_3d(
        res,
        commands,
        target + Vec2::new(0.0, 6.0),
        Vec2::splat(size),
        color.with_alpha(0.8),
        0.30,
        Vec2::ZERO,
    );
    spawn_vfx_quad_3d(
        res,
        commands,
        target + Vec2::new(0.0, 6.0),
        Vec2::new(size, 5.0),
        Color::srgba(1.0, 0.93, 0.63, 0.75),
        0.22,
        Vec2::ZERO,
    );
}

pub(crate) fn spawn_streak_3d(
    res: &mut Res3d,
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
    // Flat ground quad rotated so its long axis follows the attack direction
    // (2D angle θ maps to a Y rotation of the same value: +X stays +X).
    let angle = delta.y.atan2(delta.x);
    let center = source + delta * 0.55;
    let width = if attack_type == AttackType::Magic {
        5.0
    } else {
        3.0
    };
    let base = world2_to_3d(center + Vec2::new(0.0, 12.0));
    commands.spawn((
        Mesh3d(res.stand_quad(length, width)),
        MeshMaterial3d(res.flat_mat(attack_type_color(attack_type).with_alpha(0.72))),
        Transform::from_translation(base + Vec3::Y * 10.0)
            .with_rotation(Quat::from_rotation_y(angle)),
        NotShadowCaster,
        VfxBillboard,
        CombatVfx {
            lifetime: 0.2,
            max_lifetime: 0.2,
            velocity: Vec2::ZERO,
        },
    ));
}

pub(crate) fn spawn_sparkle_3d(
    res: &mut Res3d,
    commands: &mut Commands,
    pos: Vec2,
    color: Color,
    offset: Vec2,
    velocity: Vec2,
    lifetime: f32,
) {
    spawn_vfx_quad_3d(
        res,
        commands,
        pos + offset,
        Vec2::splat(7.0),
        color.with_alpha(0.85),
        lifetime,
        velocity,
    );
}

pub(crate) fn spawn_structure_impact_3d(res: &mut Res3d, commands: &mut Commands, target: Vec2) {
    spawn_vfx_quad_3d(
        res,
        commands,
        target,
        Vec2::new(58.0, 46.0),
        Color::srgba(1.0, 0.24, 0.08, 0.34),
        0.26,
        Vec2::ZERO,
    );
    spawn_vfx_quad_3d(
        res,
        commands,
        target,
        Vec2::new(44.0, 34.0),
        Color::srgba(1.0, 0.78, 0.24, 0.28),
        0.18,
        Vec2::ZERO,
    );
    for offset in [
        Vec2::new(-13.0, -5.0),
        Vec2::new(15.0, -2.0),
        Vec2::new(-3.0, 10.0),
        Vec2::new(8.0, 6.0),
    ] {
        spawn_vfx_quad_3d(
            res,
            commands,
            target,
            Vec2::new(8.0, 3.0),
            Color::srgba(0.74, 0.56, 0.34, 0.82),
            0.42,
            Vec2::new(offset.x * 0.55, 26.0),
        );
    }
}

pub(crate) fn spawn_vfx_quad_3d(
    res: &mut Res3d,
    commands: &mut Commands,
    pos2d: Vec2,
    size: Vec2,
    color: Color,
    lifetime: f32,
    velocity: Vec2,
) -> Entity {
    let base = world2_to_3d(pos2d);
    commands
        .spawn((
            Mesh3d(res.stand_quad(size.x, size.y)),
            MeshMaterial3d(res.flat_mat(color)),
            Transform::from_translation(base + Vec3::Y * 14.0),
            Billboard,
            NotShadowCaster,
            VfxBillboard,
            CombatVfx {
                lifetime,
                max_lifetime: lifetime,
                velocity,
            },
        ))
        .id()
}

/// 3D replacement for `update_combat_vfx`: lifetime, rise, fade. Fading uses
/// the (short-lived, effectively unique) material instance of each effect.
pub(crate) fn update_combat_vfx_3d(
    mut commands: Commands,
    time: Res<Time>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut query: Query<
        (
            Entity,
            &mut CombatVfx,
            &mut Transform,
            &MeshMaterial3d<StandardMaterial>,
        ),
        With<VfxBillboard>,
    >,
) {
    for (entity, mut vfx, mut transform, material) in &mut query {
        vfx.lifetime -= time.delta_secs();
        if vfx.lifetime <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        let age = 1.0 - (vfx.lifetime / vfx.max_lifetime).clamp(0.0, 1.0);
        transform.translation.x += vfx.velocity.x * time.delta_secs();
        transform.translation.y += vfx.velocity.y * time.delta_secs();
        transform.scale = Vec3::splat(1.0 + age * 0.15);
        let alpha = (vfx.lifetime / vfx.max_lifetime).clamp(0.0, 1.0);
        if let Some(mat) = materials.get_mut(&material.0) {
            mat.base_color = mat.base_color.with_alpha(alpha * 0.85);
        }
    }
}

/// Orient every billboard (unit roots, sprites, vfx) to the 3D camera.
pub(crate) fn orient_billboards(
    cam: Query<&GlobalTransform, With<Camera3d>>,
    mut query: Query<&mut Transform, With<Billboard>>,
) {
    let Ok(cam_transform) = cam.single() else {
        return;
    };
    let cam_pos = cam_transform.translation();
    for mut transform in &mut query {
        let to_cam = cam_pos - transform.translation;
        transform.rotation = Quat::from_rotation_y(to_cam.x.atan2(to_cam.z));
    }
}
