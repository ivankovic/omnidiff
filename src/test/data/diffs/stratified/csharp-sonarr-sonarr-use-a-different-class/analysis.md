# Accuracy analysis: csharp-sonarr-sonarr-use-a-different-class

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 400 nodes, 400 graded, N:M floor 0 visible |
| Painting, Minimal | 64 bytes wrong of 1978: renderer 64, matcher 0 |
| Painting, Full | 255 bytes wrong of 1978: renderer 255, matcher 0 |
| Ground-truth invariant violations | none |

## The change

`CustomFormatAllowedbyProfileSpecification` moves to the download-decision API:
`IDecisionEngineSpecification` becomes `IDownloadDecisionEngineSpecification` (line 7), `Decision`
becomes `DownloadSpecDecision` (lines 12, 19, 22), and the `Reject` call gains a leading
`DownloadRejectionReason.CustomFormatMinimumScore,` argument (line 19). It is the same kind of edit
as `csharp-sonarr-sonarr-many-updates-with-one-insert`.

## Painting

Mapping is perfect and every run is identical with the human mapping, so all of it is
renderer-owned.

Minimal, 64 bytes:

- Renamed identifiers, 48 bytes on after lines 7, 12, 19, 22: the painting inserts `Download`
  after `I` and `DownloadSpec` in front of `Decision`. omnidiff's word diff slides each insertion
  one character right over the shared `D` (painting `ownloadD`, `ownloadSpecD`) and labels it
  Update, so each run is the first `D` (None vs Insert), the inserted letters (Update vs Insert)
  and the `D` of `Decision` (Update vs None). `renderer`.
- `.Reject(`, 16 bytes, Move -> None, line 19 both sides: identical text pushed 12 columns right by
  the renamed receiver on the same row. omnidiff's Minimal rendering paints it Move; the
  `shifted_by_an_edit_beside_it` exemption of `identical_or_move` (src/diff/text.rs) did not apply
  here (why was not traced; the `(` is directly followed by the inserted argument). `renderer`.

Full, 255 bytes, line 19: the identical argument tail `"Custom Formats …", …, minScore);` (127
before, 128 after including the space the painting inserts after the new argument), pushed right on
its row, is a Match (Move) in the Full painting; omnidiff leaves it unpainted through the same-row
exemption. `renderer`.

Painting conventions differ between sibling fixtures: this Minimal painting paints the identifier
renames as insertions (`Download`), `csharp-sonarr-sonarr-many-updates-with-one-insert`'s Minimal
painting paints the same renames as whole-identifier Updates (flagged there by invariant [16]); and
Full paintings disagree on whether identical text displaced along its row is a Move (this one and
many-updates: yes; `csharp-sonarr-sonarr-ok-small-edit`: no).

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: the intra-identifier diff in src/diff/text.rs (an insertion slides over the
  shared `D` and is labelled Update), and the Full policy for identical text displaced along its row
  (`identical_or_move`), on which the Full paintings differ
- Ground truth questionable: no (but the Minimal and Full painting conventions differ from sibling
  fixtures, see Painting)
