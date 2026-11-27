#!/usr/bin/env python3
"""Build the sprite sheet, its atlas and the parallax background layers.

    python3 tools/make_sprites.py          (needs Pillow)

Writes assets/img/sprites.png, assets/data/sprites.json and
assets/img/bg_*.png. Every sprite is a horizontal strip of equal frames; the
atlas records where each strip is. All vehicles and animals face right.

The player's vehicles, animals and scenery are drawn by this script. Traffic,
the hitchhiker, the blue sign, the deer, the rabbit and the backgrounds come
from the CC0 art in assets/vendor (see CREDITS.md). The elk and the pronghorn
are repainted from the deer.
"""
import json
import math
import random
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter, ImageOps

ROOT = Path(__file__).resolve().parent.parent
CELL_W, CELL_H, MAX_FRAMES = 48, 32, 4
WIDE_W = 64  # cell width for the longer downloaded vehicles
VENDOR = ROOT / "assets/vendor"
OUT = (24, 18, 14, 255)  # outline
GLASS = (170, 214, 232, 255)
TYRE = (28, 26, 26, 255)
HUB = (196, 196, 200, 255)


def wheel(d, cx, cy, frame, r=5):
    d.ellipse([cx - r, cy - r, cx + r, cy + r], fill=TYRE)
    d.ellipse([cx - 2, cy - 2, cx + 2, cy + 2], fill=HUB)
    # One spoke that turns an eighth of a circle per frame.
    a = frame * math.pi / 4
    for sign in (1, -1):
        x = cx + sign * round(math.cos(a) * (r - 2))
        y = cy + sign * round(math.sin(a) * (r - 2))
        d.line([cx, cy, x, y], fill=HUB)


def pickup(d, frame, body, door, rust=False, stack=False):
    b = frame % 2  # the body bounces on its springs
    # bed, cab, hood
    d.rectangle([3, 15 + b, 20, 24 + b], fill=body, outline=OUT)
    d.polygon([(20, 24 + b), (20, 7 + b), (31, 7 + b), (35, 15 + b), (44, 16 + b),
               (44, 24 + b)], fill=body, outline=OUT)
    d.rectangle([22, 15 + b, 31, 23 + b], fill=door, outline=OUT)
    d.polygon([(22, 14 + b), (22, 9 + b), (30, 9 + b), (33, 14 + b)], fill=GLASS, outline=OUT)
    d.line([24, 17 + b, 26, 17 + b], fill=OUT)  # handle
    d.rectangle([43, 18 + b, 45, 20 + b], fill=(255, 232, 130, 255))  # headlight
    d.rectangle([2, 17 + b, 3, 20 + b], fill=(214, 48, 38, 255))      # tail light
    d.rectangle([43, 23 + b, 46, 25 + b], fill=HUB, outline=OUT)      # bumper
    if rust:
        for x, y in [(6, 18), (7, 18), (7, 19), (14, 22), (15, 22), (15, 21), (37, 19),
                     (38, 19), (38, 20), (21, 9), (27, 22), (11, 16)]:
            d.point((x, y + b), fill=(112, 52, 22, 255))
        d.rectangle([5, 12 + b, 12, 14 + b], fill=(130, 96, 60, 255), outline=OUT)  # junk
    if stack:
        d.rectangle([17, 3 + b, 19, 15 + b], fill=HUB, outline=OUT)
        puff = frame % 4
        d.ellipse([14 - puff * 3, max(0, 2 - puff), 18 - puff * 2, 5],
                  fill=(90, 90, 96, 220 - puff * 40))
        d.line([6, 19 + b, 17, 19 + b], fill=(210, 130, 40, 255))  # pinstripe
    for cx in (11, 37):
        wheel(d, cx, 26, frame)


def prius(d, frame):
    b = frame % 2 + 2  # sits lower than the pickups, tyres on the same road
    body = (150, 186, 204, 255)
    # the unmistakable wedge hatchback
    d.polygon([(3, 24 + b), (3, 15 + b), (8, 9 + b), (19, 7 + b), (29, 9 + b),
               (37, 15 + b), (44, 18 + b), (45, 24 + b)], fill=body, outline=OUT)
    d.polygon([(8, 14 + b), (11, 10 + b), (19, 9 + b), (19, 14 + b)], fill=GLASS, outline=OUT)
    d.polygon([(21, 9 + b), (28, 10 + b), (34, 15 + b), (21, 14 + b)], fill=GLASS, outline=OUT)
    d.line([20, 15 + b, 20, 23 + b], fill=OUT)
    d.line([23, 17 + b, 25, 17 + b], fill=OUT)
    d.rectangle([42, 19 + b, 44, 20 + b], fill=(255, 240, 170, 255))
    d.rectangle([3, 14 + b, 4, 17 + b], fill=(214, 48, 38, 255))
    # the faded campaign sticker: washed-out blue, a pale red arrow
    d.rectangle([5, 18 + b, 15, 22 + b], fill=(118, 136, 184, 255), outline=(214, 216, 224, 255))
    for x, y in [(8, 19), (8, 20), (8, 21), (11, 19), (11, 20), (11, 21), (9, 20), (10, 20)]:
        d.point((x, y + b), fill=(226, 228, 236, 255))
    for x, y in [(12, 20), (13, 20), (12, 19), (12, 21)]:
        d.point((x, y + b), fill=(206, 128, 128, 255))
    for cx in (11, 37):
        wheel(d, cx, 27, frame, r=4)


def legs(d, xs, top, bottom, frame, colour):
    for i, x in enumerate(xs):
        swing = (1 if (frame + i) % 2 == 0 else -1)
        d.line([x, top, x + swing, bottom], fill=colour, width=2)


def sheep(d, frame):
    wool = (240, 236, 224, 255)
    dark = (44, 36, 32, 255)
    legs(d, [17, 21, 28, 32], 23, 30, frame if frame < 2 else 0, dark)
    for box in ([13, 15, 24, 25], [19, 12, 31, 24], [26, 15, 36, 25], [18, 17, 31, 27]):
        d.ellipse(box, fill=wool, outline=(170, 160, 146, 255))
    d.ellipse([19, 15, 31, 25], fill=wool)
    graze = 5 if frame >= 2 else 0  # frames 2 and 3 have the head down
    d.ellipse([33, 12 + graze, 41, 20 + graze], fill=dark)
    d.point((38, 15 + graze), fill=(255, 255, 255, 255))
    d.ellipse([32, 11 + graze, 36, 16 + graze], outline=(196, 170, 120, 255))  # horn
    if frame == 3:
        d.point((41, 27), fill=(90, 140, 60, 255))


def jackalope(d, frame):
    fur = (158, 128, 100, 255)
    hop = [0, 4, 6, 3][frame]
    y = -hop
    d.ellipse([16, 20 + y, 29, 29 + y], fill=fur, outline=OUT)
    d.ellipse([25, 15 + y, 33, 23 + y], fill=fur, outline=OUT)
    d.ellipse([14, 22 + y, 18, 26 + y], fill=(240, 236, 224, 255))
    d.line([27, 16 + y, 25, 9 + y], fill=fur, width=2)
    # antlers
    antler = (226, 206, 160, 255)
    for x0, x1 in [(29, 31), (31, 35)]:
        d.line([x0, 15 + y, x1, 8 + y], fill=antler)
        d.line([x1 - 1, 11 + y, x1 + 2, 10 + y], fill=antler)
    d.point((31, 18 + y), fill=OUT)
    d.line([18, 29 + y, 22, 30 + y], fill=OUT)


