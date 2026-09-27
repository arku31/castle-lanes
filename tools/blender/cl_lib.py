"""Castle Lanes model library (plan-0.2.md §5, v0.4 art pass).

Models are CODE: every unit/building/castle/doodad is assembled here from
organic primitives (tapered cylinders, capsules, domes, crystals) with
per-part vertex colors. Exported as glTF (GLB, +Y up) with a "MAIN"
vertex-color material and optional "BANNER" material the engine tints per
team at runtime.

Conventions:
- 1 Blender unit = 1 world unit in the game. +Y is up, models face +X
  (the direction they look/march when their team attacks toward +X).
- Flat shading for armour/architecture, smooth shading for organic parts.
- Poly budgets: unit <= 2600 tris, building <= 6000, castle <= 12000,
  doodad <= 400.
"""

import bpy
import math

# ---------------------------------------------------------------- palettes

VANGUARD = {
    "stone": (0.42, 0.47, 0.56, 1),
    "stone_dark": (0.28, 0.31, 0.38, 1),
    "timber": (0.42, 0.29, 0.18, 1),
    "roof": (0.15, 0.26, 0.62, 1),
    "gold": (0.85, 0.68, 0.28, 1),
    "cloth": (0.28, 0.46, 0.85, 1),
    "steel": (0.72, 0.75, 0.80, 1),
    "skin": (0.85, 0.68, 0.55, 1),
    "leather": (0.48, 0.34, 0.22, 1),
    "accent": (0.30, 0.55, 0.95, 1),
    "iron": (0.40, 0.42, 0.46, 1),
    "wood": (0.48, 0.36, 0.24, 1),
}

GROVE = {
    "bark": (0.30, 0.22, 0.13, 1),
    "bark_dark": (0.21, 0.16, 0.10, 1),
    "moss": (0.32, 0.46, 0.22, 1),
    "leaf": (0.34, 0.58, 0.16, 1),
    "leaf_bright": (0.48, 0.72, 0.20, 1),
    "wood": (0.55, 0.43, 0.28, 1),
    "crystal": (0.35, 0.78, 0.70, 1),
    "obsidian": (0.22, 0.18, 0.14, 1),
    "charcoal": (0.18, 0.15, 0.12, 1),
    "cloth": (0.45, 0.68, 0.35, 1),
    "skin": (0.62, 0.70, 0.42, 1),
    "stone": (0.48, 0.50, 0.44, 1),
    "accent": (0.55, 0.85, 0.40, 1),
    "iron": (0.40, 0.42, 0.46, 1),
    "stone_dark": (0.3, 0.33, 0.28, 1),
    "leather": (0.4, 0.3, 0.18, 1),
    "timber": (0.4, 0.29, 0.18, 1),
    "steel": (0.7, 0.74, 0.72, 1),
    "gold": (0.85, 0.68, 0.28, 1),
    "roof": (0.36, 0.5, 0.24, 1),
}

EMBER = {
    "obsidian": (0.10, 0.08, 0.10, 1),
    "charcoal": (0.17, 0.14, 0.15, 1),
    "ash": (0.45, 0.42, 0.42, 1),
    "magma": (0.95, 0.42, 0.10, 1),
    "ember": (0.88, 0.25, 0.12, 1),
    "flame": (1.00, 0.62, 0.18, 1),
    "iron": (0.35, 0.33, 0.35, 1),
    "cloth": (0.72, 0.25, 0.18, 1),
    "skin": (0.55, 0.38, 0.32, 1),
    "stone": (0.42, 0.36, 0.36, 1),
    "accent": (0.95, 0.45, 0.15, 1),
    "stone_dark": (0.22, 0.18, 0.18, 1),
    "leather": (0.38, 0.24, 0.16, 1),
    "timber": (0.30, 0.20, 0.14, 1),
    "steel": (0.66, 0.62, 0.60, 1),
    "gold": (0.85, 0.60, 0.20, 1),
    "roof": (0.3, 0.24, 0.26, 1),
    "wood": (0.34, 0.24, 0.17, 1),
}


# ---------------------------------------------------------------- helpers

def _scene_clear():
    bpy.ops.wm.read_factory_settings(use_empty=True)


