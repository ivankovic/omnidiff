# Accuracy analysis: c-openssl-openssl-very-interesting-test-case

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 4588 nodes, 4588 graded, N:M floor 0 visible |
| Painting, Minimal | 633 bytes wrong of 19331: renderer 633, matcher 0 |
| Painting, Full | 636 bytes wrong of 19331: renderer 636, matcher 0 |
| Ground-truth invariant violations | 3 (one rule 16 in Minimal, two rule 5 in Full) |

## The change

openssl `snmpkdf.c` (`description.md`: "Showcase"). The doc comment above `SNMPKDF` (lines 262-275)
gains ` * Input:` and ` * Output:` headings, `okeylen - key output length` becomes
`keylen - key length`, and a new `okey - filled with derived key` line is added. In the code the
parameter `okeylen` is renamed to `keylen` (lines 279, 288, 298), the comparison
`mdsize < okeylen` becomes `mdsize > keylen`, and `memcpy(okey, digest, okeylen)` becomes
`memcpy(okey, digest, md_len)` (line 318). The paintings are rich: the new `okey` line is a copy
of the old one (an N:M text match).

## Painting

The mapping is perfect and every run is identical with the human mapping, so none of it is the
matcher's.

- Both presets, `Update -> None` (plus `None -> Insert` at the first byte of the new ` * Input:`
  line), the comment, lines 268-274 before and 268-277 after: 624 bytes Minimal, 616 bytes Full.
  The whole doc comment is one `comment` token, mapped as an Update. omnidiff narrows it only by the
  common prefix and suffix, so it paints one block from column 3 of line 268 to the end of the
  changed lines on each side, unchanged parameter lines included. The paintings mark only the
  inserted headings, the renamed `okeylen`, the deleted words and the copied `okey` line, and leave
  the unchanged lines alone. `renderer`.
- Minimal, `Update -> Delete`, 3 bytes, the `o` of `okeylen` on lines 279, 288 and 298 (before):
  `okeylen` -> `keylen` is an identifier update narrowed to the dropped `o`. omnidiff labels the
  dropped character Update; the painting labels it Delete. `renderer`.
- Minimal, `None -> Update`, 6 bytes, `len` of `okeylen` -> `md_len` on line 318 / 321: the painting
  matches the whole identifier; omnidiff paints only `okey` / `md_` and leaves the common `len`.
  invariants.txt flags this painting entry (rule 16: for a Minimal reading only the differing words
  should be painted). `ground-truth`.
- Full, `Move -> None`, 18 bytes, the tails `)`, `== 0)`, `)` and `);` after the renamed identifier
  on lines 279/282, 288/291, 298/301 and 318/321: the shorter identifier shifts them one column
  left, and the Full renderer paints the shifted tail Move. The painting leaves an identical line
  tail unpainted (painting rule 3). `renderer`.
- Full, `Update -> None`, 2 bytes, the space between `<`/`>` and `okeylen`/`keylen` on line 298 /
  301: omnidiff merges the two adjacent updates across the space; the Full painting leaves the
  space unpainted between two painted regions, which invariants.txt flags on both sides (rule 5).
  `ground-truth`.

## Verdict

- Primary cause: `renderer`
- Other causes: `ground-truth`
- Where a fix would go: `intra_node_update_ranges` in `src/diff/text.rs` for multi-line comment
  updates (line- or word-level alignment instead of one prefix/suffix-trimmed block); the Full
  displaced-tail rule in `identical_or_move`; Delete vs Update labelling of a pure-deletion part
  of a narrowed update
- Ground truth questionable: yes: invariants.txt flags the Minimal `okeylen` -> `md_len` entry
  painting the shared `len` (rule 16), and the Full painting's unpainted whitespace gap between `>`
  and `keylen` on both sides (rule 5); 8 bytes in total
