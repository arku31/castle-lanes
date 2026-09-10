"""Builds one Castle Lanes model and exports it as GLB (plan-0.2.md §5).

Usage:
    blender -b -P tools/blender/build_asset.py -- --kind vanguard_castle
    blender -b -P tools/blender/build_asset.py -- --all

Every model is assembled from the shared part library (cl_lib.py) so the
faction sets stay consistent. Poly budgets are enforced (printed warnings).
"""

import bpy
import sys
import os
import math

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import cl_lib as cl  # noqa: E402

V = cl.VANGUARD
G = cl.GROVE
E = cl.EMBER

REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))


def out_path(faction, name):
    d = os.path.join(REPO, "assets", "models", faction)
    os.makedirs(d, exist_ok=True)
    return os.path.join(d, f"{name}.glb")


# ---------------------------------------------------------------- units

def unit_guard(pal, b=None):
    b = b or cl.Builder()
    cl.unit_base(b, pal, helmet="pot", cloak=True)
    cl.weapon_sword(b, pal)
    cl.weapon_shield(b, pal)
    cl.banner(b, pal, tall=26)
    return b


def unit_archer(pal):
    b = cl.Builder()
    cl.unit_base(b, pal, helmet="hood")
    cl.weapon_bow(b, pal)
    b.box((-4, -4, 26), (3, 4, 10), pal["leather"])  # quiver
    return b


def unit_pikeman(pal):
    b = cl.Builder()
    cl.unit_base(b, pal, helmet="pot")
    cl.weapon_spear(b, pal)
    return b


def unit_shieldbearer(pal):
    b = cl.Builder()
    cl.unit_base(b, pal, bulky=1.0, helmet="helm")
    b.box((9, 7, 24), (3, 12, 22), pal["steel"])
    b.box((10.5, 7, 24), (1.5, 6, 6), pal["gold"])
    return b


def unit_cleric(pal):
    b = cl.Builder()
    cl.unit_base(b, pal, helmet="helm", cloak=True)
    cl.weapon_staff(b, pal)
    return b


def unit_lancer(pal):
    b = cl.Builder()
    cl.unit_base(b, pal, helmet="helm", cloak=True)
    cl.weapon_lance(b, pal)
    return b


def unit_ballista(pal):
    b = cl.Builder()
    # siege wagon: deck, wheels, arms, bolts
    b.box((0, 0, 12), (34, 22, 5), pal["timber"])
    for sx in (-10, 10):
        for sy in (-9, 9):
            b.cyl((sx, sy, 6), 6, 2.5, pal["iron"], rot=(math.pi / 2, 0, 0), verts=10)
    b.box((-6, 0, 18), (8, 4, 8), pal["iron"])
    for sy in (-4, 4):
        b.box((14, sy, 20), (26, 2, 2), pal["timber"])
    b.box((24, 0, 20), (10, 3, 3), pal["steel"])
    cl.banner(b, pal, tall=24)
    return b


# ---------------------------------------------------------------- buildings (kitbash)

def slab(b, pal, w=44, d=44, h=4):
    b.box((0, 0, h / 2), (w, d, h), pal["stone_dark"])


def walls(b, pal, w, d, h, x=0, y=0):
    b.box((x, y, h / 2 + 4), (w, d, h), pal["stone"])


def roof_prism(b, pal, w, d, h, x=0, y=0, z=0):
    b.prism_roof((x, y, z), w, d, h, pal["roof"])


def tower(b, pal, x, y, r=9, h=42, roof_h=12):
    b.cyl((x, y, 4 + h / 2), r, h, pal["stone"], verts=8)
    b.cone((x, y, 4 + h + roof_h / 2), r + 2, roof_h, pal["roof"], verts=8)
    b.box((x, y + r * 0.75, 4 + h * 0.55), (2, 2, 10), pal["stone_dark"])


def crystal(b, pal, x, y, z, r=4, h=12):
    b.cone((x, y, z), r, h, pal["accent"], verts=6)


