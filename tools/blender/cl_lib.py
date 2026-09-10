"""Castle Lanes low-poly model library (plan-0.2.md §5).

Models are CODE: every unit/building/castle/doodad is assembled here from
box/cylinder/cone primitives with per-part vertex colors. Exported as glTF
(GLB, +Y up) with a "MAIN" vertex-color material and optional "BANNER"
material the engine tints per team at runtime.

Conventions:
- 1 Blender unit = 1 world unit in the game. +Y is up, models face +X
  (the direction they look/march when their team attacks toward +X).
- Flat shading, crisp silhouettes first.
- Poly budgets: unit <= 600 tris, building <= 2000, castle <= 5000,
  doodad <= 300.
"""

import bpy
import math

# ---------------------------------------------------------------- palettes

VANGUARD = {
    "stone": (0.52, 0.56, 0.63, 1),
    "stone_dark": (0.38, 0.41, 0.47, 1),
    "timber": (0.42, 0.29, 0.18, 1),
    "roof": (0.22, 0.32, 0.55, 1),
    "gold": (0.85, 0.68, 0.28, 1),
    "cloth": (0.28, 0.46, 0.85, 1),
    "steel": (0.72, 0.75, 0.80, 1),
    "skin": (0.85, 0.68, 0.55, 1),
    "leather": (0.48, 0.34, 0.22, 1),
    "accent": (0.30, 0.55, 0.95, 1),
    "iron": (0.40, 0.42, 0.46, 1),
}

