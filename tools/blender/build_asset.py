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


def organic_hall(b, pal, w, d, h):
    b.cyl((0, 0, 4 + h / 2), min(w, d) * 0.42, h, pal["bark"], verts=8)
    cl.leaf_canopy(b, pal, (0, 0, 6 + h), min(w, d) * 0.34)


def spire_tower(b, pal, x, y, r=9, h=42):
    b.cyl((x, y, 4 + h / 2), r, h, pal["obsidian"], verts=6)
    b.cone((x, y, 4 + h + 9), r + 1, 18, pal["charcoal"], verts=6)
    crystal(b, pal, x, y, 4 + h + 20, r=2.5, h=8)


# --- Grove buildings (organic shapes, living wood) ---------------------------

def b_grove_root_den(pal):
    b = cl.Builder()
    slab(b, pal, 46, 46)
    organic_hall(b, pal, 34, 30, 20)
    b.cyl((-18, -14, 12), 4, 20, pal["bark_dark"], verts=6)
    cl.leaf_canopy(b, pal, (-18, -14, 24), 9)
    cl.banner(b, pal, tall=28)
    return b


def b_grove_thorn_spire(pal):
    b = cl.Builder()
    slab(b, pal, 40, 40)
    spire_tower(b, pal, 0, 0, r=9, h=46)
    return b


def b_grove_bloom_well(pal):
    b = cl.Builder()
    slab(b, pal, 42, 42)
    b.cyl((0, 0, 6), 15, 8, pal["stone"], verts=10)
    b.cyl((0, 0, 9), 11, 5, pal["crystal"], verts=10)
    crystal(b, pal, 0, 0, 20, r=4, h=14)
    return b


def b_grove_moss_nursery(pal):
    b = cl.Builder()
    slab(b, pal, 48, 40)
    organic_hall(b, pal, 30, 24, 12)
    fence(b, pal, 0, -14, 40, 10)
    cl.leaf_canopy(b, pal, (0, 8, 20), 12)
    return b


def b_grove_bark_bastion(pal):
    b = cl.Builder()
    slab(b, pal, 48, 48, h=6)
    b.cyl((0, 0, 16), 20, 20, pal["bark"], verts=8)
    b.cyl((0, 0, 30), 15, 12, pal["bark_dark"], verts=8)
    for sx in (-14, 14):
        for sy in (-14, 14):
            b.cyl((sx, sy, 14), 4, 24, pal["bark_dark"], verts=6)
    cl.leaf_canopy(b, pal, (0, 0, 40), 12)
    return b


def b_grove_mire_pool(pal):
    b = cl.Builder()
    slab(b, pal, 44, 44)
    b.cyl((0, 0, 3), 16, 5, pal["bark_dark"], verts=10)
    b.cyl((0, 0, 5), 12, 4, pal["crystal"], verts=10)
    for a in range(4):
        x = 14 * math.cos(a * 1.57)
        y = 14 * math.sin(a * 1.57)
        b.cyl((x, y, 10), 1.2, 16, pal["bark"], verts=5)
    return b


def b_grove_vine_warren(pal):
    b = cl.Builder()
    slab(b, pal, 46, 42)
    organic_hall(b, pal, 26, 22, 10)
    for sy in (-8, 0, 8):
        b.cyl((10, sy, 14), 1.0, 22, pal["leaf"], verts=5)
    fence(b, pal, 0, 0, 40, 36)
    return b


def b_grove_ancient_seed(pal):
    b = cl.Builder()
    slab(b, pal, 42, 42)
    crystal(b, pal, 0, 0, 16, r=6, h=26)
    b.cyl((0, 0, 5), 10, 8, pal["bark"], verts=8)
    cl.leaf_canopy(b, pal, (0, 0, 34), 10)
    return b


# --- Ember buildings (obsidian + fire) ---------------------------------------

def b_ember_cinder_pit(pal):
    b = cl.Builder()
    slab(b, pal, 44, 44)
    walls(b, pal, 32, 28, 14)
    roof_prism(b, pal, 28, 24, 10, z=14)
    for sx in (-8, 8):
        b.box((sx, 0, 26), (5, 5, 14), pal["iron"])
        crystal(b, pal, sx, 0, 36, r=3, h=8)
    return b


def b_ember_flame_spire(pal):
    b = cl.Builder()
    slab(b, pal, 40, 40)
    spire_tower(b, pal, 0, 0, r=9, h=44)
    return b


def b_ember_ash_mine(pal):
    b = cl.Builder()
    slab(b, pal, 46, 40)
    walls(b, pal, 30, 24, 12)
    roof_prism(b, pal, 26, 20, 9, z=12)
    b.box((14, -8, 16), (6, 6, 18), pal["charcoal"])
    return b


