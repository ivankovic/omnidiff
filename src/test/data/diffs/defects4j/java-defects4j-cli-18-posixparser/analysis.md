# Accuracy analysis: java-defects4j-cli-18-posixparser

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 16 (10 visible) of 2076 nodes, 2076 graded, N:M floor 0 visible |
| Painting, Minimal | 27 bytes wrong of 20560: renderer 18, matcher 41 |
| Painting, Full | 25 bytes wrong of 20560: renderer 48, matcher 73 |
| Ground-truth invariant violations | 4 |

## The change

Defects4J Cli-18 in `PosixParser`: for a lone `-` token, `processSingleHyphen(token);` (line 128)
becomes `tokens.add(token);`, the now-unused `processSingleHyphen(String)` method (before lines
213-216) is removed, and `processOptionToken` adds the token when `stopAtNonOption` is set (after
line 237).

## Mapping

All 16 mismatches have reason `APTED("qualified_name")`.

- **`(token);` of the call on line 128 (8 visible, 6 invisible).** The ground truth's tree deletes
  `processSingleHyphen(token);` whole and inserts `tokens.add(token);` whole. omnidiff pairs the
  two statements and keeps `(`, `token`, `)` and `;`. Both paintings agree with omnidiff: they
  leave `(token);` unpainted on both sides. `invariants.txt` reports this as 4 violations of
  invariant 11 (one per painting and side): "leaves 1 removed leaf unpainted on row 128 ...
  `token` ... the tree mapping says this is gone". `ground-truth`.
- **The callee name (2 visible).** omnidiff pairs `processSingleHyphen` (before) with the receiver
  `tokens` (after) as an Update (`APTED("qualified_name")`, which pairs the two statements and with
  them their first identifiers). The tree and both paintings delete one and insert the other: a
  method name and a receiver object are different things. `matcher`.

Reasons for the "with children" mismatches re-read on 2026-10-13 after the test helper began
printing them.

## Painting

Matcher-caused (absent with the human mapping), 25 bytes in each preset:

- Update -> Delete/Insert: `processSingleHyphen` (before line 128, 19 bytes) and `tokens` (after
  line 128, 6 bytes), painted Update because omnidiff pairs them; the paintings paint them Delete
  and Insert.

Renderer-caused (also with the human mapping):

- Minimal 2 bytes, None -> Delete: `{` and `}` of the removed `processSingleHyphen` method (before
  lines 214 and 216). The Minimal painting paints them; `MINIMAL` leaves structural punctuation
  unpainted. `renderer`.

With the human mapping the renderer would also paint `(token);` Delete/Insert (and in Full its
indentation), against the paintings; that is the tree/painting conflict above, not in `real`.

## Verdict

- Primary cause: `ground-truth`
- Other causes: `matcher`, `renderer`
- Where a fix would go: ground truth (Marko's call) for the `(token);` conflict;
  `APTED("qualified_name")` for the callee
- Ground truth questionable: yes: the tree deletes and inserts `(token);` on line 128 while both
  paintings keep it unpainted (4 invariant-11 violations)
