# Accuracy analysis: rust-turbopack-module-rule

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers. Line numbers below are 1-based.

## Numbers

| | |
|---|---|
| Mapping mismatches | 62 (43 visible) of 3925 nodes, 3831 graded, N:M floor 0 visible |
| Painting, Minimal | 3116 bytes wrong of 19564: renderer 2487, matcher 687 |
| Painting, Full | 4177 bytes wrong of 19564: renderer 3289, matcher 948 |
| Ground-truth invariant violations | 7 (three [3], two [11], one [12], one [21]) |

## The change

`impl ModuleType { fn from_str_with_defaults(type_str, preprocess, ..., environment) }` is split
in two. A new `enum ConfiguredModuleType` gets `parse(type_str: &str) -> Result<Self>`, which keeps
the old `match type_str { "asset" => ..., _ => bail!(...) }`, and
`into_effect(self, preprocess, ..., environment)`, which keeps the old parameters and the old arm
bodies (`ModuleType::Ecmascript { preprocess, ... }`), now wrapped in
`ModuleRuleEffect::ModuleType(...)`. The `InlinedBytesJs` variant goes, a `json` arm is added, and
one `use` list is rewrapped. The fixture has no README.md or description.md.

## Mapping

The ground truth splits the old function in the same way: the arm patterns, `=>`, `,`, the
scrutinee and the `bail!` arm go to `parse` (after lines 204-219); the arm bodies and parameters
2-6 go to `into_effect`. omnidiff agrees on the arm patterns (`"asset"`, ...), the arm bodies and
the parameters. It disagrees on the glue around them.

- **Arm glue of the old `match` (before lines 191-217, 222).** 20 visible, 10 invisible. The ground
  truth maps the scrutinee `type_str` (191), the `=>` and `,` of the nine string arms (192-217) and
  the match block's `}` (222:9) to their counterparts in `parse`. omnidiff deletes all of them,
  together with the nine `match_arm` nodes and the `match_block` (invisible), with reason
  `APTED("fast_fallback")` (the whole-file `for_roots_fallback`, a Myers LCS over the residual
  forest). Each arm's two halves went to two different functions, so the `match_arm` parents stayed
  unmatched until the fallback, and the fallback did not pair them with `parse`'s arms. The
  evidence gives only the final reason, not why the LCS left them unpaired. `matcher`.
- **The `_ => bail!(...)` arm (before lines 218-221).** 12 visible, 5 invisible. The arm in `parse`
  (after 215-218) differs only by `json, ` in the second line of the string. The ground truth maps
  it (an Update on the line-220 string fragment). omnidiff deletes it and inserts the new one
  (`APTED("fast_fallback")`). The arm is not hash-identical, and nothing before the fallback
  paired it. `matcher`.
- **`parse`'s signature (after line 203).** 8 visible, 4 invisible. The ground truth inserts
  `parse`'s parameters (`type_str: &str`) and return type (`Result<Self>`) with their children.
  omnidiff maps them to the old function's `type_str: &str` (184) and `Result<Self>` (190) as
  moves (`MovedSubtree`, the texts are byte-identical). The ground truth sends the old
  function's scrutinee `type_str` to `parse`, and its own Minimal painting leaves these exact
  tokens unpainted: invariant [11] lists `type_str`, `str`, `Result`, `Self` on line 203 as
  "unchanged and in place" against the tree's insert. omnidiff's mapping agrees with the painting.
  `ground-truth`.
- **Closing brackets (before 222:10 `)`, 223:5 `}`, 224:1 `}`).** 3 visible. The ground truth maps
  them to `parse`'s `Ok(...)` `)` (after 219), `parse`'s block `}` (220) and the new impl's `}`
  (276), while it inserts the matching opening brackets (after 204:10 `(`, 203:49 `{`, 201:26 `{`).
  Invariant [3] reports all three split pairs. omnidiff deletes the closing brackets
  (`APTED("fast_fallback")`), which is consistent with the inserted openers. `ground-truth`.