def b_ember_spark_kennel(pal):
    b = cl.Builder()
    slab(b, pal, 48, 36)
    walls(b, pal, 36, 20, 12, y=4)
    roof_prism(b, pal, 32, 26, 9, y=4, z=12)
    fence(b, pal, 0, -12, 42, 10)
    crystal(b, pal, 0, 14, 26, r=2.5, h=8)
    return b


def b_ember_obsidian_gate(pal):
    b = cl.Builder()
    slab(b, pal, 46, 44, h=6)
    for sx in (-13, 13):
        b.box((sx, 0, 20), (10, 26, 34), pal["obsidian"])
    b.box((0, 0, 38), (36, 22, 6), pal["charcoal"])
    crystal(b, pal, 0, 0, 48, r=3, h=10)
    return b


def b_ember_blaze_stable(pal):
    b = cl.Builder()
    slab(b, pal, 50, 36)
    walls(b, pal, 38, 20, 13, y=4)
    roof_prism(b, pal, 34, 26, 10, y=4, z=13)
    fence(b, pal, 0, -12, 44, 10)
    return b


def b_ember_smoke_altar(pal):
    b = cl.Builder()
    slab(b, pal, 42, 42)
    b.cyl((0, 0, 6), 14, 10, pal["charcoal"], verts=8)
    b.cyl((0, 0, 13), 9, 6, pal["iron"], verts=8)
    for a in range(3):
        x = 10 * math.cos(a * 2.09)
        y = 10 * math.sin(a * 2.09)
        b.cone((x, y, 16), 2, 10, pal["flame"], verts=5)
    crystal(b, pal, 0, 0, 22, r=3.5, h=12)
    return b


def b_ember_inferno_engine(pal):
    b = cl.Builder()
    slab(b, pal, 46, 42)
    b.box((0, 0, 12), (30, 22, 14), pal["obsidian"])
    b.cyl((-10, 0, 24), 6, 10, pal["iron"], rot=(0, math.pi / 2, 0), verts=8)
    crystal(b, pal, 12, 0, 24, r=4, h=12)
    for sy in (-8, 8):
        b.box((14, sy, 10), (10, 3, 4), pal["iron"])
    return b


def grove_castle():
    b = cl.Builder()
    pal = G
    b.box((0, 0, 4), (150, 120, 8), pal["bark_dark"])
    b.cyl((0, 0, 44), 34, 72, pal["bark"], verts=8)
    b.cyl((0, 0, 84), 28, 10, pal["bark_dark"], verts=8)
    cl.leaf_canopy(b, pal, (0, 0, 96), 30)
    for sx in (-33, 33):
        for sy in (-33, 33):
            b.cyl((sx, sy, 36), 9, 56, pal["bark_dark"], verts=7)
            b.cone((sx, sy, 76), 11, 22, pal["leaf"], verts=7)
            crystal(b, pal, sx, sy, 92, r=1.6, h=6)
    b.box((40, 0, 20), (22, 60, 32), pal["bark"])
    for sy in (-22, 22):
        b.cyl((40, sy, 46), 7, 44, pal["bark_dark"], verts=6)
        cl.leaf_canopy(b, pal, (40, sy, 72), 9)
    b.cyl((0, 0, 104), 1.6, 26, pal["bark_dark"])
    b.box((7, 0, 111), (12, 0.8, 8), (1, 1, 1, 1), banner=True)
    return b


