# Accuracy analysis: java-defects4j-jacksondatabind-105-jdkdeserializers

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 865 nodes, 865 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 3629: renderer 0, matcher 0 |
| Painting, Full | 1 bytes wrong of 3629: renderer 1, matcher 0 |
| Ground-truth invariant violations | none |

## The change

`Void.class` is added to the list of JDK types handled (line 22 gains a `,`, line 23 is new), and
`find()` gains an `if (rawType == Void.class) return NullifyingDeserializer.instance;` branch (lines
49-51 after).

## Painting

Minimal matches exactly. Full, `Insert -> None`, line 22 after side, 1 byte: the `,` appended to
`ByteBuffer.class` so that `Void.class` can follow. omnidiff's Full rendering paints it inserted;
the Full painting does not, although it paints the inserted line's indentation and Full is the style
that paints punctuation (`text_painting_findings.md`, rule 1). Same with the human mapping. The
added comma is a real insertion, so omnidiff's Full reading looks right and the Full painting looks
like it missed the byte. `ground-truth`.

## Verdict

- Primary cause: `ground-truth`
- Other causes: none
- Where a fix would go: ground truth (Marko's call): Full painting of the appended comma
- Ground truth questionable: yes: Full painting leaves the appended , on line 22 unpainted while
  painting the new line's indentation; Full otherwise paints punctuation