def fence(b, pal, x, y, w, d):
    step = 6
    n = int(w // step)
    for i in range(n + 1):
        b.box((x - w / 2 + i * step, y - d / 2, 8), (1.5, 1.5, 8), pal["timber"])
    b.box((x, y - d / 2, 11), (w, 1, 1.5), pal["timber"])


# --- concrete Vanguard buildings -------------------------------------------

def b_barracks(pal):
    b = cl.Builder()
    slab(b, pal, 46, 46)
    walls(b, pal, 34, 30, 22, y=4)
    roof_prism(b, pal, 30, 38, 12, y=4, z=26)
    tower(b, pal, -19, -15, r=6, h=26, roof_h=9)
    fence(b, pal, 0, -18, 40, 10)
    cl.banner(b, pal, tall=30)
    return b


def b_range_tower(pal):
    b = cl.Builder()
    slab(b, pal, 40, 40)
    tower(b, pal, 0, 0, r=10, h=52, roof_h=14)
    b.box((0, 0, 42), (26, 26, 3), pal["timber"])
    b.box((10, 0, 38), (8, 16, 2), pal["timber"])
    return b


def b_forge(pal):
    b = cl.Builder()
    slab(b, pal, 44, 40)
    walls(b, pal, 32, 26, 18, y=-2)
    roof_prism(b, pal, 28, 34, 10, y=-2, z=22)
    b.box((-12, 8, 30), (8, 8, 26), pal["stone_dark"])  # chimney
    b.box((-12, 8, 44), (10, 10, 3), pal["iron"])
    b.box((12, 12, 14), (6, 6, 8), pal["accent"])  # glowing vent
    return b


def b_pike_yard(pal):
    b = cl.Builder()
    slab(b, pal, 48, 44)
    walls(b, pal, 18, 16, 14, x=-12)
    roof_prism(b, pal, 15, 20, 9, x=-12, z=14)
    for sy in (-10, 0, 10):
        b.box((10, sy, 16), (2, 2, 24), pal["timber"])
        b.cone((10, sy, 30), 1.6, 6, pal["steel"], verts=6)
    fence(b, pal, 0, 0, 44, 40)
    cl.banner(b, pal, tall=26)
    return b


def b_bulwark_hall(pal):
    b = cl.Builder()
    slab(b, pal, 48, 48, h=6)
    walls(b, pal, 38, 38, 16)
    walls(b, pal, 26, 26, 10)
    for sx, sy in ((-15, -15), (15, -15), (-15, 15), (15, 15)):
        b.cyl((sx, sy, 12), 4.5, 24, pal["stone"], verts=8)
    b.box((0, 0, 22), (12, 12, 4), pal["stone_dark"])
    return b


def b_chapel(pal):
    b = cl.Builder()
    slab(b, pal, 40, 44)
    walls(b, pal, 26, 30, 20)
    roof_prism(b, pal, 22, 36, 14, z=20)
    tower(b, pal, 0, 14, r=5, h=34, roof_h=16)
    b.box((0, 18, 52), (2.5, 2.5, 5), pal["gold"])
    for sx in (-8, 8):
        b.box((sx, -15.5, 26), (6, 2, 10), pal["gold"])
    return b


def b_stables(pal):
    b = cl.Builder()
    slab(b, pal, 52, 36)
    walls(b, pal, 40, 22, 14, y=4)
    roof_prism(b, pal, 36, 28, 10, y=4, z=14)
    fence(b, pal, 0, -12, 44, 12)
    return b


def b_siege_workshop(pal):
    b = cl.Builder()
    slab(b, pal, 46, 42)
    walls(b, pal, 32, 28, 16)
    roof_prism(b, pal, 28, 32, 10, z=16)
    b.box((-16, -10, 26), (3, 3, 20), pal["timber"])
    b.box((-10, -10, 34), (16, 2.5, 2.5), pal["timber"])  # crane arm
    b.cyl((-2, -10, 30), 1.2, 8, pal["iron"])
    b.box((12, 0, 24), (14, 6, 4), pal["timber"])  # workbench
    return b


# ---------------------------------------------------------------- castle

def vanguard_castle():
    b = cl.Builder()
    pal = V
    # terrace base
    b.box((0, 0, 4), (150, 120, 8), pal["stone_dark"])
    # main keep
    b.box((0, 0, 38), (58, 58, 68), pal["stone"])
    b.box((0, 0, 74), (50, 50, 6), pal["stone_dark"])
    roof_prism(b, pal, 44, 44, 26, z=77)
    # corner towers
    for sx in (-33, 33):
        for sy in (-33, 33):
            b.cyl((sx, sy, 40), 10, 60, pal["stone"], verts=8)
            b.cone((sx, sy, 82), 12, 20, pal["roof"], verts=8)
            b.cone((sx, sy, 94), 1.5, 6, pal["gold"], verts=6)
    # gate wall toward the field (+X)
    b.box((38, 0, 18), (24, 64, 28), pal["stone"])
    b.box((48, 0, 12), (8, 18, 24), pal["stone_dark"])
    b.cyl((44, 0, 24), 9, 6, pal["iron"], rot=(math.pi / 2, 0, 0), verts=10)
    for sy in (-24, 24):
        b.cyl((38, sy, 40), 6, 34, pal["stone"], verts=8)
        b.cone((38, sy, 62), 7.5, 12, pal["roof"], verts=8)
    # keep windows
    for sy in (-14, 0, 14):
        b.box((28.2, sy, 40), (2, 6, 12), pal["stone_dark"])
    # keep banner (team tint)
    b.cyl((0, 0, 92), 1.6, 30, pal["timber"])
    b.box((7, 0, 100), (13, 0.8, 9), (1, 1, 1, 1), banner=True)
    return b


# ---------------------------------------------------------------- manifest

def unit_manifest():
    return {
        "vanguard": [
            ("vanguard_guard", lambda: unit_guard(V), V, 600),
            ("vanguard_archer", lambda: unit_archer(V), V, 600),
            ("vanguard_pikeman", lambda: unit_pikeman(V), V, 600),
            ("vanguard_shieldbearer", lambda: unit_shieldbearer(V), V, 600),
            ("vanguard_battle_cleric", lambda: unit_cleric(V), V, 600),
            ("vanguard_lancer", lambda: unit_lancer(V), V, 600),
            ("vanguard_ballista", lambda: unit_ballista(V), V, 600),
        ],
        "grove": [],
        "ember": [],
    }


def building_manifest():
    return {
        "vanguard": [
            ("vanguard_barracks", lambda: b_barracks(V), 2000),
            ("vanguard_range_tower", lambda: b_range_tower(V), 2000),
            ("vanguard_forge", lambda: b_forge(V), 2000),
            ("vanguard_pike_yard", lambda: b_pike_yard(V), 2000),
            ("vanguard_bulwark_hall", lambda: b_bulwark_hall(V), 2000),
            ("vanguard_chapel", lambda: b_chapel(V), 2000),
            ("vanguard_stables", lambda: b_stables(V), 2000),
            ("vanguard_siege_workshop", lambda: b_siege_workshop(V), 2000),
        ],
        "grove": [],
        "ember": [],
    }


def castle_manifest():
    return {
        "vanguard": [("vanguard_castle", vanguard_castle, 5000)],
        "grove": [],
        "ember": [],
    }


BUILDERS = {}


def _register(faction, name, fn, budget, out):
    BUILDERS[name] = (fn, out, budget)


for _name, _fn, _pal, _budget in unit_manifest()["vanguard"]:
    _register("vanguard", _name, _fn, _budget, out_path("vanguard", _name))
for _name, _fn, _pal, _budget in unit_manifest()["grove"]:
    _register("grove", _name, _fn, _budget, out_path("grove", _name))
for _name, _fn, _pal, _budget in unit_manifest()["ember"]:
    _register("ember", _name, _fn, _budget, out_path("ember", _name))
for _name, _fn, _budget in building_manifest()["vanguard"]:
    _register("vanguard", _name, _fn, _budget, out_path("vanguard", _name))
for _name, _fn, _budget in building_manifest()["grove"]:
    _register("grove", _name, _fn, _budget, out_path("grove", _name))
for _name, _fn, _budget in building_manifest()["ember"]:
    _register("ember", _name, _fn, _budget, out_path("ember", _name))
for _name, _fn, _budget in castle_manifest()["vanguard"]:
    _register("vanguard", _name, _fn, _budget, out_path("vanguard", _name))
for _name, _fn, _budget in castle_manifest()["grove"]:
    _register("grove", _name, _fn, _budget, out_path("grove", _name))
for _name, _fn, _budget in castle_manifest()["ember"]:
    _register("ember", _name, _fn, _budget, out_path("ember", _name))


def main():
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    kind = None
    for i, a in enumerate(argv):
        if a == "--kind":
            kind = argv[i + 1]
    if not kind:
        print("usage: --kind <name> (one of: ", ", ".join(sorted(BUILDERS)), ")")
        return
    if kind not in BUILDERS:
        print(f"unknown kind {kind}")
        return
    fn, out, budget = BUILDERS[kind]
    cl.clear_scene()
    b = fn()
    cl.finish_and_export(b, kind, out, budget)


main()