Reasons for the "with children" mismatches re-read on 2026-10-13 after the test helper began
printing them.

## Painting

The only painting is `Minimal`, and both presets score against it. It has six entries: the
rewrapped `use`, the two deleted `InlinedBytesJs` lines, the new enum (after 181-200), an Update
`impl ModuleType` -> `impl ConfiguredModuleType`, and one Match of
`ModuleType::StaticUrlJs { tag: None }` (before 192 -> after 243). Except for these last two, there
is nothing on before lines 181-224 or after lines 202-276. That includes the whole new
`into_effect` (`pub async fn into_effect(`, `self,`, the `ConfiguredModuleType::Bytes => {` arm
with its comments), which is plainly new code. Invariants [11] (81 inserted leaves left unpainted
on after lines 202-...) and [12] (the `json, ` edit in the `bail!` string not painted) flag the
same gap. The painting looks unfinished. Every disagreement run has `theirs=None` at its first
byte.

| Group | Lines | Minimal bytes | Full bytes | Owner | Category |
|---|---|---|---|---|---|
| omnidiff paints the refactored functions, the painting has nothing there | before 181-224, after 202-276 | 2984 (about 640 matcher, 2345 renderer) | 3977 (about 900 matcher, 3080 renderer) | painting | `ground-truth` |
| `EcmascriptInputTransforms, EcmascriptOptions` (and `};` in Full) painted Move after the `use` list was rewrapped onto its own line (Move -> None) | before 13, after 14-15 | 88 | 92 | renderer | `renderer` |
| `impl ModuleType {` Delete / `impl ConfiguredModuleType {` Insert against the painting's Update of the type name | before 180, after 201 | 44 (17 matcher, 27 renderer) | 44 | painting | `ground-truth` |
| Indentation of deleted and inserted lines painted (Delete/Insert -> None): Full's `leading_whitespace` against a Minimal-only painting | before 152, 173; after 187-198 | 0 | 64 | renderer | `grading-artifact` |

- In the large group, the matcher's share is the mapping groups above made visible:
  `Ok(match type_str {`, every `=>`, the `bail!` arm and the closing `})`/`}` are Delete/Insert
  instead of unpainted, and `type_str: &str` (before 184) is a Move. Without a painting for these
  lines, no claim about them can be checked here.
- `impl` header: the painting's Update contradicts the tree mapping, which inserts the after
  `impl_item:3` (invariant [21]). Both mappings render `impl ConfiguredModuleType {` as Insert. The
  17 matcher bytes are omnidiff deleting the before header, which the human mapping leaves
  ungraded and unpainted.
- `use` rewrap: both mappings render it the same way. The renderer reads the column shift of the
  two names (line 13 col 27 -> line 14 col 4) as a relocation; the painting treats the rewrap as no
  change.

## Verdict

- Primary cause: `ground-truth` (the painting covers almost none of the refactored region: about
  96% of the wrong bytes in both presets)
- Other causes: `matcher` (32 of 43 visible mapping mismatches: `APTED("fast_fallback")` deletes
  the arm glue and the `bail!` arm of a `match` whose arms were split across two functions),
  `renderer` (`use` rewrap painted as Move), `grading-artifact` (Full scored against a Minimal
  painting)
- Where a fix would go: the painting and the three bracket pairs (Marko's call); for the matcher,
  the residual-forest fallback (`apted::for_roots_fallback` ->
  `common::resolve_residual_forest_via_myers_lcs`) or an earlier pass that matches a `match_arm` by
  its pattern when its body went elsewhere
- Ground truth questionable: yes: the Minimal painting looks unfinished (nothing on most of before
  181-224 and after 202-276; invariants [11], [12]); the tree maps three closing brackets whose
  opening brackets it inserts (invariant [3]); the tree inserts `parse`'s `type_str: &str` and
  `Result<Self>` while the painting leaves them unpainted (invariant [11]); the painting updates the
  impl type name that the tree inserts (invariant [21])