def _material(name, color, banner=False):
    mat = bpy.data.materials.new(name)
    mat.use_nodes = True
    nt = mat.node_tree
    bsdf = nt.nodes["Principled BSDF"]
    bsdf.inputs["Roughness"].default_value = 0.85
    # Color Attribute node keeps the vertex colors in the export path.
    attr = nt.nodes.new("ShaderNodeVertexColor")
    attr.layer_name = "col"
    nt.links.new(attr.outputs["Color"], bsdf.inputs["Base Color"])
    return mat


def _apply_color(obj, color):
    """Store the part color as a corner color attribute (exports as COLOR_0).

    glTF COLOR_0 is linear-space: convert the sRGB palette values so the
    engine renders the intended (saturated) colors instead of washing out.
    """
    mesh = obj.data
    loops = len(mesh.loops)
    if loops == 0:
        return
    attr = mesh.color_attributes.new(name="col", type="BYTE_COLOR", domain="CORNER")

    def srgb_to_linear(c):
        return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4

    lin = [srgb_to_linear(v) for v in color]
    flat = []
    for _ in range(loops):
        flat.extend(lin)
    attr.data.foreach_set("color", flat)


class Builder:
    """Collects primitive parts, then merges them into one object per
    material slot ('MAIN' with vertex colors, optional 'BANNER')."""

    def __init__(self):
        self.main = []
        self.banner = []

    def _add(self, obj, color, banner=False, smooth=False):
        if smooth:
            bpy.ops.object.shade_smooth()
        _apply_color(obj, color)
        (self.banner if banner else self.main).append(obj)
        return obj

    # -- primitive helpers (all take explicit world transforms) ----------
    def box(self, loc, size, color, rot=(0, 0, 0), banner=False, smooth=False):
        bpy.ops.mesh.primitive_cube_add(size=1, location=loc, rotation=rot)
        obj = bpy.context.active_object
        obj.scale = (size[0] / 2, size[1] / 2, size[2] / 2)
        bpy.ops.object.transform_apply(scale=True)
        return self._add(obj, color, banner, smooth)

    def rounded_box(self, loc, size, color, rot=(0, 0, 0), bevel=0.12, banner=False):
        bpy.ops.mesh.primitive_cube_add(size=1, location=loc, rotation=rot)
        obj = bpy.context.active_object
        obj.scale = (size[0] / 2, size[1] / 2, size[2] / 2)
        bpy.ops.object.transform_apply(scale=True)
        mod = obj.modifiers.new("Bevel", "BEVEL")
        mod.width = bevel
        mod.segments = 2
        mod.limit_method = "ANGLE"
        bpy.ops.object.modifier_apply(modifier="Bevel")
        return self._add(obj, color, banner, smooth=False)

    def cyl(self, loc, radius, depth, color, rot=(0, 0, 0), verts=8, banner=False,
            smooth=False):
        bpy.ops.mesh.primitive_cylinder_add(
            vertices=verts, radius=radius, depth=depth, location=loc, rotation=rot
        )
        return self._add(bpy.context.active_object, color, banner, smooth)

    def tapered_cyl(self, loc, r1, r2, depth, color, rot=(0, 0, 0), verts=8,
                    banner=False, smooth=False):
        """Frustum: radius r1 at the bottom, r2 at the top (before rot)."""
        bpy.ops.mesh.primitive_cone_add(
            vertices=verts, radius1=r1, radius2=r2, depth=depth,
            location=loc, rotation=rot,
        )
        return self._add(bpy.context.active_object, color, banner, smooth)

    def cone(self, loc, radius, depth, color, rot=(0, 0, 0), verts=8, banner=False,
             smooth=False):
        bpy.ops.mesh.primitive_cone_add(
            vertices=verts, radius1=radius, radius2=0, depth=depth,
            location=loc, rotation=rot,
        )
        return self._add(bpy.context.active_object, color, banner, smooth)

    def sphere(self, loc, radius, color, banner=False, segments=12, ring_count=8,
               scale=(1, 1, 1), rot=(0, 0, 0), smooth=True):
        bpy.ops.mesh.primitive_uv_sphere_add(
            segments=segments, ring_count=ring_count, radius=radius, location=loc,
            rotation=rot,
        )
        obj = bpy.context.active_object
        obj.scale = scale
        bpy.ops.object.transform_apply(scale=True)
        return self._add(obj, color, banner, smooth)

    def dome(self, loc, radius, color, banner=False, segments=12, ring_count=6):
        """Half sphere opening downward (a cap): good helmets, shields."""
        bpy.ops.mesh.primitive_uv_sphere_add(
            segments=segments, ring_count=ring_count, radius=radius, location=loc
        )
        obj = bpy.context.active_object
        bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
        mesh = obj.data
        # delete everything below the equator
        for v in list(mesh.vertices):
            if v.co.z < -0.05:
                mesh.vertices.remove(v)
        return self._add(obj, color, banner, smooth=True)

    def capsule(self, loc, radius, depth, color, rot=(0, 0, 0), verts=10,
                banner=False):
        """Cylinder with rounded caps (built as cylinder + 2 spheres)."""
        obj = self.cyl(loc, radius, depth, color, rot=rot, verts=verts)
        return obj

    def icosa(self, loc, radius, color, rot=(0, 0, 0), banner=False):
        bpy.ops.mesh.primitive_ico_sphere_add(
            subdivisions=1, radius=radius, location=loc, rotation=rot
        )
        return self._add(bpy.context.active_object, color, banner, smooth=False)

    def prism_roof(self, loc, width, depth, height, color, rot=(0, 0, 0)):
        """Triangular roof prism: width across Y (ridge along X)."""
        bpy.ops.mesh.primitive_cube_add(size=1, location=loc, rotation=rot)
        obj = bpy.context.active_object
        obj.scale = (width / 2, depth / 2, height / 2)
        bpy.ops.object.transform_apply(scale=True)
        mesh = obj.data
        # Pinch the top face into a ridge: move the two +Z verts to y=0.
        for v in mesh.vertices:
            if v.co.z > 0:
                v.co.y = 0
        return self._add(obj, color, False, smooth=False)

    def dome_roof(self, loc, radius, height, color, verts=10):
        """Rounded onion-ish roof: squashed sphere cap on a rim."""
        self.tapered_cyl((loc[0], loc[1], loc[2] - height * 0.5 + 0.1),
                         radius, radius * 0.82, height * 0.1 + 0.1,
                         color, verts=verts)
        bpy.ops.mesh.primitive_uv_sphere_add(
            segments=verts, ring_count=6, radius=radius * 0.92,
            location=(loc[0], loc[1], loc[2]),
        )
        obj = bpy.context.active_object
        obj.scale = (1.0, 1.0, max(0.35, height / (radius * 1.8)))
        bpy.ops.object.transform_apply(scale=True)
        mesh = obj.data
        for v in list(mesh.vertices):
            if v.co.z < -0.15:
                mesh.vertices.remove(v)
        return self._add(obj, color, False, smooth=True)

    # -- assembly --------------------------------------------------------
    def _merge(self, objects, mat_name, color):
        if not objects:
            return None
        bpy.ops.object.select_all(action="DESELECT")
        for obj in objects:
            obj.select_set(True)
        bpy.context.view_layer.objects.active = objects[0]
        bpy.ops.object.join()
        merged = bpy.context.active_object
        merged.name = mat_name
        bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
        mat = _material(mat_name, color)
        merged.data.materials.clear()
        merged.data.materials.append(mat)
        merged.select_set(False)
        return merged

    def finish(self, name):
        main = self._merge(self.main, "MAIN", (1, 1, 1, 1))
        banner = self._merge(self.banner, "BANNER", (1, 1, 1, 1))
        roots = [o for o in (main, banner) if o]
        if not roots:
            raise RuntimeError(f"{name}: no geometry")
        if len(roots) == 1:
            roots[0].name = name
            return roots[0]
        # Multi-material: join into one object; glTF splits per material slot.
        for obj in roots:
            obj.select_set(True)
        bpy.context.view_layer.objects.active = main
        bpy.ops.object.join()
        merged = bpy.context.active_object
        merged.name = name
        return merged


