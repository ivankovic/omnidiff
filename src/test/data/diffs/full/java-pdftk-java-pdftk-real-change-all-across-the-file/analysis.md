# Accuracy analysis: java-pdftk-java-pdftk-real-change-all-across-the-file

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 348 (237 visible) of 8987 nodes, 8987 graded, N:M floor 0 visible |
| Painting | none on file |
| Ground-truth invariant violations | none |

## The change

pdftk-java's `RandomAccessFileOrArray` is synced with iText 4.2.0. Changes: a memory-mapped
`MappedRandomAccessFile rf` plus a plain `RandomAccessFile trf` and a `plainRandomAccess` flag; the
private `inputStreamToArray` becomes `public static byte[] InputStreamToArray` and moves above the
constructors; the call sites become `this.arrayIn = InputStreamToArray(is)`; the copy constructor is
simplified; `read()` is rewritten with early returns; `readFully` is reordered and rewritten as a
do-while; `skip`/`skipBytes` are added; `bb` is renamed to `b` in `read(byte[], int, int)`; imports
are re-sorted and the license header shortened. No painting on file.

## Mapping

307 of the 348 mismatches (206 of 237 visible) come from one call site, APTED("large_flat_subtree"),
the APTED that `solve_large_flat_subtrees` runs over this large class body. 40 (30 visible), all
printed on "with children" lines, come from `MovedSubtree` (`solve_moved_subtrees`, which runs
later and pairs a fully deleted subtree with a byte-identical fully inserted one). It takes three
small fragments that APTED left unmapped on both sides, `out.toByteArray()`, `(filename, "r")` and
`return retVal;`, and pairs them across methods. One is `UnresolvedNode`. Rows are 1-based. Groups,
largest first:

- **`inputStreamToArray` method moved and rewritten (46 visible, 20 invisible), `matcher`.** The
  ground truth maps the old `private void inputStreamToArray(InputStream is)` (lines 196-205) to the
  new `public static byte[] InputStreamToArray` (lines 152-163). This is a rename, a modifier change
  and a rewritten loop (`bb` -> `b`, `len` -> `read`). The method also moved above the `byte[]` and
  copy constructors. omnidiff deletes it and inserts the new one. The one piece kept,
  `out.toByteArray()` (line 203 -> 162, 10 visible + 4 invisible), is not APTED's: `MovedSubtree`
  pairs the two leftover copies as a move. The ground truth deletes and inserts it, because
  `this.arrayIn = ...` became `return ...`. APTED preserves order. With the constructors matched in
  between, it cannot also keep the method that crossed them.
- **`readFully` overloads swapped and rewritten (44 visible, 20 invisible), `matcher`.**
  `readFully(bb, off, len)` (lines 312-320) and `readFully(bb[])` (lines 322-324) swap order (after
  lines 276-278 and 280-289), and the while loop becomes a do-while (`bytesRead` -> `n`, `nn` ->
  `count`). The ground truth follows the long overload and its statements. omnidiff deletes most of
  the body and maps single leaves (`int`, `0`, `=`) piecemeal to other declarations.
- **forceRead block reformatted (40 visible, 21 invisible), `matcher`.** The `else if (forceRead)`
  body (lines 133-142) is the same `InputStream ... = null; try { ... } finally { ... }` code with
  `is` renamed to `s`, compressed spacing in the `finally`, and the call rewritten. The ground truth
  maps it token by token, with `is` -> `s` as Updates. omnidiff deletes all of it.
- **Wrong twins across methods (25 visible, 10 invisible), `matcher`.** The constructor's new `rf =
  new MappedRandomAccessFile(filename, "r");` (after line 131) takes its `(filename, "r")` pieces
  from `rf = new RandomAccessFile(filename, "r");` in `seek(int)` (before line 394). That part (14
  visible, 4 invisible) is `MovedSubtree`: APTED left both argument lists unmapped, and the move
  pass paired the byte-identical copies across methods. The constructor's own old `rf = new
  RandomAccessFile(file, "r");` (line 144) is deleted. Inside `seek(int)`, the old `;` and
  `IOException` (lines 394, 396) pair with `insureOpen();` and `trf` (after lines 370, 372). The
  ground truth keeps each statement with its own method.
- **Copy constructor (14 visible, 9 invisible), `matcher`.** `this.startOffset = file.startOffset;`
  (line 189) is mapped to `plainRandomAccess = file.plainRandomAccess;` (line 173), and
  `this.arrayInPtr = 0`'s `arrayInPtr` to `startOffset`. The ground truth maps the identical
  `startOffset = file.startOffset` and deletes the `arrayInPtr` reset.
- **Call sites (8 visible; plus 2 stray invisible nodes on lines 108 and 351), `matcher`.** At lines
  115, 126, 159 and 167, `this.inputStreamToArray(is)` -> `this.arrayIn = InputStreamToArray(is)`.
  The ground truth updates `inputStreamToArray` -> `InputStreamToArray` and keeps the `.`. omnidiff
  deletes both.
- **`clearBack()` moved inside `close()` (8 visible, 6 invisible), `matcher`.** The old trailing
  `clearBack();` (line 357) moved to the top of `close()`. omnidiff pairs it with `trf.close();`
  (line 352, `clearBack` -> `trf` as an Update) instead of following it.
- **`read()` restructured (28 visible, 13 invisible), `ambiguous`.** `int retVal = -1; ... return
  retVal;` (lines 237-249) becomes early returns (after lines 206-217). The ground truth maps the
  old declaration onto `int retVal = getBack();`, deletes the final `return retVal;` and the old
  `else if (arrayIn != null && ... arrayInPtr< arrayIn.length)`, and inserts the new `if (arrayInPtr
  >= arrayIn.length)`. omnidiff maps the final return to the new early `return retVal;` (line 249
  -> 209, `MovedSubtree`, 6 visible + 2 invisible, after APTED left both unmapped) and, through
  APTED, the bounds test to the new inverted one. Each pairing has a reading.
- **`reOpen()` (24 visible, 10 invisible), `ambiguous`.** The old `rf = new
  RandomAccessFile(filename, "r");` (line 329) becomes two branches. The ground truth follows the
  variable, to `rf = new MappedRandomAccessFile(filename, "r");` (`RandomAccessFile` updated).
  omnidiff follows the class, to `trf = new RandomAccessFile(filename, "r");` (line 319, `rf` ->
  `trf`). Both share 11 of 12 tokens with the old line.

Reasons for the "with children" mismatches re-read on 2026-10-13 after the test helper began
printing them.

## Verdict

- Primary cause: `matcher`
- Other causes: `ambiguous`
- Where a fix would go: `solve_large_flat_subtrees` (`APTED("large_flat_subtree")`): one
  order-preserving APTED over the whole class body cannot follow reordered methods
  (`inputStreamToArray`, `readFully`); `solve_moved_subtrees` then pairs its small identical
  leftovers (`out.toByteArray()`, `(filename, "r")`, `return retVal;`) with distant twins;
  renamed/reformatted blocks (forceRead) would need a similarity-based subtree match before it
- Ground truth questionable: no
