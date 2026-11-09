"""Builds the tiny fonts `diff::content::font`'s tests read: two versions of a three-glyph font.

`before.ttf` has `.notdef`, A (a square) and B (a triangle); `after.ttf` keeps A, makes B wider,
and adds C, with a new version string; `after.woff` and `after.woff2` are `after.ttf` wrapped.
Synthetic, so no font's license travels with them. Run from research/ (it has fontTools):

    uv run --with brotli python ../src/test/data/content/make_fonts.py
"""

from pathlib import Path

from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen

HERE = Path(__file__).parent


def square(pen):
    pen.moveTo((100, 0))
    pen.lineTo((100, 700))
    pen.lineTo((600, 700))
    pen.lineTo((600, 0))
    pen.closePath()


def triangle(width):
    def draw(pen):
        pen.moveTo((50, 0))
        pen.lineTo((width // 2, 700))
        pen.lineTo((width - 50, 0))
        pen.closePath()

    return draw


def build(glyphs, version):
    order = [".notdef", *glyphs]
    builder = FontBuilder(1000, isTTF=True)
    builder.setupGlyphOrder(order)
    builder.setupCharacterMap({ord(name): name for name in glyphs})
    drawn = {}
    for name in order:
        pen = TTGlyphPen(None)
        if name in glyphs:
            glyphs[name][0](pen)
        drawn[name] = pen.glyph()
    builder.setupGlyf(drawn)
    builder.setupHorizontalMetrics(
        {name: (glyphs[name][1] if name in glyphs else 500, 0) for name in order}
    )
    builder.setupHorizontalHeader(ascent=800, descent=-200)
    builder.setupNameTable({"familyName": "Diff Test", "styleName": "Regular", "version": version})
    builder.setupOS2()
    builder.setupPost()
    return builder.font


before = build({"A": (square, 700), "B": (triangle(600), 600)}, "Version 1.000")
before.save(HERE / "before.ttf")
after = build(
    {"A": (square, 700), "B": (triangle(900), 900), "C": (triangle(500), 500)}, "Version 1.001"
)
after.save(HERE / "after.ttf")
for flavor in ("woff", "woff2"):
    after.flavor = flavor
    after.save(HERE / f"after.{flavor}")
