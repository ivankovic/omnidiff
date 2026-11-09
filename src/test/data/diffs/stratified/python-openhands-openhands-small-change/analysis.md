# Accuracy analysis: python-openhands-openhands-small-change

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 301 nodes, 301 graded, N:M floor 0 visible |
| Painting, Minimal | 115 bytes wrong of 1473: renderer 115, matcher 0 |
| Painting, Full | 127 bytes wrong of 1473: renderer 127, matcher 0 |
| Ground-truth invariant violations | none |

## The change

In `enterprise/integrations/manager.py`:

- `from typing import Any` is added;
- `send_message(self, message: Message)` becomes
  `send_message(self, message: str, *args: Any, **kwargs: Any)`;
- its one-line docstring `"Send message to integration from Openhands server"` becomes a
  triple-quoted docstring. The new docstring says `OpenHands` and ends with `.`, and gains an
  `Args:` section (after lines 17-21);
- `create_outgoing_message` is deleted.

The mapping agrees with the ground truth everywhere. Every run is the same with the human mapping.

## Painting

- **Added docstring lines painted Update, not Insert (82 bytes, both presets), `renderer`.** After
  lines 19-20 (`Args:` and `message: The message content ...`) are new text inside the one
  `string_content` leaf that maps old -> new. omnidiff's narrowed update paints them Update, and in
  Minimal it also paints their indentation. Both paintings mark them Insert.
- **String delimiters left unpainted (8 bytes Minimal, 16 Full), `renderer`.** Both paintings
  mark the opening `"` -> `"""` (before line 16, after line 17) and the closing `"` -> `"""` (after
  line 21) as Update. omnidiff leaves the delimiters unpainted on both sides. In Full the
  indentation of the new closing `"""` line (Insert in the painting) is also unpainted.
- **`):` of the def line painted Move (4 bytes, Full), `renderer`.** The inserted parameters and
  the inserted import displace the identical `):`. omnidiff paints it Move. The painting leaves the
  identical line tail unpainted (rule 3). This is the same Full displacement gating issue as in
  `python-langflow-ai-langflow-real-small-change`.
- **Content change not in the paintings (25 bytes, both presets), `ground-truth`.** omnidiff paints
  `hands server` -> `Hands server.` as Update: `Openhands` -> `OpenHands`, plus the added period.
  Both paintings leave the old and new docstring text unpainted. Only the delimiters and the new
  `Args:` lines are painted, so the case change and the period are not recorded anywhere. This
  looks like an omission in the painting.

## Verdict

- Primary cause: `renderer`
- Other causes: `ground-truth`
- Where a fix would go: update narrowing inside multi-line string leaves in src/diff/text.rs. Label
  whole added lines Insert, paint changed string delimiters (`string_start`/`string_end`), and use
  the Full displacement gating for `):`. The `Openhands` case change is Marko's call.
- Ground truth questionable: yes. Neither painting marks the `Openhands` -> `OpenHands` case
  change or the added `.` in the docstring.
