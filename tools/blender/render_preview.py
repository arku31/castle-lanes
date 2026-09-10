"""Renders a turntable preview of a built model with the shared lighting rig
(plan-0.2.md §5: one rig = visual consistency across all assets).

    blender -b -P tools/blender/build_asset.py -- --kind vanguard_castle
    blender -b -P tools/blender/render_preview.py -- assets/models/vanguard/vanguard_castle.glb out.png
"""

import bpy
import sys
import os
import math


def main():
    argv = sys.argv[sys.argv.index("--") + 1:]
    glb, out = argv[0], argv[1]

    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=glb)

    # Shared rig: warm key sun from viewer-left, cool fill, neutral world.
    world = bpy.data.worlds.new("World")
    bpy.context.scene.world = world
    world.use_nodes = True
    world.node_tree.nodes["Background"].inputs[0].default_value = (0.45, 0.55, 0.70, 1)
    world.node_tree.nodes["Background"].inputs[1].default_value = 0.35

    sun = bpy.data.lights.new("Sun", type="SUN")
    sun.energy = 4.0
    sun.color = (1.0, 0.94, 0.82)
    sun_o = bpy.data.objects.new("Sun", sun)
    bpy.context.scene.collection.objects.link(sun_o)
    sun_o.rotation_euler = (0.9, 0.0, 0.7854)

    fill = bpy.data.lights.new("Fill", type="SUN")
    fill.energy = 0.8
    fill.color = (0.7, 0.8, 1.0)
    fill_o = bpy.data.objects.new("Fill", fill)
    bpy.context.scene.collection.objects.link(fill_o)
    fill_o.rotation_euler = (1.4, 0.0, -1.2)

    # Camera: WC3-style 3/4 view framing the whole model.
    import mathutils
    mins = [1e9] * 3
    maxs = [-1e9] * 3
    for o in bpy.context.scene.objects:
        if o.type == "MESH":
            for c in o.bound_box:
                w = o.matrix_world @ mathutils.Vector(c)
                for i in range(3):
                    mins[i] = min(mins[i], w[i])
                    maxs[i] = max(maxs[i], w[i])
    center = mathutils.Vector(((mins[0] + maxs[0]) / 2, (mins[1] + maxs[1]) / 2, (mins[2] + maxs[2]) / 2))
    radius = max(maxs[i] - mins[i] for i in range(3)) * 0.5 + 1.0

    cam = bpy.data.cameras.new("Cam")
    cam.lens = 55
    cam_o = bpy.data.objects.new("Cam", cam)
    bpy.context.scene.collection.objects.link(cam_o)
    import math as _m
    direction = mathutils.Vector((-1.0, 1.5, 0.8)).normalized()
    distance = radius / _m.tan(cam.angle_y * 0.5) * 1.15 + radius
    cam_o.location = center + direction * distance
    look = center - cam_o.location
    cam_o.rotation_euler = look.to_track_quat("-Z", "Y").to_euler()
    bpy.context.scene.camera = cam_o

    # Ground plane for contact reading.
    bpy.ops.mesh.primitive_plane_add(size=radius * 12, location=(center.x, center.y, mins[2] - 0.01))

    scene = bpy.context.scene
    scene.render.resolution_x = 800
    scene.render.resolution_y = 600
    scene.view_settings.view_transform = "Standard"
    scene.render.engine = "BLENDER_EEVEE_NEXT"
    scene.render.filepath = out
    bpy.ops.render.render(write_still=True)
    print(f"preview written to {out}")


main()