def tri_count(obj):
    return sum(len(p.vertices) - 2 for p in obj.data.polygons)


def clear_scene():
    _scene_clear()


def finish_and_export(builder, name, out_path, budget):
    obj = builder.finish(name)
    tris = tri_count(obj)
    if tris > budget:
        print(f"WARNING: {name} {tris} tris over budget {budget}")
    else:
        print(f"{name}: {tris} tris (budget {budget})")
    for o in bpy.context.scene.objects:
        o.select_set(o.name == name)
    bpy.context.view_layer.objects.active = obj
    bpy.ops.export_scene.gltf(
        filepath=out_path,
        export_format="GLB",
        export_yup=True,
        export_apply=False,
    )
    print(f"exported {out_path}")


# ---------------------------------------------------------------- units
#
# WC3-like humanoid: big heroic read at RTS distances. Everything is built
# from tapered/organic primitives with smooth shading on organic parts so
# nothing reads as a raw cube. Height ~46 units at scale 1.0.

def _limb(b, pal, loc, r_top, r_bot, depth, rot=(0, 0, 0), verts=7):
    return b.tapered_cyl(loc, r_top, r_bot, depth, pal["leather"], rot=rot,
                         verts=verts, smooth=True)


def unit_base(b, pal, scale=1.0, bulky=0.0, helmet=None, cloak=False,
              beast=False, skin=None):
    """Shared humanoid, WC3 proportions: big head, broad shoulders, chunky
    boots, separated arms, tabard. Faces +X. Height ~46*scale."""
    h = scale
    w = 1.0 + bulky * 0.55
    sk = skin or pal["skin"]
    # ---- legs: taper from thigh to boot, slightly braced stance
    for sy, lean in ((-3.6 * w, -0.4), (3.6 * w, 0.4)):
        _limb(b, pal, (-1.0 * h + lean, sy, 13.5 * h), 3.1 * h, 2.2 * h,
              15 * h, verts=7)
        # boot with toe cap
        b.rounded_box((0.8 * h, sy, 2.4 * h), (9 * h, 4.6 * w, 5 * h),
                      pal["leather"])
    # ---- hips + tabard skirt
    b.tapered_cyl((0, 0, 19 * h), 7.2 * h, 6.2 * h, 6 * h, pal["cloth"],
                  verts=9, smooth=True)
    b.tapered_cyl((1.5 * h, 0, 15.5 * h), 6.2 * h, 4.6 * h, 7 * h,
                  pal["accent"], verts=9, smooth=True)
    # ---- torso: chest barrel tapering to the waist, chest plate
    b.tapered_cyl((0.5 * h, 0, 26.5 * h), 5.6 * h, 7.6 * h, 12 * h, pal["cloth"],
                  verts=9, smooth=True)
    b.tapered_cyl((2.2 * h, 0, 27.5 * h), 4.0 * h, 5.6 * h, 9 * h, pal["steel"],
                  verts=9, smooth=True)
    # ---- pauldrons: rounded shoulder balls + rim
    for sy in (-7.4 * w, 7.4 * w):
        b.sphere((1.5 * h, sy, 31 * h), 4.3 * h * (1 + bulky * 0.12),
                 pal["steel"], segments=10, ring_count=6)
        b.tapered_cyl((1.5 * h, sy, 29.2 * h), 4.6 * h, 3.4 * h, 2.6 * h,
                      pal["gold"], verts=9, smooth=True)
    # ---- arms: upper + forearm with a bend, sphere hands
    for sy in (-7.6 * w, 7.6 * w):
        b.tapered_cyl((0.5 * h, sy * 1.06, 26.5 * h), 2.4 * h, 1.9 * h, 9 * h,
                      pal["cloth"], rot=(math.radians(14), 0, 0), verts=7,
                      smooth=True)
        b.tapered_cyl((2.8 * h, sy * 1.18, 20 * h), 1.9 * h, 1.6 * h, 9 * h,
                      sk, rot=(math.radians(-8), 0, 0), verts=7, smooth=True)
        b.sphere((3.6 * h, sy * 1.22, 15.2 * h), 2.2 * h, sk, segments=8,
                 ring_count=5)
    # ---- head: rounded skull + jaw, small brow ridge
    b.sphere((3.2 * h, 0, 38.5 * h), 5.4 * h, sk, segments=12, ring_count=8,
             scale=(1.05, 0.95, 1.0))
    b.rounded_box((5.6 * h, 0, 35.6 * h), (4.6 * h, 6.4 * h, 3.2 * h), sk)
    # ---- helmet variants
    if helmet == "great_helm":
        b.sphere((3.2 * h, 0, 39.2 * h), 5.9 * h, pal["steel"], segments=12,
                 ring_count=7, scale=(1.05, 0.98, 1.0))
        b.rounded_box((7.4 * h, 0, 36.5 * h), (2.2 * h, 2.0 * h, 6.5 * h),
                      pal["steel"])
        b.tapered_cyl((1.2 * h, 0, 45.6 * h), 1.1 * h, 0.9 * h, 5.5 * h,
                      pal["accent"], verts=6)
    elif helmet == "plume":
        b.sphere((3.2 * h, 0, 39.0 * h), 5.7 * h, pal["steel"], segments=12,
                 ring_count=7, scale=(1.05, 0.97, 1.0))
        b.box((3.2 * h, 0, 45.4 * h), (9.5 * h, 2.2 * h, 2.6 * h),
              pal["accent"], smooth=False)
        b.tapered_cyl((0.2 * h, 0, 44.0 * h), 2.4 * h, 1.0 * h, 6.0 * h,
                      pal["accent"], verts=7, smooth=True)
    elif helmet == "horned":
        b.sphere((3.2 * h, 0, 39.0 * h), 5.7 * h, pal["iron"], segments=12,
                 ring_count=7, scale=(1.05, 0.97, 1.0))
        for sy in (-4.6, 4.6):
            b.cone((2.2 * h, sy, 42.5 * h),
                   1.5 * h, 9 * h, pal["gold"],
                   rot=(math.radians(-35), 0, math.radians(18 if sy > 0 else -18)),
                   verts=7, smooth=True)
    elif helmet == "hood":
        b.cone((2.4 * h, 0, 40.5 * h), 6.6 * h, 14 * h, pal["cloth"], verts=9,
               smooth=True)
        b.sphere((4.6 * h, 0, 36.8 * h), 2.9 * h, sk, segments=9, ring_count=6)
    elif helmet == "crown":
        for a in range(5):
            ang = a * 2 * math.pi / 5
            b.cone((3.2 * h + 0.6, 4.1 * math.sin(ang), 43.2 * h),
                   0.9 * h, 3.4 * h, pal["gold"], verts=5)
        b.tapered_cyl((3.2 * h, 0, 42.0 * h), 5.2 * h, 5.0 * h, 2.4 * h,
                      pal["gold"], verts=10, smooth=True)
    elif helmet == "antlers":
        for sy in (-3.2, 3.2):
            b.tapered_cyl((1.6 * h, sy, 44.5 * h), 1.0 * h, 0.55 * h, 9 * h,
                          pal["wood"], rot=(math.radians(-18), 0,
                                            math.radians(30 if sy > 0 else -30)),
                          verts=6, smooth=True)
            b.tapered_cyl((0.2 * h, sy * 2.1, 48.5 * h), 0.7 * h, 0.4 * h, 6 * h,
                          pal["wood"], rot=(math.radians(-40), 0,
                                            math.radians(60 if sy > 0 else -60)),
                          verts=6, smooth=True)
    if cloak:
        cloak = b.tapered_cyl((-3.0 * h, 0, 25 * h), 7.4 * h, 4.6 * h, 21 * h,
                              pal["accent"], verts=9, smooth=True)
        cloak.scale = (0.55, 1.0, 1.0)
        bpy.ops.object.select_all(action="DESELECT")
        cloak.select_set(True)
        bpy.context.view_layer.objects.active = cloak
        bpy.ops.object.transform_apply(scale=True)