def pine(d, _):
    d.rectangle([22, 25, 25, 31], fill=(92, 62, 40, 255))
    for top, half, h in [(2, 6, 11), (8, 9, 12), (14, 12, 13)]:
        d.polygon([(23, top), (23 - half, top + h), (24 + half, top + h)],
                  fill=(40, 92, 56, 255), outline=(22, 54, 34, 255))


def sage(d, _):
    for x, y, r in [(18, 26, 5), (25, 24, 7), (32, 27, 4)]:
        d.ellipse([x - r, y - r, x + r, y + r], fill=(132, 150, 112, 255))
    for x, y in [(16, 24), (22, 20), (27, 22), (30, 26), (20, 28), (25, 27)]:
        d.point((x, y), fill=(188, 198, 160, 255))


def sign(d, _):
    d.rectangle([23, 17, 24, 31], fill=(110, 100, 90, 255))
    d.polygon([(23, 3), (33, 11), (23, 19), (13, 11)], fill=(244, 196, 40, 255), outline=OUT)
    d.line([19, 13, 27, 9], fill=OUT, width=2)  # a leaping something
    d.line([27, 9, 29, 7], fill=OUT)


def dust(d, frame):
    alpha = 220 - frame * 50
    spread = 3 + frame * 3
    for dx, dy, r in [(0, 0, 3), (-spread, -2, 2), (-spread // 2, -4 - frame, 2)]:
        x, y = 36 + dx, 28 + dy
        d.ellipse([x - r - frame, y - r, x + r, y + r], fill=(214, 192, 150, alpha))


def horse(d, frame):
    """A bay horse at a trot, with a rider who has walked this road before."""
    b = frame % 2
    bay, shade = (142, 86, 46, 255), (104, 60, 32, 255)
    black = (36, 28, 24, 255)

    # Legs first, so the body covers their tops. Diagonal pairs swing together;
    # a lifted hoof is the one swinging forward.
    swing = [3, 1, -3, -1][frame]
    for hip, step, colour in [(13, swing, shade), (29, -swing, shade),
                              (16, -swing, bay), (32, swing, bay)]:
        lift = 2 if step > 1 else 0
        knee = (hip + step // 2, 25 + b)
        foot = (hip + step, 30 - lift)
        d.line([(hip, 20 + b), knee], fill=colour, width=3)
        d.line([knee, foot], fill=black if colour == shade else colour, width=2)
        d.rectangle([foot[0] - 1, foot[1], foot[0] + 1, foot[1] + 1], fill=black)

    # tail, body, neck and head
    sway = [0, 1, 0, -1][frame]
    d.line([(11, 16 + b), (6, 18 + b), (4 + sway, 23 + b), (5 + sway, 27)], fill=black, width=3)
    d.ellipse([9, 13 + b, 35, 24 + b], fill=bay, outline=OUT)
    d.polygon([(27, 17 + b), (35, 6 + b), (40, 8 + b), (35, 21 + b)], fill=bay, outline=OUT)
    d.polygon([(35, 6 + b), (39, 5 + b), (46, 11 + b), (44, 14 + b), (38, 11 + b)],
              fill=bay, outline=OUT)
    d.rectangle([29, 15 + b, 33, 20 + b], fill=bay)  # blend the shoulder
    d.line([(36, 5 + b), (35, 2 + b)], fill=black, width=2)  # ear
    d.line([(35, 6 + b), (31, 11 + b), (28, 15 + b)], fill=black, width=2)  # mane
    d.point((40, 8 + b), fill=OUT)
    d.rectangle([44, 12 + b, 45, 13 + b], fill=shade)
    d.line([(41, 12 + b), (28, 13 + b)], fill=(70, 44, 24, 255))  # reins

    # saddle and blanket
    d.rectangle([16, 13 + b, 27, 16 + b], fill=(172, 44, 40, 255), outline=OUT)
    d.rectangle([18, 12 + b, 25, 14 + b], fill=(188, 142, 84, 255), outline=OUT)

    # rider: jeans, boot, shirt, head and a hat
    d.line([(21, 12 + b), (26, 15 + b), (25, 20 + b)], fill=(58, 82, 138, 255), width=3)
    d.rectangle([24, 20 + b, 28, 21 + b], fill=(96, 58, 28, 255))
    lean = 1 if frame == 3 else 0  # a wobble, for obvious reasons
    d.polygon([(18, 13 + b), (19 + lean, 6 + b), (24 + lean, 6 + b), (24, 13 + b)],
              fill=(74, 118, 84, 255), outline=OUT)
    d.line([(22 + lean, 8 + b), (28, 12 + b)], fill=(74, 118, 84, 255), width=2)
    d.rectangle([20 + lean, 2 + b, 24 + lean, 6 + b], fill=(228, 176, 136, 255), outline=OUT)
    d.line([(16 + lean, 2 + b), (28 + lean, 2 + b)], fill=(110, 74, 42, 255), width=1)
    d.rectangle([19 + lean, b, 25 + lean, 1 + b], fill=(150, 104, 60, 255))


def sprinter(d, frame):
    """A very expensive van that has never been dirty."""
    b = frame % 2
    body = (238, 236, 228, 255)
    d.polygon([(3, 25 + b), (3, 7 + b), (6, 5 + b), (31, 5 + b), (38, 8 + b), (44, 17 + b),
               (45, 25 + b)], fill=body, outline=OUT)
    d.rectangle([7, 2 + b, 27, 4 + b], fill=(44, 62, 110, 255), outline=OUT)  # solar
    for x in (11, 15, 19, 23):
        d.line([x, 2 + b, x, 4 + b], fill=(110, 140, 200, 255))
    d.polygon([(30, 8 + b), (36, 9 + b), (41, 16 + b), (30, 16 + b)], fill=GLASS, outline=OUT)
    d.rectangle([19, 9 + b, 27, 14 + b], fill=GLASS, outline=OUT)
    d.ellipse([7, 10 + b, 15, 18 + b], outline=(126, 76, 150, 255))  # mandala
    for dx, dy in [(0, -4), (0, 4), (-4, 0), (4, 0)]:
        d.line([11, 14 + b, 11 + dx, 14 + b + dy], fill=(220, 140, 60, 255))
    d.rectangle([3, 20 + b, 44, 21 + b], fill=(196, 150, 90, 255))  # wood trim
    d.rectangle([43, 19 + b, 45, 20 + b], fill=(255, 236, 150, 255))
    d.rectangle([2, 15 + b, 3, 19 + b], fill=(214, 48, 38, 255))
    for cx in (11, 36):
        wheel(d, cx, 26, frame)


def goddess(d, frame):
    """The Groove Goddess of Gaia, up from the river and nodding along."""
    sway = [0, 1, 0, -1][frame]
    for r, a in [(22, 36), (17, 60), (12, 90)]:
        d.ellipse([24 - r, 26 - r, 24 + r, 26 + r], fill=(150, 255, 190, a))
    d.polygon([(15, 60), (21 + sway, 28), (27 + sway, 28), (33, 60)], fill=(70, 190, 130, 255), outline=(26, 90, 60, 255))
    d.line([(21 + sway, 31), (9, 22 - sway * 2)], fill=(210, 250, 220, 255), width=3)
    d.line([(27 + sway, 31), (39, 22 + sway * 2)], fill=(210, 250, 220, 255), width=3)
    d.ellipse([19 + sway, 16, 29 + sway, 28], fill=(214, 250, 220, 255), outline=(40, 110, 76, 255))
    d.point((22 + sway, 21), fill=OUT)
    d.point((26 + sway, 21), fill=OUT)
    d.arc([21 + sway, 21, 27 + sway, 26], 20, 160, fill=OUT)
    for dx, c in [(-6, (250, 214, 60)), (-2, (240, 120, 150)), (2, (250, 244, 230)), (6, (250, 214, 60))]:
        d.ellipse([22 + sway + dx, 12 + abs(dx) // 3, 26 + sway + dx, 16 + abs(dx) // 3], fill=(*c, 255))
    d.line([(12, 60), (36, 60)], fill=(170, 220, 255, 255), width=3)


def vint_blanket(d, _):
    d.rectangle([4, 8, 28, 28], fill=(182, 70, 50, 255), outline=OUT)
    for y, c in [(12, (236, 214, 160)), (17, (40, 80, 110)), (22, (236, 214, 160))]:
        d.line([5, y, 27, y], fill=(*c, 255), width=2)
    for x in range(6, 28, 3):
        d.line([x, 29, x, 31], fill=(236, 214, 160, 255))


def vint_phone(d, _):
    d.polygon([(6, 30), (9, 16), (23, 16), (26, 30)], fill=(42, 42, 48, 255), outline=OUT)
    d.ellipse([11, 18, 21, 28], fill=(220, 216, 200, 255), outline=OUT)
    d.ellipse([14, 21, 18, 25], fill=(42, 42, 48, 255))
    d.arc([5, 6, 27, 22], 180, 360, fill=(70, 70, 80, 255), width=3)


def vint_antlers(d, _):
    d.line([(16, 2), (16, 12)], fill=(90, 70, 50, 255))
    for side in (-1, 1):
        d.line([(16, 18), (16 + side * 12, 12), (16 + side * 14, 5)], fill=(228, 210, 170, 255), width=2)
        d.line([(16 + side * 8, 14), (16 + side * 8, 7)], fill=(228, 210, 170, 255), width=2)
        d.rectangle([16 + side * 12 - 1, 14, 16 + side * 12 + 1, 18], fill=(255, 220, 120, 255))
    d.ellipse([13, 17, 19, 23], fill=(120, 90, 60, 255), outline=OUT)


def vint_saddle(d, _):
    d.polygon([(5, 14), (11, 10), (15, 16), (24, 12), (28, 18), (22, 24), (8, 22)],
              fill=(150, 90, 44, 255), outline=OUT)
    d.line([(13, 22), (12, 30)], fill=(110, 66, 34, 255), width=2)
    d.ellipse([9, 27, 15, 31], outline=(196, 196, 200, 255))
    d.arc([10, 14, 24, 24], 20, 160, fill=(210, 160, 90, 255))


def vint_jukebox(d, _):
    d.pieslice([5, 1, 27, 23], 180, 360, fill=(190, 60, 60, 255), outline=OUT)
    d.rectangle([5, 12, 27, 30], fill=(190, 60, 60, 255), outline=OUT)
    d.pieslice([9, 5, 23, 19], 180, 360, fill=(250, 220, 120, 255), outline=OUT)
    d.rectangle([9, 14, 23, 20], fill=(70, 200, 200, 255), outline=OUT)
    for x in range(10, 23, 3):
        d.line([x, 22, x, 28], fill=(240, 220, 180, 255))


VINTAGE = [vint_blanket, vint_phone, vint_antlers, vint_saddle, vint_jukebox]


# --- seasonal scenery -----------------------------------------------------------

SNOW = (244, 248, 253, 255)


def pine_snow(d, _):
    pine(d, 0)
    for top, half, h in [(2, 6, 11), (8, 9, 12), (14, 12, 13)]:
        mid = top + h // 2
        d.polygon([(23, top), (23 - half // 2, mid), (21, mid - 2), (24, mid), (24 + half // 2, mid)],
                  fill=SNOW)
        d.line([23 - half + 2, top + h - 1, 24 + half - 2, top + h - 1], fill=SNOW)


def sage_snow(d, _):
    for x, y, r in [(18, 26, 5), (25, 24, 7), (32, 27, 4)]:
        d.ellipse([x - r, y - r, x + r, y + r], fill=(128, 128, 116, 255))
    for x, y, r in [(18, 23, 4), (25, 20, 6), (32, 25, 3)]:
        d.chord([x - r, y - r, x + r, y + r], 180, 360, fill=SNOW)


def flowers(d, _):
    """A patch of spring wildflowers."""
    for x, y, petal in [(15, 26, (252, 208, 40)), (19, 28, (170, 110, 214)), (23, 25, (246, 244, 236)),
                        (28, 28, (236, 92, 70)), (32, 26, (252, 208, 40)), (35, 29, (170, 110, 214))]:
        d.line([x, y, x, 31], fill=(74, 128, 52, 255))
        d.ellipse([x - 1, y - 2, x + 1, y], fill=(*petal, 255))
    d.line([13, 31, 37, 31], fill=(96, 150, 60, 255))


def snow_puff(d, frame):
    alpha = 235 - frame * 50
    spread = 3 + frame * 3
    for dx, dy, r in [(0, 0, 3), (-spread, -2, 2), (-spread // 2, -4 - frame, 2)]:
        x, y = 36 + dx, 28 + dy
        d.ellipse([x - r - frame, y - r, x + r, y + r], fill=(240, 246, 255, alpha))


SPRITES = [
    ("pickup_rusty", 4, lambda d, f: pickup(d, f, (172, 98, 48, 255), (150, 118, 86, 255), rust=True)),
    ("halfton", 4, lambda d, f: pickup(d, f, (52, 98, 164, 255), (178, 52, 44, 255))),
    ("diesel", 4, lambda d, f: pickup(d, f, (54, 56, 62, 255), (66, 68, 76, 255), stack=True)),
    ("prius", 4, prius),
    ("horse", 4, horse),
    ("sprinter", 4, sprinter),
    ("sheep", 4, sheep),
    ("jackalope", 4, jackalope),
    ("pine", 1, pine),
    ("sage", 1, sage),
    ("sign", 1, sign),
    ("dust", 4, dust),
    ("pine_snow", 1, pine_snow),
    ("sage_snow", 1, sage_snow),
    ("flowers", 1, flowers),
    ("snow_puff", 4, snow_puff),
]


# (name, source files in assets/vendor/kenney, one per frame)
DOWNLOADED = [(n, [n]) for n in ("police", "hotdog", "bus_school", "truck", "tractor",
                                 "towtruck", "suv_travel", "vendor", "firetruck")]
DOWNLOADED += [("hiker", ["man_walk1", "man_walk2"]), ("sign_blue", ["sign_blue"])]


# --- wild plants (drawn here, modelled on the real species) -------------------
#
# Each is drawn for what tells it apart in the field: leaf shape, habit and the
# part you would pick. References: the photos in assets/img/plant_*.jpg and the
# species' Wikipedia descriptions.

PLANT = 56  # cell size


def rnd(seed):
    """A repeatable scatter, so the sheet is the same on every build."""
    return random.Random(seed)


def at(x, y, angle, distance):
    a = math.radians(angle)
    return (x + distance * math.cos(a), y - distance * math.sin(a))


def blade(d, base, angle, length, width, fill, rib=None, droop=0.0):
    """A leaf from `base` toward `angle` degrees (90 is straight up). `droop`
    bends the outer half downward."""
    left, right, spine = [], [], []
    for i in range(9):
        t = i / 8
        x, y = at(base[0], base[1], angle, length * t)
        y += droop * t * t * length
        half = width / 2 * math.sin(math.pi * min(0.97, 0.12 + t * 0.88))
        nx, ny = at(0, 0, angle + 90, half)
        left.append((x + nx, y + ny))
        right.append((x - nx, y - ny))
        spine.append((x, y))
    d.polygon(left + right[::-1], fill=fill)
    if rib:
        d.line(spine[:-1], fill=rib, width=1)


def dot(d, x, y, r, fill, outline=None):
    d.ellipse([x - r, y - r, x + r, y + r], fill=fill, outline=outline)


def shrub_frame(d, seed, bark, leaves, leaf_len, leaf_wide, count):
    """Several stems from one root crown, clothed in leaves."""
    r = rnd(seed)
    tips = [(14, 24), (22, 16), (34, 15), (42, 23), (18, 33), (39, 34)]
    for tx, ty in tips:
        d.line([(28, 54), ((28 + tx) // 2 + r.randint(-3, 3), (54 + ty) // 2), (tx, ty)],
               fill=bark, width=2)
    for i in range(count):
        tx, ty = tips[i % len(tips)]
        base = (tx + r.randint(-5, 5), ty + r.randint(-2, 9))
        angle = r.randint(-30, 210)
        colour = leaves[i % len(leaves)]
        blade(d, base, angle, leaf_len + r.randint(-1, 2), leaf_wide, colour,
              rib=(150, 184, 112, 255))


def chokecherry(d, _):
    # Pointed oval leaves; fruit hangs in long, drooping strings, dark red to black.
    shrub_frame(d, 1, (104, 70, 50, 255),
                [(52, 104, 44, 255), (68, 124, 50, 255), (88, 142, 58, 255)], 10, 6, 46)
    r = rnd(2)
    for sx, sy in [(14, 25), (24, 18), (38, 21), (43, 30), (21, 32), (32, 31)]:
        d.line([sx, sy - 2, sx + 1, sy + 14], fill=(120, 82, 50, 255))
        for k in range(7):
            x = sx + (-2 if k % 2 else 2) + r.randint(-1, 1)
            y = sy + k * 2
            ripe = [(122, 14, 36, 255), (82, 10, 34, 255), (46, 8, 24, 255)][r.randrange(3)]
            dot(d, x, y, 1.6, ripe)
            d.point((x - 1, y - 1), fill=(222, 120, 130, 255))


def serviceberry(d, _):
    # Nearly round leaves toothed toward the tip; loose clusters of purple-blue
    # fruit with a pale waxy bloom and a little crown.
    shrub_frame(d, 3, (128, 118, 112, 255),
                [(86, 128, 74, 255), (108, 148, 90, 255), (128, 164, 102, 255)], 7, 7, 50)
    r = rnd(4)
    for cx, cy in [(16, 22), (31, 17), (41, 26), (22, 31), (35, 37), (13, 35)]:
        for k, (dx, dy) in enumerate([(0, 0), (4, 2), (-3, 3), (1, 5), (5, -2)]):
            x, y = cx + dx, cy + dy
            d.line([cx + 1, cy - 4, x, y], fill=(112, 76, 60, 255))
            dot(d, x, y, 2, [(96, 66, 138, 255), (70, 58, 124, 255), (128, 68, 120, 255)][r.randrange(3)])
            d.point((x - 1, y - 1), fill=(186, 188, 226, 255))
            d.point((x, y + 2), fill=(40, 24, 48, 255))


def pricklypear(d, _):
    # A low, wide mat of flat oval pads set with spines; yellow cup flowers that
    # age toward orange.
    pads = [(13, 45, 11, 8, -1), (42, 46, 12, 8, 1), (28, 49, 10, 6, 0),
            (21, 34, 8, 10, -1), (37, 34, 9, 11, 1), (48, 32, 6, 8, 1), (7, 33, 5, 7, -1)]
    for x, y, rx, ry, _ in pads:
        d.ellipse([x - rx, y - ry, x + rx, y + ry], fill=(106, 140, 98, 255), outline=(48, 76, 48, 255))
        d.arc([x - rx + 2, y - ry + 2, x + rx - 2, y + ry - 2], 180, 290, fill=(150, 180, 130, 255))
        for row, dy in enumerate(range(-ry + 3, ry - 1, 4)):
            for dx in range(-rx + 3 + 2 * (row % 2), rx - 2, 5):
                px, py = x + dx, y + dy
                d.point((px, py), fill=(124, 88, 56, 255))
                for ex, ey in [(-2, -2), (2, -2), (0, -3)]:
                    d.line([px, py, px + ex, py + ey], fill=(236, 230, 206, 255))
    for x, y, old in [(19, 22, False), (36, 20, False), (48, 22, True)]:
        petal = (240, 150, 40, 255) if old else (250, 214, 48, 255)
        d.rectangle([x - 2, y + 2, x + 2, y + 5], fill=(116, 124, 70, 255))
        for a in (30, 62, 90, 118, 150):
            blade(d, (x, y + 3), a, 8, 5, petal)
        dot(d, x, y, 2, (232, 132, 24, 255))
        d.point((x, y - 1), fill=(150, 186, 80, 255))
    for x, y in [(7, 24), (28, 27)]:  # buds
        blade(d, (x, y + 3), 90, 6, 4, (178, 82, 66, 255))


def balsamroot(d, _):
    # Big silvery-green arrowhead leaves from the base; bare hairy stems each
    # carrying a yellow sunflower with a yellow-orange middle.
    silver, pale, rib = (128, 156, 120, 255), (156, 178, 144, 255), (206, 216, 190, 255)
    for angle, length, colour in [(168, 22, silver), (12, 22, silver), (138, 24, pale),
                                  (42, 24, pale), (108, 22, silver), (74, 21, pale)]:
        base = (28, 53)
        sx, sy = at(base[0], base[1], angle, 7)
        d.line([base, (sx, sy)], fill=silver, width=2)
        tip = at(sx, sy, angle, length - 5)
        wing_l, wing_r = at(sx, sy, angle + 112, 7), at(sx, sy, angle - 112, 7)
        mid_l = at(*at(sx, sy, angle, 7), angle + 90, 5)
        mid_r = at(*at(sx, sy, angle, 7), angle - 90, 5)
        d.polygon([wing_l, mid_l, tip, mid_r, wing_r, (sx, sy)], fill=colour, outline=(82, 108, 78, 255))
        d.line([(sx, sy), at(sx, sy, angle, length - 8)], fill=rib)
    for hx, hy, size in [(15, 19, 9), (37, 14, 10), (29, 31, 8), (44, 31, 7)]:
        d.line([(28, 52), ((28 + hx) // 2, (52 + hy) // 2 + 3), (hx, hy)], fill=(122, 148, 90, 255), width=2)
        for k in range(13):
            blade(d, (hx, hy), k * 360 / 13 + hx, size, 4,
                  (252, 208, 24, 255) if k % 2 else (244, 188, 16, 255))
        dot(d, hx, hy, 3, (226, 150, 20, 255))
        for dx, dy in [(-1, -1), (1, 0), (0, 1)]:
            d.point((hx + dx, hy + dy), fill=(250, 196, 60, 255))


def wild_onion(d, _):
    # Thread-thin round leaves; bare stalks ending in a round head of small
    # cream bells with reddish-brown midribs; an egg-shaped bulb.
    leaf = (74, 112, 66, 255)
    for angle, length in [(112, 36), (98, 44), (64, 30), (128, 22)]:
        mid = at(28, 49, angle, length * 0.6)
        d.line([(28, 49), mid, at(mid[0], mid[1], angle - 14, length * 0.4)], fill=leaf, width=1)
    for hx, hy, bend in [(18, 17, -3), (38, 24, 3)]:
        d.line([(28, 49), (28 + (hx - 28) // 2 + bend, (49 + hy) // 2), (hx, hy + 4)],
               fill=(104, 140, 84, 255), width=2)
        cy = hy + 1
        for ring, count, length in [(6.5, 11, 4), (3.5, 7, 4)]:
            for k in range(count):
                a = k * 360 / count + hx
                fx, fy = at(hx, cy, a, ring)
                d.line([(hx, hy + 4), (fx, fy)], fill=(130, 150, 96, 255))
                dot(d, fx, fy, 2, (244, 234, 214, 255), (196, 178, 150, 255))
                d.line([(fx, fy), at(fx, fy, a, length - 2)], fill=(158, 96, 78, 255))
        for dx, dy in [(-2, -1), (2, -2), (0, 1)]:
            d.point((hx + dx, cy + dy), fill=(238, 200, 60, 255))
    d.ellipse([23, 45, 33, 55], fill=(182, 148, 112, 255), outline=(112, 84, 60, 255))
    d.arc([25, 46, 31, 54], 90, 270, fill=(134, 102, 74, 255))


def sego_lily(d, _):
    # One large cup of three broad white petals, narrow sepals between them, and
    # a yellow throat ringed with maroon crescents; a few grassy leaves.
    stem = (92, 128, 72, 255)
    d.line([(28, 54), (30, 41), (28, 28)], fill=stem, width=2)
    blade(d, (29, 52), 122, 24, 3, stem)
    blade(d, (29, 46), 62, 20, 3, stem)
    cx, cy = 28, 22
    for a in (30, 150, 270):  # sepals
        blade(d, (cx, cy), a, 18, 6, (206, 212, 160, 255), rib=(150, 120, 120, 255))
    for a in (90, 210, 330):  # petals: narrow at the base, broad and round at the end
        outer = [at(cx, cy, a - 40, 13), at(cx, cy, a - 24, 19), at(cx, cy, a, 21),
                 at(cx, cy, a + 24, 19), at(cx, cy, a + 40, 13)]
        d.polygon([(cx, cy)] + outer, fill=(248, 246, 238, 255), outline=(188, 172, 190, 255))
        d.line([at(cx, cy, a, 9), at(cx, cy, a, 17)], fill=(226, 214, 232, 255))
    for a in (90, 210, 330):
        sx, sy = at(cx, cy, a, 7)
        dot(d, sx, sy, 3, (244, 196, 36, 255))
        d.arc([sx - 4, sy - 4, sx + 4, sy + 4], -a - 62, -a + 62, fill=(136, 28, 64, 255), width=1)
    dot(d, cx, cy, 3, (250, 220, 90, 255))
    for a in range(0, 360, 60):
        d.line([(cx, cy), at(cx, cy, a, 4)], fill=(240, 232, 160, 255))


def cattail(d, _):
    # Tall flat blades; the brown velvet sausage with its thin spike on top.
    greens = [(96, 138, 62, 255), (122, 160, 72, 255), (162, 176, 94, 255)]
    for i, (bx, angle, length, droop) in enumerate([
            (18, 104, 50, 0.05), (24, 96, 46, 0), (30, 82, 52, 0.04), (36, 70, 40, 0.14),
            (22, 124, 32, 0.22), (34, 56, 28, 0.26)]):
        blade(d, (bx, 55), angle, length, 4, greens[i % 3], droop=droop)
    for x, top in [(21, 11), (37, 17)]:
        d.line([(x, 55), (x, top)], fill=(120, 132, 72, 255), width=2)
        d.line([(x, top), (x, top - 9)], fill=(196, 170, 110, 255))
        d.rounded_rectangle([x - 3, top, x + 3, top + 19], radius=3,
                            fill=(126, 62, 28, 255), outline=(72, 34, 18, 255))
        d.line([(x - 1, top + 3), (x - 1, top + 15)], fill=(166, 92, 46, 255))


def death_camas(d, _):
    # The trap: grass-like leaves folded into a V, and one tall stalk with a
    # dense, tapering spike of small cream star flowers.
    for angle, length, droop in [(150, 24, 0.30), (30, 25, 0.30), (126, 30, 0.22),
                                 (54, 28, 0.24), (112, 22, 0.05), (68, 20, 0.08)]:
        blade(d, (28, 55), angle, length, 4, (86, 128, 70, 255), rib=(170, 196, 130, 255), droop=droop)
    d.line([(28, 55), (28, 6)], fill=(118, 150, 82, 255), width=2)
    blade(d, (28, 36), 66, 10, 2, (86, 128, 70, 255))
    r = rnd(8)
    for i, y in enumerate(range(8, 31, 2)):
        spread = 2 + i * 0.6
        spots = [-spread, 0, spread] if i % 2 else [-spread * 0.55, spread * 0.55]
        for off in spots:
            x = 28 + round(off) + r.randint(0, 1) - (1 if off < 0 else 0)
            d.line([(28, y + 2), (x, y)], fill=(150, 170, 100, 255))
            for dx, dy in [(0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)]:
                d.point((x + dx, y + dy), fill=(250, 244, 196, 255))
            d.point((x + 1, y + 1), fill=(222, 212, 148, 255))
            d.point((x, y), fill=(214, 184, 60, 255))
    dot(d, 28, 5, 1.5, (216, 220, 150, 255))  # buds at the tip


# --- what turns up behind a grocery store in winter ------------------------------

def bagels(d, _):
    d.polygon([(8, 36), (48, 36), (44, 54), (12, 54)], fill=(176, 132, 84, 255), outline=(96, 66, 38, 255))
    for x, y in [(18, 32), (32, 29), (40, 34), (25, 37)]:
        d.ellipse([x - 9, y - 7, x + 9, y + 7], fill=(214, 156, 78, 255), outline=(128, 82, 36, 255))
        d.ellipse([x - 3, y - 2, x + 3, y + 2], fill=(110, 72, 36, 255))
        for sx, sy in [(-5, -3), (4, -4), (6, 2)]:
            d.point((x + sx, y + sy), fill=(248, 232, 196, 255))


def cans(d, _):
    for x, y, label in [(10, 30, (188, 52, 44)), (24, 22, (52, 110, 170)), (36, 32, (86, 140, 60))]:
        d.rectangle([x, y, x + 13, y + 22], fill=(178, 182, 188, 255), outline=(80, 84, 92, 255))
        d.rectangle([x + 1, y + 6, x + 12, y + 16], fill=(*label, 255))
        d.ellipse([x, y - 3, x + 13, y + 3], fill=(206, 210, 214, 255), outline=(80, 84, 92, 255))
        d.line([x + 3, y + 8, x + 6, y + 14], fill=(60, 60, 66, 255))  # the dent


def produce(d, _):
    d.rectangle([8, 34, 48, 54], fill=(150, 110, 70, 255), outline=(82, 56, 32, 255))
    d.line([8, 44, 48, 44], fill=(82, 56, 32, 255))
    for x, y, r, c in [(17, 31, 7, (196, 60, 44)), (29, 27, 8, (230, 140, 40)),
                       (40, 31, 7, (120, 160, 50)), (24, 35, 5, (214, 190, 60))]:
        d.ellipse([x - r, y - r, x + r, y + r], fill=(*c, 255), outline=(70, 46, 22, 255))
        d.ellipse([x + 1, y + 1, x + 3, y + 3], fill=(110, 80, 40, 255))  # bruise
    blade(d, (29, 20), 60, 9, 4, (86, 128, 60, 255))


def pizza(d, _):
    d.polygon([(6, 40), (34, 30), (50, 40), (22, 52)], fill=(190, 150, 104, 255), outline=(96, 70, 44, 255))
    d.polygon([(10, 39), (33, 31), (44, 41)], fill=(236, 190, 80, 255), outline=(176, 104, 40, 255))
    for x, y in [(22, 36), (31, 35), (35, 39)]:
        d.ellipse([x - 2, y - 2, x + 2, y + 2], fill=(176, 44, 34, 255))
    d.line([10, 39, 44, 41], fill=(196, 128, 52, 255), width=3)


def burrito(d, _):
    d.ellipse([8, 30, 48, 48], fill=(206, 214, 220, 255), outline=(96, 104, 116, 255))
    d.ellipse([12, 28, 34, 42], fill=(226, 196, 132, 255), outline=(130, 100, 56, 255))
    for x, y in [(18, 33), (24, 37), (29, 32), (15, 37)]:
        d.ellipse([x - 2, y - 2, x + 2, y + 1], fill=(104, 150, 72, 255))  # it's growing
    for x in (20, 30, 40):
        d.line([(x, 26), (x - 3, 20), (x, 15)], fill=(150, 190, 90, 255))  # the smell


def sock(d, _):
    d.polygon([(18, 22), (30, 22), (30, 40), (44, 44), (42, 54), (24, 52), (18, 44)],
              fill=(206, 202, 188, 255), outline=(110, 104, 90, 255))
    d.rectangle([18, 22, 30, 27], fill=(150, 60, 52, 255), outline=(110, 104, 90, 255))
    d.ellipse([31, 45, 37, 50], fill=(118, 96, 66, 255))  # the stain
    d.ellipse([22, 33, 25, 36], fill=(60, 52, 44, 255))  # the hole


def trash_bag(d, _):
    d.polygon([(10, 54), (8, 40), (16, 28), (24, 24), (22, 16), (28, 20), (34, 15), (33, 24),
               (42, 29), (49, 41), (47, 54)], fill=(44, 46, 54, 255), outline=(14, 14, 18, 255))
    d.line([(23, 24), (34, 24)], fill=(190, 170, 60, 255), width=2)  # the tie
    for a, b in [((15, 44), (20, 33)), ((30, 50), (36, 34)), ((40, 46), (44, 38))]:
        d.line([a, b], fill=(92, 98, 112, 255), width=2)  # the shine


def box_pile(d, _):
    d.rectangle([8, 32, 40, 54], fill=(168, 124, 76, 255), outline=(86, 58, 30, 255))
    d.polygon([(8, 32), (14, 24), (24, 30), (22, 32)], fill=(190, 148, 96, 255), outline=(86, 58, 30, 255))
    d.polygon([(40, 32), (35, 23), (26, 31)], fill=(150, 108, 66, 255), outline=(86, 58, 30, 255))
    d.rectangle([22, 40, 49, 54], fill=(188, 146, 94, 255), outline=(86, 58, 30, 255))
    d.line([(35, 40), (35, 54)], fill=(206, 190, 150, 255), width=3)  # tape
    d.line([(12, 42), (18, 42)], fill=(86, 58, 30, 255))


def paper_heap(d, _):
    r = rnd(61)
    for _i in range(16):
        x, y = r.randint(8, 40), r.randint(30, 48)
        shade = r.choice([(232, 228, 214), (204, 200, 186), (214, 196, 160), (180, 190, 200)])
        d.polygon([(x, y), (x + r.randint(6, 12), y - r.randint(0, 6)),
                   (x + r.randint(8, 14), y + r.randint(2, 8)), (x + r.randint(0, 5), y + r.randint(4, 9))],
                  fill=(*shade, 255), outline=(120, 112, 96, 255))
    d.line([(16, 40), (28, 38)], fill=(90, 90, 96, 255))


SCRAPS = [bagels, cans, produce, pizza, burrito, sock, trash_bag, box_pile, paper_heap]
PLANTS = [chokecherry, serviceberry, pricklypear, balsamroot, wild_onion, death_camas, sego_lily, cattail]


def plant_cell(draw):
    """One plant, edged in dark so it reads against a photograph."""
    art = Image.new("RGBA", (PLANT, PLANT), (0, 0, 0, 0))
    draw(ImageDraw.Draw(art), 0)
    solid = art.getchannel("A").point(lambda a: 255 if a > 0 else 0)
    edge = Image.new("RGBA", art.size, (22, 26, 14, 210))
    edge.putalpha(solid.filter(ImageFilter.MaxFilter(3)).point(lambda a: 210 if a else 0))
    return Image.alpha_composite(edge, art)


# --- the player characters (Antifarea, CC-BY 3.0) -----------------------------

HEROES = ["healer", "mage", "ninja", "ranger", "townfolk", "warrior"]
HERO_W, HERO_H = 32, 36


def heroes():
    """Three walking frames for each character, sex and direction."""
    out = []
    for kind in HEROES:
        for sex in "mf":
            art = Image.open(VENDOR / "antifarea" / f"{kind}_{sex}.png").convert("RGBA")
            for row, way in enumerate(["up", "right", "down", "left"]):
                frames = [art.crop((c * HERO_W, row * HERO_H, (c + 1) * HERO_W, (row + 1) * HERO_H))
                          for c in range(3)]
                out.append((f"hero_{kind}_{sex}_{way}", HERO_W, HERO_H, frames))
    return out + cowboy() + trustafarians()


def dreadlocks(frame, way, seed):
    """Hang locks from the head of a walking character."""
    art = frame.copy()
    box = art.getbbox()
    if not box:
        return art
    x0, y0, x1, _ = box
    d = ImageDraw.Draw(art)
    r = rnd(seed)
    mid = (x0 + x1) // 2
    roots = {"down": [-8, -6, 6, 8], "up": [-7, -4, -1, 2, 5, 8],
             "left": [1, 4, 6, 8], "right": [-9, -7, -5, -2]}[way]
    for i, dx in enumerate(roots):
        x, y = mid + dx, y0 + 5 + (i % 2)
        shade = [(104, 66, 34), (136, 92, 46)][i % 2]
        for step in range(5 + r.randint(0, 2)):
            px = x + [0, 0, 1, 0, -1, 0, 0, 1][(step + i) % 8]
            d.rectangle([px, y + step * 2, px + 1, y + step * 2 + 1], fill=(*shade, 255))
        d.point((x, y + 8), fill=(240, 200, 70, 255))  # a bead
    # a headband
    d.line([(mid - 6, y0 + 5), (mid + 6, y0 + 5)], fill=(70, 160, 130, 255))
    return art


def trustafarians():
    """Antifarea's mage, repainted with dreadlocks and a headband."""
    out = []
    for sex in "mf":
        art = Image.open(VENDOR / "antifarea" / f"mage_{sex}.png").convert("RGBA")
        for row, way in enumerate(["up", "right", "down", "left"]):
            frames = [dreadlocks(art.crop((c * HERO_W, row * HERO_H, (c + 1) * HERO_W, (row + 1) * HERO_H)),
                                 way, 70 + row) for c in range(3)]
            out.append((f"hero_trust_{sex}_{way}", HERO_W, HERO_H, frames))
    return out


def cowboy():
    """Wasilev's rifleman: 48px cells, each direction a run of three frames
    (aiming, then two steps)."""
    art = Image.open(VENDOR / "wasilev/cowboy_rifle.png").convert("RGBA")
    # (row, first column) of each direction used.
    where = {"down": (0, 0), "left": (1, 0), "up": (2, 0), "right": (3, 0)}
    return [(f"hero_cowboy_m_{way}", 48, 48,
             [art.crop(((col + i) * 48, row * 48, (col + i + 1) * 48, (row + 1) * 48))
              for i in range(3)])
            for way, (row, col) in where.items()]



# --- animals repainted from the CC0 deer -------------------------------------

DEER_ANTLER = (156, 128, 95)
DEER_ANTLER_SHADE = (102, 78, 51)
ANTLERS = (DEER_ANTLER, DEER_ANTLER_SHADE)


def repaint(frame, coat, shade, antler, belly=None, rump=None, neck=None):
    """Recolour one right-facing deer frame; regions are relative to its box."""
    art = frame.copy()
    box = art.getbbox()
    if not box:
        return art
    x0, y0, x1, y1 = box
    px = art.load()
    for y in range(y0, y1):
        for x in range(x0, x1):
            r, g, b, a = px[x, y]
            if not a or (r, g, b) == (0, 0, 0):
                continue
            fx, fy = (x - x0) / (x1 - x0), (y - y0) / (y1 - y0)
            body = (r, g, b) == (110, 81, 57)
            if (r, g, b) in ANTLERS:
                colour = antler[ANTLERS.index((r, g, b))] if antler else None
            elif body and rump and fx < 0.17 and 0.35 < fy < 0.68:
                colour = rump
            elif neck and fx > 0.62 and fy < 0.62:
                colour = neck if body else shade
            elif body and belly and 0.58 < fy < 0.70 and 0.15 < fx < 0.68:
                colour = belly
            else:
                colour = coat if body else shade
            if colour is None:
                px[x, y] = (0, 0, 0, 0)
            else:
                px[x, y] = (*colour, a)
    return art


def elk_frame(frame):
    return repaint(frame, (176, 136, 92), (104, 76, 52), ((228, 206, 160), (186, 160, 116)),
                   rump=(236, 220, 180), neck=(92, 62, 40))


def pronghorn_frame(frame):
    """No antlers, a tan and white coat, and two short black horns."""
    px = frame.load()
    antlers = [(x, y) for y in range(frame.height) for x in range(frame.width)
               if px[x, y][3] and px[x, y][:3] in ANTLERS]
    art = repaint(frame, (206, 138, 70), (128, 78, 40), None,
                  belly=(244, 238, 224), rump=(248, 244, 232))
    if antlers:
        # Antler pixels that touch the head's outline mark the crown, even
        # when the head is down grazing.
        def touches_head(x, y):
            return any(0 <= x + dx < frame.width and 0 <= y + dy < frame.height
                       and px[x + dx, y + dy] == (0, 0, 0, 255)
                       for dx in (-1, 0, 1) for dy in (-1, 0, 1))
        roots = [q for q in antlers if touches_head(*q)] or antlers
        bx = sum(x for x, _ in roots) // len(roots)
        base_y = max(y for _, y in roots)
        d = ImageDraw.Draw(art)
        black = (30, 22, 18, 255)
        d.line([bx, base_y + 1, bx - 1, base_y - 5], fill=black, width=2)
        d.line([bx, base_y - 3, bx + 3, base_y - 4], fill=black)
    return art


# The deer art stands left of centre in its 72px frames, so it goes in a wider
# cell: the body then sits in the middle, where shots and mirroring expect it.
DEER_W, DEER_SHIFT = 80, 8


def centred(frame):
    cell = Image.new("RGBA", (DEER_W, frame.height), (0, 0, 0, 0))
    cell.paste(frame, (DEER_SHIFT, 0))
    return cell


def animals():
    """(name, cell width, cell height, frames) for every walking meal."""
    def strip(file, width):
        art = Image.open(VENDOR / "scratchio" / f"{file}.png").convert("RGBA")
        # The pack faces left; the sheet faces right.
        return [ImageOps.mirror(art.crop((i * width, 0, (i + 1) * width, art.height)))
                for i in range(art.width // width)]

    out = []
    for act in ("walk", "run", "idle"):
        deer = strip(f"deer_{act}", 72)
        for name, frames in [("deer", deer), ("elk", [elk_frame(f) for f in deer]),
                             ("antelope", [pronghorn_frame(f) for f in deer])]:
            out.append((f"{name}_{act}", DEER_W, 52, [centred(f) for f in frames]))
    for act in ("hop", "idle"):
        out.append((f"rabbit_{act}", 32, 26, strip(f"rabbit_{act}", 32)))
    return out


def downloaded_cell(file):
    """A downloaded image centred on the bottom edge of a wide cell."""
    art = Image.open(VENDOR / "kenney" / f"{file}.png").convert("RGBA")
    art.thumbnail((WIDE_W, CELL_H), Image.NEAREST)
    cell = Image.new("RGBA", (WIDE_W, CELL_H), (0, 0, 0, 0))
    cell.paste(art, ((WIDE_W - art.width) // 2, CELL_H - art.height))
    return cell


def drawn_cells(draw, frames, width):
    cells = []
    for f in range(frames):
        cell = Image.new("RGBA", (width, CELL_H), (0, 0, 0, 0))
        draw(ImageDraw.Draw(cell), f)
        cells.append(cell)
    return cells


def visible(cells):
    """[x, y, width, height] of what can be seen in any frame of a strip, so
    the game can stand a sprite on its feet whatever padding its cell has."""
    boxes = [c.getchannel("A").point(lambda a: 255 if a > 40 else 0).getbbox() for c in cells]
    x0, y0 = (min(b[i] for b in boxes if b) for i in (0, 1))
    x1, y1 = (max(b[i] for b in boxes if b) for i in (2, 3))
    return [x0, y0, x1 - x0, y1 - y0]


def tall_cells(draw, frames, width, height):
    cells = []
    for f in range(frames):
        cell = Image.new("RGBA", (width, height), (0, 0, 0, 0))
        draw(ImageDraw.Draw(cell), f)
        cells.append(cell)
    return cells


# The project owner's own dumpsters (MIT, from their scratch_window project).
# Both are cut by the same box, so one swaps for the other without moving.
DUMPSTER_BOX = (4, 1, 60, 48)


def dumpsters():
    out = []
    for name in ("empty", "full"):
        art = Image.open(VENDOR / "spacestate1" / f"dumpster-{name}.png").convert("RGBA").crop(DUMPSTER_BOX)
        out.append((f"dumpster_{name}", art.width, art.height, [art]))
    return out


def sprite_sheet():
    strips = [(name, CELL_W, CELL_H, drawn_cells(draw, frames, CELL_W))
              for name, frames, draw in SPRITES]
    strips += [(name, WIDE_W, CELL_H, [downloaded_cell(f) for f in files])
               for name, files in DOWNLOADED]
    strips += [(draw.__name__, PLANT, PLANT, [plant_cell(draw)]) for draw in PLANTS + SCRAPS]
    strips += [("goddess", 48, 64, tall_cells(goddess, 4, 48, 64))]
    strips += [(draw.__name__, 32, 32, tall_cells(draw, 1, 32, 32)) for draw in VINTAGE]
    strips += dumpsters()
    strips += animals()
    strips += heroes()

    width = max(w * len(cells) for _, w, _, cells in strips)
    height = sum(h for _, _, h, _ in strips)
    sheet = Image.new("RGBA", (width, height), (0, 0, 0, 0))
    atlas, y = {}, 0
    for name, w, h, cells in strips:
        for f, cell in enumerate(cells):
            sheet.paste(cell, (f * w, y))
        atlas[name] = {"x": 0, "y": y, "w": w, "h": h, "frames": len(cells),
                       "art": visible(cells)}
        y += h
    sheet.save(ROOT / "assets/img/sprites.png", optimize=True)
    text = ",\n".join(f'  "{k}": {json.dumps(v)}' for k, v in atlas.items())
    (ROOT / "assets/data/sprites.json").write_text("{\n" + text + "\n}\n")
    print(f"{len(atlas)} sprites, sheet {sheet.size[0]}x{sheet.size[1]}")


def layer(source):
    return Image.open(VENDOR / f"{source}.png").convert("RGBA")


def tint(art, dark, light):
    """Repaint a layer between two colours, keeping its shading and shape."""
    grey = ImageOps.autocontrast(art.convert("L"))
    out = ImageOps.colorize(grey, dark, light).convert("RGBA")
    out.putalpha(art.getchannel("A"))
    return out


def seamless(art):
    """The layer followed by its mirror image, so it repeats without a seam."""
    out = Image.new("RGBA", (art.width * 2, art.height), (0, 0, 0, 0))
    out.paste(art, (0, 0))
    out.paste(ImageOps.mirror(art), (art.width, 0))
    return out


def backgrounds():
    far = layer("ansimuz/far")
    green = layer("fabinhosc/mountain1")
    ridge = layer("fabinhosc/mountain2")
    trees = layer("ansimuz/trees")
    out = {
        # Dusk set, as downloaded: used on the mountain passes.
        "dusk_sky": layer("ansimuz/sky"),
        "dusk_far": seamless(far),
        "dusk_mid": layer("ansimuz/mountains"),
        "dusk_trees": trees,
        # Daytime sets, repainted for each kind of country.
        "clouds": seamless(layer("fabinhosc/clouds")),
        "day_far": seamless(tint(far, (104, 126, 160), (214, 226, 238))),
        "hills_tan": seamless(tint(ridge, (132, 104, 74), (214, 190, 138))),
        "hills_green": seamless(tint(green, (44, 84, 66), (128, 166, 104))),
        "hills_red": seamless(tint(ridge, (112, 58, 46), (214, 138, 94))),
        "trees_green": tint(trees, (36, 70, 44), (80, 116, 66)),
        # Winter: everything under snow, shape and shading kept.
        "day_far_snow": seamless(tint(far, (136, 156, 188), (250, 252, 255))),
        "hills_snow": seamless(tint(ridge, (158, 172, 196), (248, 250, 255))),
        "trees_snow": tint(trees, (58, 86, 92), (214, 228, 236)),
        # Daytime peaks for the mountain stretches.
        "peaks_day": tint(layer("ansimuz/mountains"), (58, 78, 106), (168, 188, 206)),
        "peaks_snow": tint(layer("ansimuz/mountains"), (122, 142, 174), (248, 250, 255)),
        "dusk_mid_snow": tint(layer("ansimuz/mountains"), (70, 70, 110), (214, 214, 240)),
        "dusk_trees_snow": tint(trees, (34, 48, 66), (150, 164, 190)),
        # Spring green and autumn gold.
        "hills_spring": seamless(tint(green, (54, 104, 60), (166, 208, 100))),
        "hills_spring_low": seamless(tint(ridge, (92, 116, 62), (190, 208, 120))),
        "hills_gold": seamless(tint(ridge, (126, 84, 38), (232, 188, 92))),
        "trees_gold": tint(trees, (112, 66, 22), (216, 158, 46)),
    }
    for name, art in out.items():
        art.save(ROOT / f"assets/img/bg_{name}.png", optimize=True)
    print(f"{len(out)} background layers")


def dumpster():
    """The winter gathering backdrop: down inside a dumpster behind a store."""
    r = rnd(40)
    art = Image.new("RGB", (320, 240), (46, 52, 42))
    d = ImageDraw.Draw(art)
    # What shows over the rim: the back wall of the store, in snow.
    # (The score bar covers the top, so the rim sits well below it.)
    d.rectangle([0, 0, 319, 58], fill=(138, 78, 62))
    for row, y in enumerate(range(4, 58, 6)):
        d.line([0, y, 319, y], fill=(108, 60, 50))
        for x in range((row % 2) * 8, 320, 16):
            d.line([x, y - 5, x, y], fill=(108, 60, 50))
    d.rectangle([0, 0, 319, 5], fill=(236, 242, 250))
    for _ in range(90):
        d.point((r.randrange(320), r.randrange(58)), fill=(240, 246, 255))
    # The rim, with snow along it.
    d.rectangle([0, 58, 319, 71], fill=(58, 118, 90), outline=(20, 48, 36))
    d.line([0, 59, 319, 59], fill=(236, 242, 250), width=3)
    for x in range(10, 320, 22):
        d.rectangle([x, 65, x + 3, 68], fill=(30, 70, 52))
    # The inside: a stained steel floor, strewn with litter.
    d.rectangle([0, 72, 319, 80], fill=(24, 30, 22))
    for _ in range(900):
        x, y = r.randrange(320), r.randrange(80, 240)
        d.point((x, y), fill=r.choice([(60, 66, 52), (36, 40, 30), (74, 60, 40)]))
    for _ in range(110):
        x, y = r.randrange(6, 306), r.randrange(84, 232)
        c = r.choice([(150, 146, 130), (116, 90, 56), (86, 110, 128), (128, 70, 60), (96, 104, 74)])
        d.polygon([(x, y), (x + r.randint(3, 9), y - r.randint(0, 3)), (x + r.randint(2, 8), y + r.randint(2, 5))], fill=c)
    for x in (0, 312):  # side walls
        d.rectangle([x, 72, x + 7, 239], fill=(44, 92, 70), outline=(20, 48, 36))
    art.resize((960, 720), Image.NEAREST).save(ROOT / "assets/img/field_dumpster.jpg", quality=84)


def main():
    sprite_sheet()
    backgrounds()
    dumpster()
    # The UI sheet is used as it comes.
    layer("ermizidis/gui").save(ROOT / "assets/img/ui.png", optimize=True)


if __name__ == "__main__":
    main()
