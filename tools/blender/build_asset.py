"""Builds one Castle Lanes model and exports it as GLB (plan-0.2.md §5).

Usage:
    blender -b -P tools/blender/build_asset.py -- --kind vanguard_castle
    blender -b -P tools/blender/build_asset.py -- --all-units

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
#
# Three factions, three silhouettes: Vanguard = ordered steel with plumes,
# Grove = living wood with antlers and bark plating, Ember = horned
# obsidian with fire accents. Same humanoid rig, different reads.

# --- Vanguard ---

def unit_guard(pal, b=None):
    b = b or cl.Builder()
    cl.unit_base(b, pal, helmet="great_helm", cloak=True)
    cl.weapon_sword(b, pal)
    cl.weapon_shield(b, pal)
    cl.banner(b, pal, tall=26)
    return b


def unit_archer(pal):
    b = cl.Builder()
    cl.unit_base(b, pal, helmet="hood")
    cl.weapon_crossbow(b, pal)
    b.rounded_box((-4, -4, 27), (3, 5, 11), pal["leather"])
    for dz in (2, 0, -2):
        b.cyl((-4.4, -4, 32 + dz), 0.5, 1.2, pal["gold"], rot=(0, 1.57, 0), verts=5)
    return b


def unit_pikeman(pal):
    b = cl.Builder()
    cl.unit_base(b, pal, helmet="plume")
    cl.weapon_spear(b, pal)
    return b


def unit_shieldbearer(pal):
    b = cl.Builder()
    cl.unit_base(b, pal, bulky=1.0, helmet="great_helm")
    # towering kite shield
    b.rounded_box((8.6, 7.4, 25), (3, 12.5, 24), pal["steel"], bevel=0.3)
    b.tapered_cyl((8.6, 7.4, 15.5), 5.6, 4.4, 5, pal["steel"], verts=9)
    b.box((10.0, 7.4, 26), (1.1, 4.6, 5), pal["gold"])
    cl.weapon_mace(b, pal)
    return b


def unit_cleric(pal):
    b = cl.Builder()
    cl.unit_base(b, pal, helmet="crown", cloak=True)
    cl.weapon_staff(b, pal)
    # holy tome on the hip
    b.rounded_box((-3, 4.6, 18), (4, 5, 6), pal["cloth"], bevel=0.2)
    b.box((-1.2, 4.6, 18), (0.6, 5.2, 6.2), pal["gold"])
    return b


def unit_lancer(pal):
    b = cl.Builder()
    cl.unit_base(b, pal, helmet="plume", cloak=True)
    cl.weapon_lance(b, pal)
    return b


def unit_ballista(pal):
    b = cl.Builder()
    # siege wagon: braced deck, spoked wheels, twin arms, big bolt
    b.box((0, 0, 12), (34, 22, 5), pal["timber"])
    b.box((0, 0, 15), (30, 18, 3), pal["iron"])
    for sx in (-10, 10):
        for sy in (-9, 9):
            b.cyl((sx, sy, 6), 6.5, 2.5, pal["iron"], rot=(math.pi / 2, 0, 0),
                  verts=12)
            for a in range(4):
                ang = a * math.pi / 2
                b.box((sx + 4.4 * math.cos(ang), sy, 6),
                      (8.8, 1.2, 1.2), pal["timber"],
                      rot=(0, 0, ang))
    b.box((-6, 0, 19), (8, 4, 9), pal["iron"])
    for sy in (-4, 4):
        b.tapered_cyl((12, sy, 21), 1.4, 1.0, 26, pal["timber"],
                      rot=(0, math.pi / 2, 0), verts=6)
    b.cone((26, 0, 21), 2.4, 10, pal["steel"], rot=(0, math.pi / 2, 0), verts=6)
    cl.banner(b, pal, tall=24)
    return b


# --- Grove (bark-and-leaf read; green skin, antlers, wood weapons) ---

def grove_bruiser(pal):
    b = cl.Builder()
    cl.unit_base(b, pal, bulky=0.8, helmet="antlers", cloak=True, skin=pal["skin"])
    cl.weapon_axe(b, pal)
    b.sphere((2.6, -7.4, 15.2), 2.1, pal["skin"], segments=8, ring_count=5)
    # mossy shoulder pads
    for sy in (-7.4, 7.4):
        b.sphere((1.5, sy, 31), 4.7, pal["moss"], segments=9, ring_count=6)
    return b


def grove_needler(pal):
    b = cl.Builder()
    cl.unit_base(b, pal, helmet="hood")
    cl.weapon_bow(b, pal)
    b.rounded_box((-4, -4, 27), (3, 5, 11), pal["bark"])
    for dz in (2, 0, -2):
        b.cone((-4.4, -4, 32 + dz), 0.6, 2.4, pal["leaf_bright"],
               rot=(math.pi / 2, 0, 0), verts=5)
    return b


def grove_sproutling(pal):
    b = cl.Builder()
    cl.unit_base(b, pal, scale=0.86, helmet="antlers")
    cl.weapon_spear(b, pal)
    # leaf capes on the shoulders
    for sy in (-6.5, 6.5):
        b.cone((1.2, sy, 32), 2.6, 5, pal["leaf_bright"],
               rot=(math.radians(110), 0, 0), verts=6, smooth=True)
    return b


def grove_barkguard(pal):
    b = cl.Builder()
    cl.unit_base(b, pal, bulky=1.1, helmet="antlers", skin=pal["bark"])
    cl.weapon_club(b, pal)
    cl.weapon_shield(b, pal, round=True)
    # bark plating across the chest
    for dz in (0, 4, 8):
        b.tapered_cyl((1.4, 0, 23.5 + dz), 6.4 - dz * 0.22, 6.0 - dz * 0.22, 3.4,
                      pal["bark_dark"], verts=9, smooth=True)
    return b


def grove_mire_shaman(pal):
    b = cl.Builder()
    cl.unit_base(b, pal, helmet="hood", cloak=True, skin=pal["skin"])
    cl.weapon_staff(b, pal, orb=pal["crystal"])
    # firefly seeds orbiting the staff top
    for a in range(3):
        ang = a * 2.1
        b.sphere((5 + 3.4 * math.cos(ang), -6.5 + 3.4 * math.sin(ang), 47),
                 0.9, pal["leaf_bright"], segments=7, ring_count=4)
    return b


def grove_vine_stalker(pal):
    b = cl.Builder()
    cl.unit_base(b, pal, helmet="hood", cloak=True)
    cl.weapon_spear(b, pal)
    # coiling vines across the chest
    for dz in (0, 5):
        b.tapered_cyl((2.0, 0, 24 + dz), 6.6 - dz * 0.3, 6.2 - dz * 0.3, 2.6,
                      pal["leaf"], verts=8, smooth=True)
    return b


def grove_treant_colossus(pal):
    b = cl.Builder()
    h = 1.5
    # root legs
    for sy in (-5, 5):
        b.tapered_cyl((-2, sy * 1.2, 12 * h), 4.4 * h, 3.0 * h, 16 * h,
                      pal["bark_dark"], verts=7, smooth=True)
        for a in (-0.5, 0.5):
            b.tapered_cyl((6 * h, sy * 1.35 + 3 * a, 3 * h), 1.6 * h, 0.9 * h,
                          9 * h, pal["bark"], verts=6, smooth=True)
    # barrel torso
    b.tapered_cyl((0, 0, 27 * h), 10.5 * h, 8.6 * h, 22 * h, pal["bark"],
                  verts=10, smooth=True)
    b.tapered_cyl((0, 0, 33 * h), 8.4 * h, 7.4 * h, 8 * h, pal["moss"],
                  verts=10, smooth=True)
    # branch arms
    for sy in (-9, 9):
        b.tapered_cyl((2 * h, sy, 34 * h), 3.0 * h, 1.8 * h, 16 * h,
                      pal["bark"], rot=(math.radians(22), 0, 0), verts=7,
                      smooth=True)
        b.tapered_cyl((10 * h, sy * 1.2, 38 * h), 2.2 * h, 1.2 * h, 12 * h,
                      pal["bark_dark"], rot=(math.radians(-12), 0, 0),
                      verts=6, smooth=True)
    # canopy head
    cl.leaf_canopy(b, pal, (2, 0, 47 * h), 8.5 * h)
    cl.leaf_canopy(b, pal, (5, 3, 44 * h), 5.5 * h, pal["leaf_bright"])
    cl.leaf_canopy(b, pal, (5, -3, 44 * h), 5.5 * h, pal["leaf_bright"])
    return b


# --- Ember (horned obsidian with magma glow) ---

def ember_runner(pal):
    b = cl.Builder()
    cl.unit_base(b, pal, helmet="horned", cloak=True)
    cl.weapon_sword(b, pal)
    # magma veins on the chest
    for dz in (0, 4.5):
        b.tapered_cyl((2.4, 0, 25 + dz), 4.6 - dz * 0.25, 3.8 - dz * 0.25, 2.4,
                      pal["ember"], verts=8, smooth=True)
    return b


def ember_caster(pal):
    b = cl.Builder()
    cl.unit_base(b, pal, helmet="hood")
    cl.weapon_staff(b, pal, orb=pal["flame"])
    b.sphere((5, -6.5, 51.5), 1.4, pal["flame"], segments=8, ring_count=5)
    return b


def ember_spark_imp(pal):
    b = cl.Builder()
    cl.unit_base(b, pal, scale=0.8, helmet="horned")
    cl.weapon_club(b, pal)
    # imp tail
    b.tapered_cyl((-5, 0, 14), 1.0, 0.5, 14, pal["skin"],
                  rot=(math.radians(115), 0, 0), verts=6, smooth=True)
    b.cone((-8.5, 0, 20.5), 1.3, 3.4, pal["ember"], rot=(0, 1.57, 0), verts=5)
    return b


def ember_obsidian_guard(pal):
    b = cl.Builder()
    cl.unit_base(b, pal, bulky=1.1, helmet="horned", skin=pal["charcoal"])
    cl.weapon_mace(b, pal)
    cl.weapon_shield(b, pal)
    # obsidian plate layers
    for dz in (0, 4, 8):
        b.tapered_cyl((2.0, 0, 23.5 + dz), 6.6 - dz * 0.22, 6.2 - dz * 0.22, 3.4,
                      pal["obsidian"], verts=9, smooth=True)
    return b


def ember_smoke_witch(pal):
    b = cl.Builder()
    cl.unit_base(b, pal, helmet="hood", cloak=True)
    cl.weapon_staff(b, pal, orb=pal["magma"])
    # smoke wisps coiling the staff
    for dz in (30, 36, 42):
        b.tapered_cyl((5 + 2.2 * math.cos(dz), -6.5 + 2.2 * math.sin(dz), dz),
                      0.5, 0.3, 7, pal["ash"],
                      rot=(math.radians(30), math.radians(30), 0), verts=5,
                      smooth=True)
    return b


def ember_fire_lancer(pal):
    b = cl.Builder()
    cl.unit_base(b, pal, helmet="horned", cloak=True)
    cl.weapon_lance(b, pal, tip_color=pal["flame"])
    b.cone((34, -7, 27), 1.4, 5, pal["magma"], rot=(0, math.pi / 2, 0),
           verts=6, smooth=True)
    return b


def ember_cinder_engine(pal):
    b = cl.Builder()
    # obsidian siege wagon with a magma boiler
    b.box((0, 0, 12), (34, 22, 5), pal["charcoal"])
    b.box((0, 0, 15.5), (30, 18, 3), pal["iron"])
    for sx in (-10, 10):
        for sy in (-9, 9):
            b.cyl((sx, sy, 6), 6.5, 2.5, pal["iron"], rot=(math.pi / 2, 0, 0),
                  verts=12)
    b.cyl((-6, 0, 23), 7, 12, pal["obsidian"], verts=10, smooth=True)
    b.tapered_cyl((-6, 0, 31), 5, 2.6, 7, pal["ember"], verts=10, smooth=True)
    b.cone((-6, 0, 37), 2.2, 5, pal["flame"], verts=7, smooth=True)
    for sy in (-4, 4):
        b.tapered_cyl((12, sy, 21), 1.4, 1.0, 26, pal["charcoal"],
                      rot=(0, math.pi / 2, 0), verts=6)
    b.cone((26, 0, 21), 2.4, 10, pal["magma"], rot=(0, math.pi / 2, 0), verts=6)
    cl.banner(b, pal, tall=24)
    return b


# ---------------------------------------------------------------- buildings (kitbash)

def slab(b, pal, w=44, d=44, h=4):
    b.box((0, 0, h / 2), (w, d, h), pal["stone_dark"])


def walls(b, pal, w, d, h, x=0, y=0):
    b.rounded_box((x, y, h / 2 + 4), (w, d, h), pal["stone"], bevel=0.35)


def roof_prism(b, pal, w, d, h, x=0, y=0, z=0):
    b.prism_roof((x, y, z), w, d, h, pal["roof"])


def tower(b, pal, x, y, r=9, h=42, roof_h=12):
    """Stone tower: tapered shaft, corbel ring, onion roof, gold finial."""
    b.tapered_cyl((x, y, 4 + h / 2), r * 1.12, r * 0.92, h, pal["stone"],
                  verts=10, smooth=True)
    b.tapered_cyl((x, y, 4 + h + 1), r * 1.12, r * 1.02, 2, pal["stone_dark"],
                  verts=10)
    cl.dome_roof((x, y, 4 + h + roof_h * 0.6 + 2), r + 2, roof_h, pal["roof"],
                 verts=10)
    b.cone((x, y, 4 + h + roof_h * 1.15 + 3), 1.1, 5, pal["gold"], verts=6)
    # arrow slit
    b.box((x + r * 0.9, y, 4 + h * 0.55), (1.6, 1.4, 8), pal["stone_dark"])


def crystal(b, pal, x, y, z, r=4, h=12):
    b.icosa((x, y, z), r, pal["accent"], rot=(0.4, 0.3, 0.2))
    b.cone((x, y, z + h * 0.42), r * 0.55, h, pal["accent"], verts=6)


def fence(b, pal, x, y, w, d):
    step = 6
    n = int(w // step)
    for i in range(n + 1):
        b.box((x - w / 2 + i * step, y - d / 2, 8), (1.5, 1.5, 8),
              pal["timber"])
    b.box((x, y - d / 2, 11), (w, 1, 1.5), pal["timber"])


def windows(b, pal, x, y, face_y, count, spread, z, w=4, h=8):
    for i in range(count):
        off = (i - (count - 1) / 2) * spread
        b.box((x + off, face_y, z), (w, 1.6, h), pal["stone_dark"])


# --- concrete Vanguard buildings -------------------------------------------

def b_barracks(pal):
    b = cl.Builder()
    slab(b, pal, 46, 46)
    walls(b, pal, 34, 30, 22, y=4)
    roof_prism(b, pal, 30, 38, 12, y=4, z=26)
    b.box((17.4, 4, 14), (1.4, 8, 10), pal["stone_dark"])  # gate
    tower(b, pal, -19, -15, r=6, h=26, roof_h=9)
    windows(b, pal, 0, -11.2, -11.2, 3, 10, 18)
    fence(b, pal, 0, -18, 40, 10)
    cl.banner(b, pal, tall=30)
    return b


def b_range_tower(pal):
    b = cl.Builder()
    slab(b, pal, 40, 40)
    tower(b, pal, 0, 0, r=10, h=52, roof_h=14)
    b.box((0, 0, 44), (26, 26, 3), pal["timber"])
    b.box((11, 0, 40), (8, 16, 2), pal["timber"])
    for a in range(4):
        ang = a * math.pi / 2
        b.box((13 * math.cos(ang) - 1, 13 * math.sin(ang) - 1, 48),
              (2.2, 2.2, 5), pal["stone"])
    return b


def b_forge(pal):
    b = cl.Builder()
    slab(b, pal, 44, 40)
    walls(b, pal, 32, 26, 18, y=-2)
    roof_prism(b, pal, 28, 34, 10, y=-2, z=22)
    b.tapered_cyl((-12, 8, 32), 5, 4, 26, pal["stone_dark"], verts=8,
                  smooth=True)
    b.box((-12, 8, 46), (10, 10, 3), pal["iron"])
    b.icosa((12, 12, 15), 3.4, pal["accent"], rot=(0.4, 0.3, 0.2))
    b.box((12, 12, 10), (6, 6, 9), pal["stone_dark"])
    return b


def b_pike_yard(pal):
    b = cl.Builder()
    slab(b, pal, 48, 44)
    walls(b, pal, 18, 16, 14, x=-12)
    roof_prism(b, pal, 15, 20, 9, x=-12, z=14)
    for sy in (-10, 0, 10):
        b.cyl((10, sy, 16), 1.1, 24, pal["timber"], verts=6)
        b.cone((10, sy, 30), 1.7, 7, pal["steel"], verts=6, smooth=True)
    fence(b, pal, 0, 0, 44, 40)
    cl.banner(b, pal, tall=26)
    return b


def b_bulwark_hall(pal):
    b = cl.Builder()
    slab(b, pal, 48, 48, h=6)
    walls(b, pal, 38, 38, 16)
    walls(b, pal, 26, 26, 10)
    b.tapered_cyl((0, 0, 30), 9, 7.2, 10, pal["stone"], verts=10, smooth=True)
    for sx, sy in ((-15, -15), (15, -15), (-15, 15), (15, 15)):
        b.tapered_cyl((sx, sy, 16), 5.2, 4.2, 24, pal["stone"], verts=8,
                      smooth=True)
        cl.dome_roof((sx, sy, 31), 6, 8, pal["roof"], verts=8)
    cl.banner(b, pal, tall=34)
    return b


def b_chapel(pal):
    b = cl.Builder()
    slab(b, pal, 40, 44)
    walls(b, pal, 26, 30, 20)
    roof_prism(b, pal, 22, 36, 14, z=20)
    tower(b, pal, 0, 14, r=5, h=34, roof_h=16)
    b.cone((0, 14, 62), 2.2, 6, pal["gold"], verts=6)
    for sx in (-8, 8):
        b.box((sx, -15.5, 26), (6, 2, 10), pal["gold"])
    cl.banner(b, pal, tall=28)
    return b


def b_stables(pal):
    b = cl.Builder()
    slab(b, pal, 52, 36)
    walls(b, pal, 40, 22, 14, y=4)
    roof_prism(b, pal, 36, 28, 10, y=4, z=14)
    fence(b, pal, 0, -12, 44, 12)
    for sx in (-14, 0, 14):
        b.cyl((sx, -8, 12), 1.2, 16, pal["timber"], verts=6)
    return b


def b_siege_workshop(pal):
    b = cl.Builder()
    slab(b, pal, 46, 42)
    walls(b, pal, 32, 28, 16)
    roof_prism(b, pal, 28, 32, 10, z=16)
    b.tapered_cyl((-16, -10, 26), 1.6, 1.2, 20, pal["timber"], verts=6)
    b.box((-8, -10, 34), (16, 2.5, 2.5), pal["timber"])  # crane arm
    b.cyl((-1, -10, 30), 1.2, 8, pal["iron"])
    b.box((12, 0, 24), (14, 6, 4), pal["timber"])
    b.icosa((12, 0, 29), 2.4, pal["accent"], rot=(0.5, 0.2, 0.3))
    return b


def organic_hall(b, pal, w, d, h):
    b.tapered_cyl((0, 0, 4 + h / 2), min(w, d) * 0.46, min(w, d) * 0.36, h,
                  pal["bark"], verts=10, smooth=True)
    cl.leaf_canopy(b, pal, (0, 0, 6 + h), min(w, d) * 0.34)


def spire_tower(b, pal, x, y, r=9, h=42):
    b.tapered_cyl((x, y, 4 + h / 2), r * 1.15, r * 0.7, h, pal["obsidian"],
                  verts=8, smooth=True)
    b.cone((x, y, 4 + h + 10), r + 1.5, 20, pal["charcoal"], verts=8,
           smooth=True)
    crystal(b, pal, x, y, 4 + h + 24, r=2.5, h=9)
    for a in range(3):
        ang = a * 2.09
        b.tapered_cyl((x + r * math.cos(ang), y + r * math.sin(ang), 4 + h * 0.6),
                      0.8, 0.5, h * 0.7, pal["bark_dark"], verts=5, smooth=True)


# --- Grove buildings (organic shapes, living wood) ---------------------------

def b_grove_root_den(pal):
    b = cl.Builder()
    slab(b, pal, 46, 46)
    organic_hall(b, pal, 34, 30, 20)
    b.tapered_cyl((-18, -14, 12), 4.4, 3, 20, pal["bark_dark"], verts=7,
                  smooth=True)
    cl.leaf_canopy(b, pal, (-18, -14, 26), 9)
    # roots crawling off the slab
    for a in range(4):
        ang = a * 1.57 + 0.4
        b.tapered_cyl((16 * math.cos(ang), 16 * math.sin(ang), 3.5), 1.6, 0.8,
                      12, pal["bark_dark"], verts=5, smooth=True)
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
    b.cyl((0, 0, 6), 15, 8, pal["stone"], verts=12)
    b.cyl((0, 0, 9), 11, 5, pal["crystal"], verts=12)
    crystal(b, pal, 0, 0, 22, r=4, h=15)
    for a in range(5):
        ang = a * 1.26
        b.cone((11 * math.cos(ang), 11 * math.sin(ang), 12), 0.8, 6,
               pal["leaf_bright"], rot=(0.35 * math.sin(ang), -0.35 * math.cos(ang), 0),
               verts=5, smooth=True)
    return b


def b_grove_moss_nursery(pal):
    b = cl.Builder()
    slab(b, pal, 48, 40)
    organic_hall(b, pal, 30, 24, 12)
    fence(b, pal, 0, -14, 40, 10)
    cl.leaf_canopy(b, pal, (0, 8, 22), 12)
    cl.leaf_canopy(b, pal, (-8, 2, 16), 7, pal["leaf_bright"])
    return b


def b_grove_bark_bastion(pal):
    b = cl.Builder()
    slab(b, pal, 48, 48, h=6)
    b.tapered_cyl((0, 0, 18), 21, 17, 24, pal["bark"], verts=10, smooth=True)
    b.tapered_cyl((0, 0, 33), 15, 13, 12, pal["bark_dark"], verts=10,
                  smooth=True)
    for sx in (-14, 14):
        for sy in (-14, 14):
            b.tapered_cyl((sx, sy, 16), 4.6, 3.4, 24, pal["bark_dark"],
                          verts=7, smooth=True)
            cl.leaf_canopy(b, pal, (sx, sy, 32), 4.5)
    cl.leaf_canopy(b, pal, (0, 0, 44), 13)
    return b


def b_grove_mire_pool(pal):
    b = cl.Builder()
    slab(b, pal, 44, 44)
    b.tapered_cyl((0, 0, 3), 17, 15, 5, pal["bark_dark"], verts=12, smooth=True)
    b.cyl((0, 0, 5.5), 12, 4, pal["crystal"], verts=12)
    for a in range(4):
        x = 14 * math.cos(a * 1.57)
        y = 14 * math.sin(a * 1.57)
        b.tapered_cyl((x, y, 11), 1.3, 0.8, 17, pal["bark"], rot=(0.12 * math.sin(a * 1.57), -0.12 * math.cos(a * 1.57), 0), verts=6,
                      smooth=True)
    return b


def b_grove_vine_warren(pal):
    b = cl.Builder()
    slab(b, pal, 46, 42)
    organic_hall(b, pal, 26, 22, 10)
    for sy in (-8, 0, 8):
        b.tapered_cyl((10, sy, 15), 1.1, 0.7, 24, pal["leaf"],
                      rot=(0.1, -0.15, 0), verts=6, smooth=True)
    fence(b, pal, 0, 0, 40, 36)
    return b


def b_grove_ancient_seed(pal):
    b = cl.Builder()
    slab(b, pal, 42, 42)
    b.tapered_cyl((0, 0, 7), 11, 8.5, 9, pal["bark"], verts=9, smooth=True)
    crystal(b, pal, 0, 0, 22, r=6, h=22)
    cl.leaf_canopy(b, pal, (0, 0, 38), 10)
    for a in range(3):
        ang = a * 2.09
        b.sphere((9 * math.cos(ang), 9 * math.sin(ang), 8), 1.6,
                 pal["leaf_bright"], segments=8, ring_count=5)
    return b


# --- Ember buildings (obsidian + fire) ---------------------------------------

def b_ember_cinder_pit(pal):
    b = cl.Builder()
    slab(b, pal, 44, 44)
    walls(b, pal, 32, 28, 14)
    roof_prism(b, pal, 28, 24, 10, z=14)
    for sx in (-8, 8):
        b.tapered_cyl((sx, 0, 26), 3.4, 2.6, 15, pal["iron"], verts=8,
                      smooth=True)
        crystal(b, pal, sx, 0, 38, r=3, h=9)
    return b


def b_ember_flame_spire(pal):
    b = cl.Builder()
    slab(b, pal, 40, 40)
    spire_tower(b, pal, 0, 0, r=9, h=44)
    b.cone((0, 0, 66), 2.6, 8, pal["flame"], verts=7, smooth=True)
    return b


def b_ember_ash_mine(pal):
    b = cl.Builder()
    slab(b, pal, 46, 40)
    walls(b, pal, 30, 24, 12)
    roof_prism(b, pal, 26, 20, 9, z=12)
    b.tapered_cyl((14, -8, 18), 4, 3, 20, pal["charcoal"], verts=7, smooth=True)
    b.icosa((14, -8, 31), 2.6, pal["magma"], rot=(0.4, 0.3, 0.2))
    return b


def b_ember_spark_kennel(pal):
    b = cl.Builder()
    slab(b, pal, 48, 36)
    walls(b, pal, 36, 20, 12, y=4)
    roof_prism(b, pal, 32, 26, 9, y=4, z=12)
    fence(b, pal, 0, -12, 42, 10)
    crystal(b, pal, 0, 14, 27, r=2.5, h=9)
    return b


def b_ember_obsidian_gate(pal):
    b = cl.Builder()
    slab(b, pal, 46, 44, h=6)
    for sx in (-13, 13):
        b.rounded_box((sx, 0, 20), (10, 26, 34), pal["obsidian"], bevel=0.25)
    b.box((0, 0, 38), (36, 22, 6), pal["charcoal"])
    crystal(b, pal, 0, 0, 49, r=3, h=11)
    for sy in (-8, 0, 8):
        b.cone((13.2, sy, 26), 0.9, 4, pal["ember"], rot=(0, 1.57, 0), verts=5)
    return b


def b_ember_blaze_stable(pal):
    b = cl.Builder()
    slab(b, pal, 50, 36)
    walls(b, pal, 38, 20, 13, y=4)
    roof_prism(b, pal, 34, 26, 10, y=4, z=13)
    fence(b, pal, 0, -12, 44, 10)
    for sx in (-16, 0, 16):
        b.cone((sx, -12, 14), 0.8, 5, pal["flame"], verts=5, smooth=True)
    return b


def b_ember_smoke_altar(pal):
    b = cl.Builder()
    slab(b, pal, 42, 42)
    b.tapered_cyl((0, 0, 7), 15, 12, 10, pal["charcoal"], verts=10, smooth=True)
    b.cyl((0, 0, 14), 9, 6, pal["iron"], verts=10)
    for a in range(3):
        x = 10 * math.cos(a * 2.09)
        y = 10 * math.sin(a * 2.09)
        b.cone((x, y, 18), 2, 11, pal["flame"], verts=6, smooth=True)
    crystal(b, pal, 0, 0, 26, r=3.5, h=13)
    return b


def b_ember_inferno_engine(pal):
    b = cl.Builder()
    slab(b, pal, 46, 42)
    b.rounded_box((0, 0, 12), (30, 22, 14), pal["obsidian"], bevel=0.3)
    b.tapered_cyl((-10, 0, 25), 6, 4.4, 11, pal["iron"], rot=(0, math.pi / 2, 0),
                  verts=9, smooth=True)
    crystal(b, pal, 12, 0, 25, r=4, h=13)
    for sy in (-8, 8):
        b.box((14, sy, 10), (10, 3, 4), pal["iron"])
    b.cone((-10, 0, 34), 2.2, 6, pal["flame"], verts=7, smooth=True)
    return b


def grove_castle():
    b = cl.Builder()
    pal = G
    b.box((0, 0, 4), (150, 120, 8), pal["bark_dark"])
    b.tapered_cyl((0, 0, 44), 38, 30, 72, pal["bark"], verts=12, smooth=True)
    b.tapered_cyl((0, 0, 84), 29, 26, 10, pal["bark_dark"], verts=12,
                  smooth=True)
    cl.leaf_canopy(b, pal, (0, 0, 98), 32)
    cl.leaf_canopy(b, pal, (-14, 8, 86), 18, pal["leaf_bright"])
    cl.leaf_canopy(b, pal, (16, -6, 88), 16, pal["leaf_bright"])
    for sx in (-33, 33):
        for sy in (-33, 33):
            b.tapered_cyl((sx, sy, 38), 10.5, 7.5, 56, pal["bark_dark"],
                          verts=9, smooth=True)
            b.cone((sx, sy, 78), 11, 24, pal["leaf"], verts=9, smooth=True)
            crystal(b, pal, sx, sy, 94, r=1.6, h=6)
    b.box((40, 0, 20), (22, 60, 32), pal["bark"])
    for sy in (-22, 22):
        b.tapered_cyl((40, sy, 46), 7.5, 5.5, 44, pal["bark_dark"], verts=8,
                      smooth=True)
        cl.leaf_canopy(b, pal, (40, sy, 74), 9)
    b.cyl((0, 0, 108), 1.6, 26, pal["bark_dark"])
    b.box((7, 0, 115), (12, 0.8, 8), (1, 1, 1, 1), banner=True)
    return b


def ember_castle():
    b = cl.Builder()
    pal = E
    b.box((0, 0, 4), (150, 120, 8), pal["charcoal"])
    b.rounded_box((0, 0, 38), (58, 58, 68), pal["obsidian"], bevel=0.5)
    b.box((0, 0, 74), (50, 50, 6), pal["ash"])
    b.cone((0, 0, 94), 30, 30, pal["charcoal"], verts=5, smooth=True)
    crystal(b, pal, 0, 0, 116, r=4.5, h=15)
    for sx in (-33, 33):
        for sy in (-33, 33):
            b.tapered_cyl((sx, sy, 40), 11, 8, 60, pal["obsidian"], verts=8,
                          smooth=True)
            b.cone((sx, sy, 84), 12, 24, pal["ash"], verts=8, smooth=True)
            crystal(b, pal, sx, sy, 100, r=2, h=9)
    b.box((40, 0, 20), (22, 62, 32), pal["obsidian"])
    b.cyl((44, 0, 24), 9, 6, pal["iron"], rot=(math.pi / 2, 0, 0), verts=10)
    for sy in (-22, 22):
        b.tapered_cyl((40, sy, 44), 7.5, 5.5, 42, pal["obsidian"], verts=8,
                      smooth=True)
        crystal(b, pal, 40, sy, 72, r=2.5, h=11)
    b.cyl((0, 0, 106), 1.6, 26, pal["iron"])
    b.box((7, 0, 113), (12, 0.8, 8), (1, 1, 1, 1), banner=True)
    return b


# ---------------------------------------------------------------- castle

def vanguard_castle():
    b = cl.Builder()
    pal = V
    # terrace base
    b.box((0, 0, 4), (150, 120, 8), pal["stone_dark"])
    # main keep
    b.rounded_box((0, 0, 38), (58, 58, 68), pal["stone"], bevel=0.6)
    b.box((0, 0, 74), (50, 50, 6), pal["stone_dark"])
    cl.dome_roof((0, 0, 88), 27, 20, pal["roof"], verts=12)
    b.cone((0, 0, 104), 1.8, 8, pal["gold"], verts=6)
    # corner towers
    for sx in (-33, 33):
        for sy in (-33, 33):
            b.tapered_cyl((sx, sy, 40), 11, 8.5, 60, pal["stone"], verts=10,
                          smooth=True)
            cl.dome_roof((sx, sy, 84), 12, 20, pal["roof"], verts=10)
            b.cone((sx, sy, 98), 1.5, 6, pal["gold"], verts=6)
    # gate wall toward the field (+X)
    b.box((38, 0, 18), (24, 64, 28), pal["stone"])
    b.tapered_cyl((47, 0, 12), 12, 10, 24, pal["stone_dark"], verts=10,
                  smooth=True)
    b.cyl((44, 0, 24), 9, 6, pal["iron"], rot=(math.pi / 2, 0, 0), verts=12)
    for sy in (-24, 24):
        b.tapered_cyl((38, sy, 40), 6.6, 5, 34, pal["stone"], verts=9,
                      smooth=True)
        cl.dome_roof((38, sy, 62), 7.5, 12, pal["roof"], verts=8)
    # keep windows
    for sy in (-14, 0, 14):
        b.box((28.2, sy, 40), (2, 6, 12), pal["stone_dark"])
    # keep banner (team tint)
    b.cyl((0, 0, 100), 1.6, 30, pal["timber"])
    b.box((7, 0, 108), (13, 0.8, 9), (1, 1, 1, 1), banner=True)
    return b


# ---------------------------------------------------------------- manifest

def unit_manifest():
    return {
        "vanguard": [
            ("vanguard_guard", lambda: unit_guard(V), V, 2600),
            ("vanguard_archer", lambda: unit_archer(V), V, 2600),
            ("vanguard_pikeman", lambda: unit_pikeman(V), V, 2600),
            ("vanguard_shieldbearer", lambda: unit_shieldbearer(V), V, 2600),
            ("vanguard_battle_cleric", lambda: unit_cleric(V), V, 2600),
            ("vanguard_lancer", lambda: unit_lancer(V), V, 2600),
            ("vanguard_ballista", lambda: unit_ballista(V), V, 2600),
        ],
        "grove": [
            ("grove_bruiser", lambda: grove_bruiser(G), G, 2600),
            ("grove_needler", lambda: grove_needler(G), G, 2600),
            ("grove_sproutling", lambda: grove_sproutling(G), G, 2600),
            ("grove_barkguard", lambda: grove_barkguard(G), G, 2600),
            ("grove_mire_shaman", lambda: grove_mire_shaman(G), G, 2600),
            ("grove_vine_stalker", lambda: grove_vine_stalker(G), G, 2600),
            ("grove_treant_colossus", lambda: grove_treant_colossus(G), G, 4000),
        ],
        "ember": [
            ("ember_runner", lambda: ember_runner(E), E, 2600),
            ("ember_caster", lambda: ember_caster(E), E, 2600),
            ("ember_spark_imp", lambda: ember_spark_imp(E), E, 2600),
            ("ember_obsidian_guard", lambda: ember_obsidian_guard(E), E, 2600),
            ("ember_smoke_witch", lambda: ember_smoke_witch(E), E, 2600),
            ("ember_fire_lancer", lambda: ember_fire_lancer(E), E, 2600),
            ("ember_cinder_engine", lambda: ember_cinder_engine(E), E, 2600),
        ],
    }


def building_manifest():
    return {
        "vanguard": [
            ("vanguard_barracks", lambda: b_barracks(V), 6000),
            ("vanguard_range_tower", lambda: b_range_tower(V), 6000),
            ("vanguard_forge", lambda: b_forge(V), 6000),
            ("vanguard_pike_yard", lambda: b_pike_yard(V), 6000),
            ("vanguard_bulwark_hall", lambda: b_bulwark_hall(V), 6000),
            ("vanguard_chapel", lambda: b_chapel(V), 6000),
            ("vanguard_stables", lambda: b_stables(V), 6000),
            ("vanguard_siege_workshop", lambda: b_siege_workshop(V), 6000),
        ],
        "grove": [
            ("grove_root_den", lambda: b_grove_root_den(G), 6000),
            ("grove_thorn_spire", lambda: b_grove_thorn_spire(G), 6000),
            ("grove_bloom_well", lambda: b_grove_bloom_well(G), 6000),
            ("grove_moss_nursery", lambda: b_grove_moss_nursery(G), 6000),
            ("grove_bark_bastion", lambda: b_grove_bark_bastion(G), 6000),
            ("grove_mire_pool", lambda: b_grove_mire_pool(G), 6000),
            ("grove_vine_warren", lambda: b_grove_vine_warren(G), 6000),
            ("grove_ancient_seed", lambda: b_grove_ancient_seed(G), 6000),
        ],
        "ember": [
            ("ember_cinder_pit", lambda: b_ember_cinder_pit(E), 6000),
            ("ember_flame_spire", lambda: b_ember_flame_spire(E), 6000),
            ("ember_ash_mine", lambda: b_ember_ash_mine(E), 6000),
            ("ember_spark_kennel", lambda: b_ember_spark_kennel(E), 6000),
            ("ember_obsidian_gate", lambda: b_ember_obsidian_gate(E), 6000),
            ("ember_blaze_stable", lambda: b_ember_blaze_stable(E), 6000),
            ("ember_smoke_altar", lambda: b_ember_smoke_altar(E), 6000),
            ("ember_inferno_engine", lambda: b_ember_inferno_engine(E), 6000),
        ],
    }


def tree_pine():
    b = cl.Builder()
    b.tapered_cyl((0, 0, 14), 3.6, 2.2, 28, G["bark"], verts=7, smooth=True)
    for i, (z, r) in enumerate(((26, 16), (36, 12.5), (45, 9), (52, 5.5))):
        b.cone((0, 0, z), r, 14,
               G["leaf"] if i % 2 == 0 else G["leaf_bright"], verts=8,
               smooth=True)
    return b


def tree_round():
    b = cl.Builder()
    b.tapered_cyl((0, 0, 12), 4, 2.8, 24, G["bark_dark"], verts=7, smooth=True)
    b.sphere((0, 0, 36), 15, G["leaf"], segments=9, ring_count=6)
    b.sphere((9, 3, 30), 9, G["leaf_bright"], segments=8, ring_count=5)
    b.sphere((-8, -3, 31), 8, G["leaf_bright"], segments=8, ring_count=5)
    return b


def rock_boulder():
    b = cl.Builder()
    b.icosa((0, 0, 7), 10, V["stone_dark"], rot=(0.3, 0.5, 0.2))
    b.icosa((7, 4, 14), 6, V["stone"], rot=(0.8, 0.2, 0.6))
    return b


def doodad_manifest():
    return {
        "doodads": [
            ("tree_pine", tree_pine, 400),
            ("tree_round", tree_round, 400),
            ("rock_boulder", rock_boulder, 400),
        ]
    }


def castle_manifest():
    return {
        "vanguard": [("vanguard_castle", vanguard_castle, 12000)],
        "grove": [("grove_castle", grove_castle, 12000)],
        "ember": [("ember_castle", ember_castle, 12000)],
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
    do_all = False
    for i, a in enumerate(argv):
        if a == "--kind":
            kind = argv[i + 1]
        if a == "--all":
            do_all = True
    if do_all:
        for name in sorted(BUILDERS):
            fn, out, budget = BUILDERS[name]
            cl.clear_scene()
            b = fn()
            cl.finish_and_export(b, name, out, budget)
        return
    if not kind:
        print("usage: --kind <name> | --all  (one of: ",
              ", ".join(sorted(BUILDERS)), ")")
        return
    if kind not in BUILDERS:
        print(f"unknown kind {kind}")
        return
    fn, out, budget = BUILDERS[kind]
    cl.clear_scene()
    b = fn()
    cl.finish_and_export(b, kind, out, budget)


main()