GROVE = {
    "bark": (0.36, 0.28, 0.18, 1),
    "bark_dark": (0.27, 0.21, 0.14, 1),
    "moss": (0.32, 0.46, 0.22, 1),
    "leaf": (0.42, 0.62, 0.24, 1),
    "leaf_bright": (0.55, 0.74, 0.28, 1),
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
    "obsidian": (0.16, 0.13, 0.15, 1),
    "charcoal": (0.24, 0.21, 0.22, 1),
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
    """Store the part color as a corner color attribute (exports as COLOR_0)."""
    mesh = obj.data
    loops = len(mesh.loops)
    attr = mesh.color_attributes.new(name="col", type="BYTE_COLOR", domain="CORNER")
    flat = []
    for _ in range(loops):
        flat.extend(color)
    attr.data.foreach_set("color", flat)


class Builder:
    """Collects primitive parts, then merges them into one object per
    material slot ('MAIN' with vertex colors, optional 'BANNER')."""

    def __init__(self):
        self.main = []
        self.banner = []

    # -- primitive helpers (all take explicit world transforms) ----------
    def box(self, loc, size, color, rot=(0, 0, 0), banner=False):
        bpy.ops.mesh.primitive_cube_add(size=1, location=loc, rotation=rot)
        obj = bpy.context.active_object
        obj.scale = (size[0] / 2, size[1] / 2, size[2] / 2)
        bpy.ops.object.transform_apply(scale=True)
        _apply_color(obj, color)
        (self.banner if banner else self.main).append(obj)
        return obj

    def cyl(self, loc, radius, depth, color, rot=(0, 0, 0), verts=8, banner=False):
        bpy.ops.mesh.primitive_cylinder_add(
            vertices=verts, radius=radius, depth=depth, location=loc, rotation=rot
        )
        obj = bpy.context.active_object
        _apply_color(obj, color)
        (self.banner if banner else self.main).append(obj)
        return obj

    def cone(self, loc, radius, depth, color, rot=(0, 0, 0), verts=8, banner=False):
        bpy.ops.mesh.primitive_cone_add(
            vertices=verts, radius1=radius, radius2=0, depth=depth,
            location=loc, rotation=rot,
        )
        obj = bpy.context.active_object
        _apply_color(obj, color)
        (self.banner if banner else self.main).append(obj)
        return obj

    def sphere(self, loc, radius, color, banner=False, segments=10, ring_count=8):
        bpy.ops.mesh.primitive_uv_sphere_add(
            segments=segments, ring_count=ring_count, radius=radius, location=loc
        )
        obj = bpy.context.active_object
        bpy.ops.object.shade_smooth()
        _apply_color(obj, color)
        (self.banner if banner else self.main).append(obj)
        return obj

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
        _apply_color(obj, color)
        (self.main).append(obj)
        return obj

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

def unit_base(b, pal, scale=1.0, bulky=0.0, helmet=None, cloak=False):
    """Shared humanoid: legs, torso, head; faces +X. Height ~46*scale."""
    h = scale
    w = 1.0 + bulky * 0.5
    # legs
    b.box((-2.5 * h, -3.5 * w, 9 * h), (4.5 * h, 3.2 * w, 18 * h), pal["leather"])
    b.box((-2.5 * h, 3.5 * w, 9 * h), (4.5 * h, 3.2 * w, 18 * h), pal["leather"])
    # torso
    b.box((1 * h, 0, 22 * h), (11 * h, 11 * w, 16 * h), pal["cloth"])
    # belt
    b.box((1 * h, 0, 15 * h), (11.5 * h, 11.5 * w, 3 * h), pal["leather"])
    # head
    b.box((2 * h, 0, 34 * h), (8 * h, 7.5 * w, 8 * h), pal["skin"])
    if helmet == "pot":
        b.box((2 * h, 0, 37.5 * h), (10 * h, 10 * w, 4 * h), pal["steel"])
    elif helmet == "hood":
        b.box((1.5 * h, 0, 35 * h), (9.5 * h, 9 * w, 9 * h), pal["cloth"])
    elif helmet == "helm":
        b.box((2 * h, 0, 36.5 * h), (9 * h, 8.5 * w, 6 * h), pal["steel"])
        b.cone((2 * h, 0, 43 * h), 2 * h, 7 * h, pal["accent"])
    if cloak:
        b.box((-3.5 * h, 0, 22 * h), (2.5 * h, 11 * w, 20 * h), pal["accent"])


def weapon_sword(b, pal, h=1.0):
    b.box((9 * h, -6 * h, 24 * h), (9 * h, 2.2 * h, 2.6 * h), pal["steel"])
    b.box((5.5 * h, -6 * h, 24 * h), (2.5 * h, 3.4 * h, 3.4 * h), pal["gold"])


def weapon_spear(b, pal, h=1.0):
    shaft = pal["leather"]
    b.cyl((3 * h, -6.5 * h, 26 * h), 1.1 * h, 44 * h, shaft, rot=(0, math.pi / 2, 0))
    b.cone((25 * h, -6.5 * h, 26 * h), 2.2 * h, 8 * h, pal["steel"], rot=(0, math.pi / 2, 0))


def weapon_bow(b, pal, h=1.0):
    b.cyl((8.5 * h, -6 * h, 26 * h), 8 * h, 1.4 * h, pal["leather"], rot=(math.pi / 2, 0, 0), verts=6)


def weapon_shield(b, pal, h=1.0):
    b.box((8 * h, 6.5 * h, 24 * h), (2.2 * h, 9 * h, 15 * h), pal["steel"])
    b.box((9.4 * h, 6.5 * h, 24 * h), (1.2 * h, 4 * h, 4 * h), pal["gold"])


def weapon_staff(b, pal, h=1.0, orb=None):
    b.cyl((6 * h, -6.5 * h, 26 * h), 1.1 * h, 40 * h, pal["leather"], rot=(0, math.pi / 2, 0))
    b.sphere((6 * h, -6.5 * h, 47 * h), 3.2 * h, orb or pal["accent"])


def weapon_lance(b, pal, h=1.0, tip_color=None):
    b.cyl((4 * h, -7 * h, 27 * h), 1.3 * h, 52 * h, pal["leather"], rot=(0, math.pi / 2, 0))
    b.cone((30 * h, -7 * h, 27 * h), 2.6 * h, 9 * h, tip_color or pal["steel"], rot=(0, math.pi / 2, 0))


def banner(b, pal, h=1.0, tall=30, banner_color=None):
    """Team-color banner: 'BANNER' material slot, engine tints per side."""
    b.cyl((-4 * h, 0, 24 * h + tall * h / 2), 1.0 * h, tall * h, pal["leather"])
    b.box((-1.5 * h, 0, 24 * h + tall * h * 0.72), (9 * h, 0.6 * h, 12 * h), (1, 1, 1, 1), banner=True)


# ---------------------------------------------------------------- terrain doodad colors

def leaf_canopy(b, pal, loc, r, color=None):
    b.cone((loc[0], loc[1], loc[2] + r * 0.9), r, r * 2.1, color or pal["leaf"], verts=7)


def debug_colors(obj):
    mesh = obj.data
    print("ATTRS:", [a.name for a in mesh.color_attributes])
    if mesh.color_attributes:
        a = mesh.color_attributes[0]
        vals = [a.data[i].color for i in range(0, min(4, len(a.data)))]
        print("SAMPLE:", vals)


def debug_hook(b, name):
    return b