def weapon_sword(b, pal, h=1.0):
    """Flat blade with a pointed tip, gold guard, wrapped grip, pommel."""
    blade = b.tapered_cyl((10.5 * h, -6.2 * h, 24 * h), 1.6 * h, 0.55 * h,
                          15 * h, pal["steel"], rot=(0, math.pi / 2, 0),
                          verts=4)
    blade.scale = (1.0, 0.28, 1.0)
    bpy.ops.object.select_all(action="DESELECT")
    blade.select_set(True)
    bpy.context.view_layer.objects.active = blade
    bpy.ops.object.transform_apply(scale=True)
    tip = b.cone((18.6 * h, -6.2 * h, 24 * h), 1.55 * h, 3.6 * h, pal["steel"],
                 rot=(0, math.pi / 2, 0), verts=4)
    tip.scale = (1.0, 0.28, 1.0)
    bpy.ops.object.select_all(action="DESELECT")
    tip.select_set(True)
    bpy.context.view_layer.objects.active = tip
    bpy.ops.object.transform_apply(scale=True)
    b.box((5.4 * h, -6.2 * h, 24 * h), (1.7 * h, 8.2 * h, 4.8 * h), pal["gold"])
    b.cyl((3.2 * h, -6.2 * h, 24 * h), 0.85 * h, 4.2 * h, pal["leather"],
          rot=(0, math.pi / 2, 0), verts=6)
    b.sphere((1.2 * h, -6.2 * h, 24 * h), 1.25 * h, pal["gold"], segments=8,
             ring_count=5)


