# Accuracy analysis: java-jakartaee-rest-real-logic-change-of-a-significant-chunck

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 76 (50 visible) of 1569 nodes, 1569 graded, N:M floor 0 visible |
| Painting | none on file |
| Ground-truth invariant violations | none |

## The change

`jakarta.ws.rs.client.ClientBuilder.newInstance()` stops going through a `PrivilegedAction`. The
nested `CreateErrorMessageAction` class (before lines 275-296) and its `AccessController` /
`PrivilegedAction` imports are removed, and its `run()` body is inlined into the error branch of
`newInstance()` (after lines 62-71). The copyright year range is narrowed. No painting on file.

## Mapping

All 76 mismatches (50 visible) are of the "Delete/Insert (with children)" form. Their pass is
`APTED("qualified_name")` for all 76 (op Identical, MatchButNotIdentical or Update): the APTED call
that `solve_qualified_name_groups` (`solve_syntax_aware_matching.rs`) runs on a pair matched by
kind and scope-qualified name, here the enclosing `newInstance()` method or its class (the evidence
names the call site, not the pair).

- **Positional pairing of unrelated statements (50 visible, 26 invisible), `matcher`.** Inside the
  matched block of `if (!(delegate instanceof ClientBuilder))`, the ground truth deletes the three
  old statements and inserts the new ones. omnidiff pairs them one by one:
  - `final CreateErrorMessageAction action = new CreateErrorMessageAction(delegate);` (line 64) with
    `String classnameAsResource = pClass.getName().replace('.', '/') + ".class";` (line 63).
  - `final String errorMessage;` (line 65) with `ClassLoader loader = pClass.getClassLoader();`
    (line 64).
  - `if (System.getSecurityManager() == null) { errorMessage = action.run(); } else { ... }` (line
    66) with `if (loader == null) { loader = ClassLoader.getSystemClassLoader(); }` (line 65). At
    the leaves this gives `CreateErrorMessageAction` -> `String`, `action` -> `classnameAsResource`,
    `run` -> `getSystemClassLoader`, `errorMessage` -> `loader`, and the
    `if`/`==`/`null`/punctuation. The pairs share only statement kind, position and an `== null`
    shape. No name or value carries over.

The new statements are a copy of the deleted `run()` body (before lines 285-290, identical apart
from indentation; `return "..."` became `final String errorMessage = "..."`). Neither mapping
connects them. The ground truth deletes the nested class with all its children and inserts the new
statements. omnidiff does not find the move either: it inserts `Class<?> pClass = ...` and the
statements after the paired three, and deletes the class.

Reasons for the "with children" mismatches re-read on 2026-10-13 after the test helper began
printing them.

## Verdict

- Primary cause: `matcher`
- Other causes: `ground-truth`
- Where a fix would go: the `APTED("qualified_name")` call over the method matched by qualified
  name (`solve_qualified_name_groups`), which pairs statements inside the error-branch block; it
  should not pair statements that share no names; inline-method moves out of a deleted nested class
  are a separate gap
- Ground truth questionable: yes: the inlined run() body (before lines 285-290) and after lines
  62-67 are identical apart from indentation, but the ground truth deletes and inserts them instead
  of recording a move. This may be intended, since the fixture is named a logic change; it is
  Marko's call
