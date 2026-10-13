# Accuracy analysis: cpp-nzbgetcom-nzbget-interesting-case-of-qualified-identifiers

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 14 (10 visible) of 2205 nodes, 2205 graded, N:M floor 0 visible |
| Painting, Minimal | 1632 bytes wrong of 8013: renderer 1572, matcher 120 |
| Painting, Full | 1392 bytes wrong of 8013: renderer 1372, matcher 100 |
| Ground-truth invariant violations | none |

## The change

The test moves from `std::filesystem` to the project's own helpers.
`std::filesystem::current_path().string()` becomes `TestUtil::WorkingDir()` (line 31), and
`std::filesystem::create_directory(x)` / `copy(a, b)` become
`FileSystem::CreateDirectory(x.c_str())` / `CopyFile(a.c_str(), (b).c_str())`. The license header
gains a Copyright line and an `https`. The includes change. description.md: "Two parts of qualified
identifier should match a single identifier" (`std::filesystem` -> `FileSystem`).

## Mapping

- **Rename or replace (10 visible, 4 invisible, `ambiguous`, `APTED("large_flat_subtree")`).** The
  ground truth deletes `filesystem::current_path()` (line 30) and inserts `TestUtil::WorkingDir()`
  (line 31). Omnidiff updates `filesystem` -> `TestUtil` and `current_path` -> `WorkingDir`, and
  keeps `::` and `()`. Both are zero-argument qualified calls in the same slot, so a rename of the
  callee is a defensible reading, though no text is shared. Marko's reading is that the call was
  replaced. The `std::filesystem` -> `FileSystem` cases the description is about are not in the
  mismatch list: omnidiff matches them as the ground truth wants.

## Painting

- **License comment painted Update (renderer, about 1262 bytes in Minimal).** The header is one
  `comment` leaf. One line is inserted inside it (after line 5) and `http` becomes `https` (line
  17/18). Omnidiff paints the comment Update from the line after the insertion to the last change
  (before lines 6-17, after lines 6-18). The inserted line itself is not painted Insert. The
  painting has an Insert of the new line and an Insert of the `s`. This is the largest share of the
  fixture. The runs are the same with the human mapping.
- **Re-wrapped arguments and the collapsed qualifier (renderer, the rest).** `(dupe1` /
  `testDataDir + "/parchecker"` (lines 40/44/52, before) become `dupe1.c_str()` /
  `(testDataDir + "/parchecker").c_str()`. Omnidiff paints the unchanged parts Move. The painting
  leaves them unpainted, and Full adds a few spaces Move. For `std::filesystem::copy` ->
  `FileSystem::CopyFile`, the human rendering paints `::filesystem` Delete. The painting folds
  `std::filesystem` into the Update to `FileSystem`. In Full, `+ "/rarrenamer";` (line 30/31) is
  painted Move.
- **Matcher (Minimal 120, Full 100 bytes, `ambiguous`).** `filesystem::current_path()` /
  `TestUtil::WorkingDir()` is painted Update instead of Delete and Insert.

## Verdict

- Primary cause: `renderer`
- Other causes: `ambiguous`
- Where a fix would go: `src/diff/text.rs`. Line-level narrowing inside a changed comment leaf would
  show an inserted line as Insert. Move display of arguments re-wrapped in a call. Painting of a
  two-part qualifier that collapses into one identifier.
- Ground truth questionable: no