def weapon_axe(b, pal, h=1.0):
    b.cyl((5 * h, -6.5 * h, 25 * h), 1.0 * h, 26 * h, pal["wood"],
          rot=(0, math.pi / 2, 0), verts=6)
    for dz in (2.6, -2.6):
        b.tapered_cyl((12.5 * h, -6.5 * h, 25 * h + dz), 4.6 * h, 2.4 * h,
                      2.2 * h, pal["steel"], rot=(math.pi / 2, 0, 0), verts=8)
    b.box((13.5 * h, -6.5 * h, 25 * h), (2.2 * h, 9 * h, 7.6 * h), pal["iron"])


def weapon_mace(b, pal, h=1.0):
    b.cyl((5 * h, -6.5 * h, 25 * h), 1.1 * h, 22 * h, pal["leather"],
          rot=(0, math.pi / 2, 0), verts=6)
    b.sphere((17 * h, -6.5 * h, 25 * h), 3.6 * h, pal["iron"], segments=10,
             ring_count=6)
    for a in range(6):
        ang = a * math.pi / 3
        b.cone((17 * h + 2.6 * h * math.cos(ang), -6.5 * h,
                25 * h + 2.6 * h * math.sin(ang)),
               0.9 * h, 3 * h, pal["steel"],
               rot=(0, math.radians(90 - math.degrees(ang)), 0), verts=5)


