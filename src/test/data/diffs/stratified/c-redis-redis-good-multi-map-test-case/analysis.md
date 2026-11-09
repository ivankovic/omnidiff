# Accuracy analysis: c-redis-redis-good-multi-map-test-case

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 330 (219 visible) of 63242 nodes, 63265 graded, N:M floor 15 visible |
| Painting, Minimal | 766 bytes wrong of 241098: renderer 117, matcher 683 |
| Painting, Full | 1180 bytes wrong of 241098: renderer 319, matcher 963 |
| Ground-truth invariant violations | 7 (one `lp` -> `eptr` rename left unpainted in both paintings, one Full whitespace gap) |

## The change

`src/listpack.c`: `lpCompare` gets two new parameters (`cached_longval`, `cached_valid`). The old
`if (lpStringToInt64(..., &sval)) return sz == sval;` is now in two places: a caching branch with a
changed argument and body (line 1738), and an unchanged copy in the `else` branch (line 1747). All
call sites add `, NULL, NULL`. The "with number" benchmark changes `lpCompare(lp, ...` to
`lpCompare(eptr, ...` (line 3267 -> 3281), and a new benchmark, "Benchmark lpCompare with number
and caching", is added after it (lines 3288-3300).

## Mapping

- **Benchmark blocks paired one step late (201 visible, 100 invisible), `matcher`.** The ground
  truth maps the before "with number" benchmark (lines 3262-3272) to the after "with number"
  benchmark (lines 3276-3286), and the closing `printf`/`}` of the "with string" benchmark (lines
  3259-3260) to lines 3273-3274. The new caching benchmark (lines 3288-3300) is an insertion.
  omnidiff instead maps lines 3259-3260 to 3285-3286, and the whole before "with number" benchmark
  to the new caching benchmark at lines 3288-3300. This includes `TEST("Benchmark lpCompare with
  number")` -> `"... with number and caching"` as an Update, and `lp` -> `eptr` at line 3295. The
  after "with number" benchmark is then left as an insertion, although it differs from the old one
  only by `lp` -> `eptr` and `, NULL, NULL`. The reasons are `APTED("large_flat_subtree")`, which
  is the flat alignment of the children of `main`'s test body (`compound_statement:51`), and
  `IdenticalHash`/`IdenticalHashOfAncestor` for the two `printf` lines. Those `printf` lines are
  byte-identical in every benchmark, so the identical-hash twin choice has many candidates. The
  `TEST("Benchmark lpCompare with number")` line is byte-identical on both sides, but the
  alignment did not use it as an anchor.
- **The `if` keyword of the moved statement goes to the new wrapper (3 visible, 3 invisible),
  `matcher`.** The ground truth puts before line 1734 `if (lpStringToInt64(...&sval))` in a
  1-before/2-after all-to-all group with after lines 1738 and 1747. The wrapping
  `if (cached_valid != NULL)` at after line 1735 is new. omnidiff maps the rest of the statement
  into the group, but maps its `if` keyword to the line 1735 `if`, through
  `APTED("qualified_name")`. This is the reason on before 1734, after 1735 and after 1747.
- **The second copy of the statement (15 visible, 8 invisible), `nm-floor`.** After line 1738 is
  the group's second after copy. A one-to-one mapping can only pair one copy, and these 15 visible
  mismatches are exactly the fixture's N:M floor.

## Painting

Matcher-caused (runs that are absent when the human mapping is rendered):

- Benchmark shift, both presets. Lines 3273-3284 (the "with string" `printf`/`}` and the whole
  "with number" benchmark) are painted Insert where the painting leaves them alone (Insert ->
  None). Lines 3288-3300 (the new caching benchmark) are left unpainted where the painting marks
  them Insert (None -> Insert). This is 521 bytes in Minimal and 826 in Full. In Full it includes
  the `") {` on before line 3262 (Move -> None).
- lpCompare moved statement, both presets. omnidiff leaves before line 1734 unpainted where the
  painting has Move. It paints the two after copies (lines 1738 and 1747) Insert where the painting
  has Move, and leaves the new wrapper's `if ` on line 1735 unpainted (None -> Insert). This is 128
  bytes in Minimal and 61 in Full.

Renderer-side (also present with the human mapping):

- N:M move of the `if (lpStringToInt64(...&sval)) return sz == sval;` statement (`nm-floor`). With
  the human mapping, the renderer pairs the before statement with one after copy. ` &sval))` and
  `return sz == sval;` (before lines 1734-1735, after lines 1747-1748) and `))` on line 1738 stay
  unpainted where the painting has Move. This is 54 bytes in Minimal. In Full the affected text is
  the indentation of the moved lines: about 70 bytes, None -> Move/Insert and Move -> Insert.
- `lp` -> `eptr` on lines 3267/3281 (Update -> None). Both paintings leave the rename unpainted,
  which invariants [12]/[16] flag (`ground-truth`, 6 bytes). The renderer also widens the Update
  to the whole argument run `lp, (unsigned char*)"99999", 5` (`renderer`, 56 bytes in Minimal, 60
  in Full).
- Lone `,` at the end of after line 1721, before the wrapped new parameters, is dropped by
  Minimal's punctuation filter (None -> Insert, 1 byte, `renderer`).
- Full only (`renderer`). The closing `)`, `));`, `)) {` and `) {` after the inserted `, NULL,
  NULL` and the new parameter line are painted Move as displaced tails (52 bytes). The unchanged
  body `lp = lpDelete(lp, p, &p); }` of the `if` on line 2542/2556 is painted Move because its
  condition grew (108 bytes). The 23-space continuation indent on after line 1722 is painted Insert
  (23 bytes). The Full painting leaves all of these unpainted.

## Verdict

- Primary cause: `matcher`
- Other causes: `nm-floor`, `renderer`, `ground-truth`
- Where a fix would go: `solve_large_flat_subtrees` (flat sibling alignment of the test body). It
  should anchor on the byte-identical `TEST("Benchmark lpCompare with number")` line rather than
  shift the changed benchmark onto the new one. The identical-hash twin choice for repeated
  `printf` lines should follow that alignment. A smaller part is `APTED("qualified_name")`, which
  places the `if` keyword under the new wrapper. On the renderer side: the Full displacement Move
  of tails and unchanged `if` bodies, and the widening of a one-identifier Update to the whole
  argument run.
- Ground truth questionable: yes. Both paintings leave `lp` -> `eptr` on before line 3267 / after
  line 3281 unpainted, while the mapping pairs the two identifiers as an update (invariants
  [12]/[16]). The Full painting also leaves one space unpainted between two painted regions on
  after line 1738 (invariant [5]).
