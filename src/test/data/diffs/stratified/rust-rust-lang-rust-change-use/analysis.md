# Accuracy analysis: rust-rust-lang-rust-change-use

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 6 (6 visible) of 505 nodes, 505 graded, N:M floor 0 visible |
| Painting, Minimal | 153 bytes wrong of 2383: renderer 162, matcher 26 |
| Painting, Full | 153 bytes wrong of 2383: renderer 139, matcher 16 |
| Ground-truth invariant violations | 6 |

## The change

From rust-lang/rust (`compiler/rustc_target/src/spec/targets/csky_unknown_linux_gnuabiv2.rs`,
commit 40ebcc03): `Abi` is renamed `CfgAbi`. The import list `{Abi, Arch, Cc, LinkerFlavor, ...}`
becomes `{Arch, Cc, CfgAbi, LinkerFlavor, ..., base,}`, re-sorted and wrapped onto its own line
(after lines 1-3), and `abi: Abi::AbiV2` becomes `cfg_abi: CfgAbi::AbiV2` (line 19 / 21).

## Mapping

Six visible mismatches, one cause, reason `APTED("import_list_overlap")`, before line 1. The
ground truth maps `Abi` -> `CfgAbi` (renamed and moved to third place) and keeps `Arch` and `Cc`
identical. omnidiff aligns the first three items by position: `Abi` -> `Arch`, `Arch` -> `Cc`,
`Cc` -> `CfgAbi` (three updates, with their commas paired off by one). An off-by-one alignment of
a re-sorted import list in which one item was renamed. `matcher`.

## Painting

Renderer-caused (present with the human mapping too):

- Minimal, 142 bytes, `Move -> None`: `Arch, Cc,` and `LinkerFlavor, Lld, Target, TargetMetadata,
  TargetOptions, base` on before line 1 and after line 2. The list was wrapped onto its own line,
  so every item changed row and column, and Minimal paints them `Move`; the Minimal painting
  leaves the rewrap alone. `renderer`.
- Full, about 11 bytes: the new line's indentation and single spaces the Full painting inserts or
  moves, and `, ` before `Arch`. `renderer`.

Painting-caused (also present with the human mapping):

- Minimal, 20 bytes, `None -> Update`: `Abi,` on before line 1 and in after line 2, and `abi` /
  `Abi` on line 19 / 21. The Minimal painting updates the whole names `Abi` -> `CfgAbi` and `abi`
  -> `cfg_abi`; omnidiff paints only the added `Cfg` / `cfg_`. Invariant [16] flags all six
  (Minimal paints a renamed identifier narrowed). `ground-truth`.
- Full, about 128 bytes, `Move -> Update`: the Full painting's match for the list tail spans
  `LinkerFlavor, ..., base` (before) and `LinkerFlavor, ..., base,` (after), including the new
  trailing comma on one side only, so it resolves to `Update`; omnidiff paints the moved tail
  `Move`. The Full painting also matches the unmoved `{` to itself, which resolves to `Move` (2
  bytes). `ground-truth`.

Matcher-caused (Minimal 26 bytes, Full 16): the off-by-one updates above, painting `Arch`, `Cc`
and their commas as updated.

## Verdict

- Primary cause: `renderer`
- Other causes: `ground-truth`, `matcher`
- Where a fix would go: `src/diff/text.rs` `identical_or_move` (Minimal: items of a list wrapped
  onto a new line); `APTED("import_list_overlap")` alignment of a re-sorted list with a renamed
  item
- Ground truth questionable: yes: Minimal paints the renamed `Abi`/`abi` whole (invariant [16],
  6 violations); the Full painting's list-tail match includes the new trailing comma on one side,
  turning a move into an update, and it matches the unmoved `{` to itself