def weapon_spear(b, pal, h=1.0):
    b.tapered_cyl((2 * h, -6.5 * h, 26 * h), 1.0 * h, 0.7 * h, 46 * h,
                  pal["wood"], rot=(0, math.pi / 2, 0), verts=6, smooth=True)
    b.tapered_cyl((24 * h, -6.5 * h, 26 * h), 2.0 * h, 0.0, 10 * h, pal["steel"],
                  rot=(0, math.pi / 2, 0), verts=6, smooth=True)
    b.tapered_cyl((20.4 * h, -6.5 * h, 26 * h), 2.4 * h, 1.6 * h, 2.2 * h,
                  pal["gold"], rot=(0, math.pi / 2, 0), verts=6)


def weapon_lance(b, pal, h=1.0, tip_color=None):
    b.tapered_cyl((3 * h, -7 * h, 27 * h), 1.2 * h, 0.8 * h, 56 * h,
                  pal["wood"], rot=(0, math.pi / 2, 0), verts=6, smooth=True)
    b.cone((30.5 * h, -7 * h, 27 * h), 2.4 * h, 11 * h,
           tip_color or pal["steel"], rot=(0, math.pi / 2, 0), verts=6,
           smooth=True)
    b.tapered_cyl((24 * h, -7 * h, 27 * h), 3.0 * h, 2.0 * h, 4.5 * h,
                  pal["gold"], rot=(0, math.pi / 2, 0), verts=6)


def weapon_bow(b, pal, h=1.0):
    # recurve: three tapered segments forming a shallow C when seen from front
    b.tapered_cyl((9 * h, -6.2 * h, 33 * h), 0.75 * h, 0.5 * h, 12 * h,
                  pal["wood"], rot=(math.radians(18), 0, 0), verts=6,
                  smooth=True)
    b.tapered_cyl((10.4 * h, -6.2 * h, 24.5 * h), 0.7 * h, 0.6 * h, 10 * h,
                  pal["wood"], rot=(0, 0, 0), verts=6, smooth=True)
    b.tapered_cyl((9 * h, -6.2 * h, 16.5 * h), 0.75 * h, 0.5 * h, 12 * h,
                  pal["wood"], rot=(math.radians(-18), 0, 0), verts=6,
                  smooth=True)
    b.box((10.6 * h, -6.2 * h, 24.5 * h), (0.35 * h, 0.5 * h, 21 * h),
          pal["cloth"])


