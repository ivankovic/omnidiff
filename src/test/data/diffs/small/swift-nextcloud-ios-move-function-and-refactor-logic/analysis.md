# Accuracy analysis: swift-nextcloud-ios-move-function-and-refactor-logic

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 16 (9 visible) of 10091 nodes, 10091 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

In `iOSClient/Networking/NCService.swift`, `requestDashboardWidget(account:)` moves from the end of
the class (before lines 268-314) to after line 125 and is rewritten:

- the `taskHandler` closures that track networking tasks are dropped;
- `results` is renamed `resultsDashboardWidget` / `resultsDownloadPreview`;
- the `await MainActor.run` wrapper goes;
- `let finalImage: UIImage?` becomes `var image: UIImage?`;
- the `SVGKImage` fallback becomes `try? await NCSVGRenderer().renderSVGToUIImage(...)`.

`import SVGKit` is removed.

## Mapping

The moved function itself is found (its unchanged statements are not mismatched). The mismatches
are inside it.

- **Wrong `account: account` twin (6 visible, 4 invisible), `matcher`.** The ground truth maps the
  `account: account` argument of `getDashboardWidgetAsync(account: account, taskHandler: ...)`
  (before line 269) to the same call's only argument in the after version (line 126). omnidiff
  deletes it (`APTED("qualified_name")`). It maps instead the identical `account: account` from
  `createIdentifier(account: account, ...)` (before line 271), which sits inside the deleted
  `taskHandler` closure. That pair also has reason `APTED("qualified_name")` (op Identical), so the
  same pass both deletes the line-269 argument and takes its twin from the closure. The candidates
  are identical, but only the line-269 one belongs to the call that survives.
- **`let finalImage: UIImage?` -> `var image: UIImage?` deleted (2 visible, 3 invisible),
  `matcher`.** The ground truth updates the declaration (before line 294 -> after line 136): `let`
  -> `var` and `finalImage` -> `image`, with the same type annotation and the same role. omnidiff
  deletes the declaration, its `let` and its name (`APTED("qualified_name")`, op Delete). The
  statement also moved out of the removed `MainActor.run` closure, which changes its parent.
- **`SVGKImage` -> `renderSVGToUIImage` deleted (1 visible), `ambiguous`.** The ground truth
  updates the identifier `SVGKImage` (before line 297) to the method name `renderSVGToUIImage` in
  the new `try? await NCSVGRenderer().renderSVGToUIImage(...)`. omnidiff deletes it
  (`APTED("qualified_name")`). The two identifiers share little text and the expressions differ in
  shape, so delete plus insert is also a fair reading.

Reasons for the "with children" mismatches re-read on 2026-10-13 after the test helper began
printing them.

## Verdict

- Primary cause: `matcher`
- Other causes: `ambiguous`
- Where a fix would go: twin selection for identical `value_argument`s in `APTED("qualified_name")`:
  prefer the candidate whose enclosing call is matched. Pairing a declaration that moved out of a
  removed closure needs a statement-level re-anchoring step.
- Ground truth questionable: no
