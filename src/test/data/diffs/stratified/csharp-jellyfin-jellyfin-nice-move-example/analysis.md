# Accuracy analysis: csharp-jellyfin-jellyfin-nice-move-example

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 27 (15 visible) of 1574 nodes, 1574 graded, N:M floor 0 visible |
| Painting, Minimal | 901 bytes wrong of 8497: renderer 817, matcher 137 |
| Painting, Full | 1051 bytes wrong of 8497: renderer 938, matcher 151 |
| Ground-truth invariant violations | none |

## The change

In Jellyfin's `DotIgnoreIgnoreRule.cs`, the body of
`CheckIgnoreRules(string path, string ignoreFileContent, bool isDirectory)` moves into a new
overload `CheckIgnoreRules(string path, string[] rules, bool isDirectory, bool normalizePath)`
(after lines 95-131). The old method now ends with
`return CheckIgnoreRules(path, rules, isDirectory);` (after line 74), and a three-argument
forwarding overload is added. Inside the moved body, `ignore.Add(rules)` becomes a `foreach`
with `try { ignore.Add(rule); ... } catch (RegexParseException)`, and `IsWindows` becomes
`normalizePath`.

## Mapping

All 27 mismatches (15 visible) come from `APTED("qualified_name")`. Most moved statements are
matched as the ground truth wants (they are not in the list). The exceptions:

- Before line 69, 2 visible and 2 invisible: the old method's `method_declaration`, `block`,
  `bool` and `CheckIgnoreRules` are deleted, and the after line 70 copy is inserted. The
  signature is unchanged, and the ground truth keeps it. `matcher`.
- Before line 74 / after line 74, 5 visible and 5 invisible: `ignore.Add(rules);`. The ground
  truth moves it into the `try` and updates `rules` -> `rule` (after line 105). Omnidiff deletes
  the statement and its `(`, `)`, `;`, and pairs its `rules` argument with the `rules` of the new
  forwarding call `return CheckIgnoreRules(path, rules, isDirectory);`. `matcher`.
- Before line 78, 8 visible and 5 invisible: `var pathToCheck = IsWindows ? ... : path;`. The
  ground truth moves it to after line 122 with `IsWindows` -> `normalizePath`. Omnidiff deletes
  and inserts it. `matcher`.

## Painting

Both paintings move the whole old body (comments, `var ignore`, `pathToCheck`, the `if`, the
`return`) into the new overload. They insert the new code, and leave the old signature alone.

- Renderer, Minimal about 790 bytes, `None -> Move`. These are the moved statements on before
  lines 73-86 and after lines 97-130, and they are also present with the human mapping. The
  statements keep their column (8 spaces) in the new method. `identical_or_move` in
  `src/diff/text.rs` paints Move only on a meaningful column shift or a backwards crossing, so a
  same-column move into another method stays unpainted. The reader then sees no sign that the
  body moved. `renderer`.
- Renderer, Minimal 10 bytes, `None -> Insert`: the lone `{` / `}` of the inserted overload,
  `foreach`, `try`, `catch` and `if` blocks. The Minimal punctuation filter drops them. (A few more
  bytes with the human mapping: `s);` Update vs Delete, `);` vs Move.)
- Matcher, Minimal about 101 bytes: `bool CheckIgnoreRules` painted Delete/Insert on lines 69/70.
  `(rules);` and the `pathToCheck` lines painted Delete/Insert instead of Move. ` rules` of the
  forwarding call left unpainted.
- Full: renderer 938 bytes. These are the same moved statements with their indentation, plus the
  old method's closing `}` painted Move (before line 87, after line 75). The matcher's share is
  about 113 bytes, as in Minimal.

## Verdict

- Primary cause: `renderer`
- Other causes: `matcher`
- Where a fix would go: `identical_or_move` in `src/diff/text.rs`. A subtree that moved into a
  different method should be painted Move even when its column is unchanged. In the matcher,
  keep the unchanged method header, and move the extracted statements, at the `qualified_name`
  APTED call.
- Ground truth questionable: no
