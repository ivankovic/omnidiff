# Accuracy analysis: rust-turbopack-persistence-tools-main

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 925 nodes, 925 graded, N:M floor 0 visible |
| Painting, Minimal | 58 bytes wrong of 4291: renderer 58, matcher 0 |
| Painting, Full | 8 bytes wrong of 4291: renderer 8, matcher 0 |
| Ground-truth invariant violations | none |

## The change

`turbo-persistence-tools/src/main.rs`: `DbConfig` is added to an import and
`TurboPersistence::open_read_only(path)?` becomes
`TurboPersistence::open_read_only_with_config(path, DbConfig::default())?`, which rustfmt wraps
onto its own line (after line 20). `description.md` records it as a performance fixture
(~5 s for 925 nodes at the time).

## Painting

All runs are the same with the human mapping: renderer-owned, the mapping is right.

- **Minimal, Move -> None, 46 bytes (`TurboPersistence::` on both sides, `(path` on before line 19
  and after line 20).** The call was rewrapped onto a new line; the painting treats that as no
  change, omnidiff paints the unchanged tokens of the rewrapped call Move. `renderer`.
- **Minimal, Update -> Insert, 12 bytes (`_with_config`, after line 20)**, and the 5 bytes
  `(path` after it (Move -> None, counted above). omnidiff renders the identifier rename
  `open_read_only` -> `open_read_only_with_config` as a narrowed Update of the added suffix; the
  painting calls the suffix an Insert. `renderer`.
- **Full, None -> Insert, 8 bytes:** the indentation of the new continuation line (after line 20).
  The Full painting paints it Insert; omnidiff leaves leading whitespace unpainted. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `diff::text` Move painting of rewrapped lines (row-only shift) and Update
  narrowing of a suffix-extended identifier
- Ground truth questionable: no
