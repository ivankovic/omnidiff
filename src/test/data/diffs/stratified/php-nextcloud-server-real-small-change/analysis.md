# Accuracy analysis: php-nextcloud-server-real-small-change

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 297 nodes, 297 graded, N:M floor 0 visible |
| Painting, Minimal | 59 bytes wrong of 1223: renderer 59, matcher 0 |
| Painting, Full | 10 bytes wrong of 1223: renderer 10, matcher 0 |
| Ground-truth invariant violations | none |

## The change

`ApiFactory` switches to PHP 8 constructor property promotion: the `private $clientService;`
property with its docblock and the `$this->clientService = $clientService;` assignment are
deleted, and the constructor parameter becomes `private IClientService $clientService,` on its own
line (before lines 15-19 -> after lines 15-18). The mapping is perfect; every painting
disagreement is the same with the human mapping, so all are the renderer's.

## Painting

### Minimal, 59 bytes, Move -> None

`IClientService $clientService` (before line 18, 29 bytes; after line 16, 30 bytes including the
new trailing `,`). The parameter moved from the `__construct(` line onto a line of its own, and
omnidiff paints it Move; the Minimal painting treats that reflow as no change (the Full painting
does paint it Move, and omnidiff agrees there). On the after side the run also covers the new `,`,
which the Minimal painting marks Insert and omnidiff's Minimal punctuation filter leaves
unpainted. `renderer`.

### Full, 10 bytes, None -> Move / Insert

The Full painting accounts for the reflow byte by byte: the `(` of `__construct(` (Move, 1 byte
each side), the closing `\t}` of the constructor (Move, 2 bytes each side) and the new
indentation and space around `private` (Insert: `\t\t`, ` `, `\t`, 4 bytes). omnidiff leaves
these unpainted: the brackets and `}` are matched as the ground truth says but not painted Move,
and the new whitespace belongs to no node. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `diff::text`: in Minimal, no Move for a node that only moved to a new
  line inside the same, matched parameter list; in Full, painting of moved brackets and new
  indentation.
- Ground truth questionable: no
