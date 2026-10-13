# Accuracy analysis: csharp-lidarr-lidarr-small-but-complex-mapping

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 18 (10 visible) of 2449 nodes, 2458 graded, N:M floor 6 visible |
| Painting, Minimal | 155 bytes wrong of 11710: renderer 81, matcher 88 |
| Painting, Full | 165 bytes wrong of 11710: renderer 83, matcher 110 |
| Ground-truth invariant violations | 4 (all rule [16], Minimal painting paints whole identifiers on line 154 and 158) |

## The change

`HandleAsync(ProviderDeletedEvent<TProvider>)` used to look the status up with `FindByProviderId`,
null-check it and call `Delete(providerStatus)` (before lines 154-159). The commit replaces all of
it with one call, `_providerStatusRepository.DeleteByProviderId(message.ProviderId);` (after line
154). The ground truth maps this N:M: both before calls, `FindByProviderId(...)` and `Delete(...)`,
form an all-to-all group with the single after call, receivers, `.`, `(`, `)` and `;` included.

## Mapping

- N:M floor, 6 visible (plus about half the invisible ancestors): each all-to-all group is 2
  before <-> 1 after, so a one-to-one mapping must leave one before node of each group unmapped.
  `nm-floor`.
- Matcher, the other 4 visible: omnidiff maps neither before receiver. Both
  `_providerStatusRepository` and `.` on before lines 154 and 158 are Delete, and the after
  `_providerStatusRepository` and `.` on line 154 are Insert, all with reason
  `APTED("qualified_name")` (the whole-method APTED call of `solve_qualified_name_groups` in
  src/diff/solve_syntax_aware_matching.rs). The same call does carry the method name
  (`FindByProviderId` -> `DeleteByProviderId`, an update) and the argument list
  `(message.ProviderId)` from line 154 across, but leaves their parents (`invocation_expression`,
  `member_access_expression`) unmapped (invisible mismatches). So the after call is assembled
  from pieces of before line 154 while its receiver, which is byte-identical on both sides, is
  reported as deleted and re-inserted. A one-to-one mapping could have kept the receiver of line
  154. Why the edit-distance solution drops the receiver is not visible in the evidence; it was
  not traced further. `matcher`.

## Painting

Minimal, 155 bytes:

- Matcher, about 79 bytes (Delete/Insert -> Move): the receivers `_providerStatusRepository.` on
  before line 154 (26), before line 158 together with `Delete(` (33, of which `.Delete(` is 8
  bytes that also disagree with the human mapping), the `);` of line 158 (2), and the after
  receiver on line 154 (26, Insert -> Move). These follow from the unmapped receivers above.
  `matcher`.
- Painting, 66 bytes (None -> Update/Move): `ByProviderId(message.ProviderId);` on before and
  after line 154, which survive with the human mapping. The painting paints the whole
  identifier `FindByProviderId`/`DeleteByProviderId` as the Update where omnidiff narrows to the
  differing word (`Find`/`Delete`); invariant [16] flags exactly this as wrong for a Minimal
  reading. The rest of the run, `message.ProviderId);`, is an identical line tail on the same row
  that the painting records as matched (Move) and omnidiff leaves unpainted, which is what rule 3
  of text_painting_findings.md says a painter does. `ground-truth`.
- Renderer, 15 bytes: `{` and `}` of the deleted if block (lines 157, 159) dropped by the Minimal
  punctuation filter (2); with the human mapping omnidiff paints `.`, `.Delete(` inside the N:M
  match as Move where the painting has Update, and leaves `);` of line 158 and a space on line 154
  unpainted (13). `renderer`.

Full, 165 bytes:

- Matcher, about 82 bytes: the same receivers and `);` (26 + 29 + 2 + 26) Delete/Insert ->
  Move. `matcher`.
- Renderer, 83 bytes, all also present with the human mapping: the indentation of before line 154
  and after line 154 (12 + 12), which the Full painting matches as Move, and the indentation of
  before line 158 (16), which it paints Delete; omnidiff leaves unchanged indentation unpainted
  (40). `(message.ProviderId` on both sides of line 154 (19 + 19): the painting puts `(` in the
  Update and the identical argument in a Move; omnidiff, with the human mapping, paints it Move
  from `(`, and with its own mapping leaves it unpainted. Four single bytes (`.`, `(`, space).
  `renderer`.

## Verdict

- Primary cause: `matcher`
- Other causes: `nm-floor`, `ground-truth`, `renderer`
- Where a fix would go: the method-body APTED call of `solve_qualified_name_groups`
  (src/diff/solve_syntax_aware_matching.rs), which leaves the identical receiver unmapped; the
  painting's whole-identifier Updates are ground truth (Marko's call)
- Ground truth questionable: yes: the Minimal painting paints whole identifiers on lines 154/158 (4
  invariant [16] violations) and records the identical line tail `message.ProviderId);` of line 154
  as painted, against rule 3 of text_painting_findings.md