def ember_castle():
    b = cl.Builder()
    pal = E
    b.box((0, 0, 4), (150, 120, 8), pal["charcoal"])
    b.box((0, 0, 38), (58, 58, 68), pal["obsidian"])
    b.box((0, 0, 74), (50, 50, 6), pal["ash"])
    b.cone((0, 0, 92), 30, 26, pal["charcoal"], verts=4)
    crystal(b, pal, 0, 0, 112, r=4, h=14)
    for sx in (-33, 33):
        for sy in (-33, 33):
            b.cyl((sx, sy, 40), 10, 60, pal["obsidian"], verts=6)
            b.cone((sx, sy, 82), 12, 22, pal["ash"], verts=6)
            crystal(b, pal, sx, sy, 98, r=2, h=8)
    b.box((40, 0, 20), (22, 62, 32), pal["obsidian"])
    b.cyl((44, 0, 24), 9, 6, pal["iron"], rot=(math.pi / 2, 0, 0), verts=8)
    for sy in (-22, 22):
        b.cyl((40, sy, 44), 7, 42, pal["obsidian"], verts=6)
        crystal(b, pal, 40, sy, 72, r=2.5, h=10)
    b.cyl((0, 0, 104), 1.6, 26, pal["iron"])
    b.box((7, 0, 111), (12, 0.8, 8), (1, 1, 1, 1), banner=True)
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
        "grove": [
            ("grove_bruiser", lambda: unit_guard(G), G, 600),
            ("grove_needler", lambda: unit_archer(G), G, 600),
            ("grove_sproutling", lambda: unit_pikeman(G), G, 600),
            ("grove_barkguard", lambda: unit_shieldbearer(G), G, 600),
            ("grove_mire_shaman", lambda: unit_cleric(G), G, 600),
            ("grove_vine_stalker", lambda: unit_lancer(G), G, 600),
            ("grove_treant_colossus", lambda: unit_ballista(G), G, 600),
        ],
        "ember": [
            ("ember_runner", lambda: unit_guard(E), E, 600),
            ("ember_caster", lambda: unit_archer(E), E, 600),
            ("ember_spark_imp", lambda: unit_pikeman(E), E, 600),
            ("ember_obsidian_guard", lambda: unit_shieldbearer(E), E, 600),
            ("ember_smoke_witch", lambda: unit_cleric(E), E, 600),
            ("ember_fire_lancer", lambda: unit_lancer(E), E, 600),
            ("ember_cinder_engine", lambda: unit_ballista(E), E, 600),
        ],
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
        "grove": [
            ("grove_root_den", lambda: b_grove_root_den(G), 2000),
            ("grove_thorn_spire", lambda: b_grove_thorn_spire(G), 2000),
            ("grove_bloom_well", lambda: b_grove_bloom_well(G), 2000),
            ("grove_moss_nursery", lambda: b_grove_moss_nursery(G), 2000),
            ("grove_bark_bastion", lambda: b_grove_bark_bastion(G), 2000),
            ("grove_mire_pool", lambda: b_grove_mire_pool(G), 2000),
            ("grove_vine_warren", lambda: b_grove_vine_warren(G), 2000),
            ("grove_ancient_seed", lambda: b_grove_ancient_seed(G), 2000),
        ],
        "ember": [
            ("ember_cinder_pit", lambda: b_ember_cinder_pit(E), 2000),
            ("ember_flame_spire", lambda: b_ember_flame_spire(E), 2000),
            ("ember_ash_mine", lambda: b_ember_ash_mine(E), 2000),
            ("ember_spark_kennel", lambda: b_ember_spark_kennel(E), 2000),
            ("ember_obsidian_gate", lambda: b_ember_obsidian_gate(E), 2000),
            ("ember_blaze_stable", lambda: b_ember_blaze_stable(E), 2000),
            ("ember_smoke_altar", lambda: b_ember_smoke_altar(E), 2000),
            ("ember_inferno_engine", lambda: b_ember_inferno_engine(E), 2000),
        ],
    }


def tree_pine():
    b = cl.Builder()
    b.cyl((0, 0, 14), 3.2, 28, G["bark"], verts=6)
    for i, (z, r) in enumerate(((26, 16), (36, 12.5), (45, 9), (52, 5.5))):
        b.cone((0, 0, z), r, 14, G["leaf"] if i % 2 == 0 else G["leaf_bright"], verts=7)
    return b


def tree_round():
    b = cl.Builder()
    b.cyl((0, 0, 12), 3.6, 24, G["bark_dark"], verts=6)
    b.sphere((0, 0, 36), 15, G["leaf"], segments=7, ring_count=5)
    b.sphere((9, 3, 30), 9, G["leaf_bright"], segments=6, ring_count=4)
    b.sphere((-8, -3, 31), 8, G["leaf_bright"], segments=6, ring_count=4)
    return b


def rock_boulder():
    b = cl.Builder()
    b.box((0, 0, 7), (18, 14, 14), V["stone_dark"], rot=(0, 0.4, 0.1))
    b.box((6, 4, 15), (10, 8, 8), V["stone"], rot=(0, 0.9, -0.2))
    return b


def doodad_manifest():
    return {
        "doodads": [
            ("tree_pine", tree_pine, 300),
            ("tree_round", tree_round, 300),
            ("rock_boulder", rock_boulder, 300),
        ]
    }


def castle_manifest():
    return {
        "vanguard": [("vanguard_castle", vanguard_castle, 5000)],
        "grove": [("grove_castle", grove_castle, 5000)],
        "ember": [("ember_castle", ember_castle, 5000)],
    }


BUILDERS = {}


def _register(faction, name, fn, budget, out):
    BUILDERS[name] = (fn, out, budget)


for _name, _fn, _budget in doodad_manifest()["doodads"]:
    _register("doodads", _name, _fn, _budget, out_path("doodads", _name))


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
