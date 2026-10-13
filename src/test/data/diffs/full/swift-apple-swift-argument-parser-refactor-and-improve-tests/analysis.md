# Accuracy analysis: swift-apple-swift-argument-parser-refactor-and-improve-tests

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 63 (33 visible) of 1806 nodes, 1803 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

In `Tests/ArgumentParserUnitTests/ExitCodeTests.swift`, `testNSErrorIsHandled` moves the error
message literal into a new `static let message` (after lines 88-89). `NSError` gets
`domain: "TestError"` and `userInfo: [NSLocalizedDescriptionKey: Self.message]`. The single final
`XCTAssertEqual(NSErrorCommand.message(for: ...), "The file ...")` (before lines 98-100) becomes
three variants under `#if canImport(FoundationEssentials)` / `#if compiler(<6.1)`:

- after lines 103-105, expecting `"\(prefix)"`;
- after lines 107-110, expecting `"\(prefix)UserInfo={...}"`;
- after lines 113-115, expecting `NSErrorCommand.message`.

The first two lines of all three are identical to the before call. description.md: "Requires an
N:M mapping (2:1)", the two copies of the message literal against the one `static let`.

## Mapping

- **Which of three identical calls is the old one (28 visible, 26 invisible), `ambiguous`.** The
  ground truth keeps the before `XCTAssertEqual` call (line 98) as the first variant (after line
  103), which matches a line diff. omnidiff (`APTED("qualified_name")`) keeps it as the second
  variant (after line 107). Every leaf of the shared `XCTAssertEqual(` /
  `NSErrorCommand.message(for: NSErrorCommand.fileNotFoundNSError),` lines therefore lands one copy
  off, and the after-107 leaves the ground truth calls new are mapped. All three candidates are
  byte-identical in those lines. Each choice shows one edited call and two new ones, so both are
  defensible.
- **Old message literal updated into an unrelated literal (5 visible, 4 invisible), `matcher`.**
  The ground truth's 2:1 group says the before literal `"The file “foo/bar” couldn’t be opened ..."`
  at line 100 belongs with the new `static let message` literal (after line 89) or is deleted. Its
  twin at before line 92 takes the one-to-one slot, and the floor is 0. omnidiff updates it into
  the after line 109 literal `"\(prefix)UserInfo={NSLocalizedDescription=\(...)}"`
  (MatchButNotIdentical, `APTED("qualified_name")`), with quotes and content mapped across. The two
  strings share only their quotes. APTED pairs them because they sit at the same position in the
  chosen call.

## Verdict

- Primary cause: `ambiguous`
- Other causes: `matcher`
- Where a fix would go: none for the twin choice. For the literal, the update cost in
  `APTED("qualified_name")` (src/diff/solve_syntax_aware_matching.rs) should refuse to pair string
  literals that share no content.
- Ground truth questionable: no