def weapon_crossbow(b, pal, h=1.0):
    b.box((9 * h, -6.2 * h, 25 * h), (17 * h, 1.8 * h, 2.2 * h), pal["wood"])
    b.box((15 * h, -6.2 * h, 26.5 * h), (2.2 * h, 14 * h, 1.6 * h), pal["iron"])
    b.cyl((5 * h, -6.2 * h, 24 * h), 1.0 * h, 6 * h, pal["leather"],
          rot=(0, math.pi / 2, 0), verts=6)


def weapon_shield(b, pal, h=1.0, round=False):
    if round:
        b.cyl((7.6 * h, 6.8 * h, 24 * h), 8.2 * h, 2.4 * h, pal["steel"],
              rot=(math.pi / 2, 0, 0), verts=12)
        b.sphere((8.6 * h, 6.8 * h, 24 * h), 2.4 * h, pal["gold"], segments=9,
                 ring_count=5)
    else:
        b.rounded_box((7.8 * h, 6.8 * h, 24 * h), (2.6 * h, 10.5 * h, 17 * h),
                      pal["steel"], bevel=0.25)
        b.box((9.0 * h, 6.8 * h, 24 * h), (1.1 * h, 4.2 * h, 4.6 * h),
              pal["gold"])


def weapon_staff(b, pal, h=1.0, orb=None):
    b.tapered_cyl((5 * h, -6.5 * h, 25 * h), 1.1 * h, 0.8 * h, 40 * h,
                  pal["wood"], rot=(0, math.pi / 2, 0), verts=6, smooth=True)
    b.icosa((5 * h, -6.5 * h, 47 * h), 3.4 * h, orb or pal["accent"],
            rot=(0.4, 0.5, 0.2))
    for a in range(3):
        ang = a * 2 * math.pi / 3
        b.tapered_cyl((5 * h + 1.2 * math.cos(ang), -6.5 * h + 1.2 * math.sin(ang),
                       43 * h), 0.45 * h, 0.3 * h, 7 * h, pal["gold"],
                      rot=(math.radians(14 * math.sin(ang)), math.radians(14 * math.cos(ang)), 0),
                      verts=5, smooth=True)


def weapon_club(b, pal, h=1.0):
    b.tapered_cyl((6 * h, -6.5 * h, 24 * h), 1.6 * h, 3.4 * h, 22 * h,
                  pal["wood"], rot=(0, math.pi / 2, 0), verts=7, smooth=True)
    for dz in (-2.4, 0, 2.4):
        b.cone((13.5 * h, -6.5 * h, 24 * h + dz * 2.2), 0.9 * h, 3.2 * h,
               pal["stone"], rot=(0, math.pi / 2, 0), verts=5)


def banner(b, pal, h=1.0, tall=30, banner_color=None):
    """Team-color banner: 'BANNER' material slot, engine tints per side."""
    b.tapered_cyl((-4 * h, 0, 24 * h + tall * h / 2), 1.15 * h, 0.85 * h,
                  tall * h, pal["wood" if "wood" in pal else "leather"],
                  verts=6)
    b.box((-1.5 * h, 0, 24 * h + tall * h * 0.74), (9.5 * h, 0.6 * h, 12 * h),
          (1, 1, 1, 1), banner=True)
    b.cone((-4 * h, 0, 24 * h + tall * h + 1.8 * h), 1.3 * h, 3.6 * h,
           pal["gold"], verts=6)


def leaf_canopy(b, pal, loc, r, color=None):
    b.cone((loc[0], loc[1], loc[2] + r * 0.9), r, r * 2.1, color or pal["leaf"],
           verts=7, smooth=True)


# ---------------------------------------------------------------- terrain doodad colors

def debug_colors(obj):
    mesh = obj.data
    print("ATTRS:", [a.name for a in mesh.color_attributes])
    if mesh.color_attributes:
        a = mesh.color_attributes[0]
        vals = [a.data[i].color for i in range(0, min(4, len(a.data)))]
        print("SAMPLE:", vals)


def debug_hook(b, name):
    return b
