/*  This file is part of the OmniDiff code diffing tool.
 *
 *  Copyright (C) 2026 Marko Ivankovic
 *
 *  This program is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU Affero General Public License as published
 *  by the Free Software Foundation, version 3 of the License.
 *
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 *  GNU Affero General Public License for more details.
 *
 *  You should have received a copy of the GNU Affero General Public License
 *  along with this program. If not, see <https://www.gnu.org/licenses/>.
 */
//! Embedded layers: code of another language in one of the host's strings (TSX in a JSON
//! `"content"`), mapped and painted as a file of its own (`diff::embedded`,
//! `human_mapping::HumanEmbeddedLayer`).
//!
//! `Enter` on a host string opens its layer: the panels, the `t` view and omnidiff's run then show
//! the decoded pair - escapes resolved, the host's indentation removed - and every key works as it
//! does in a file. `q`/`Esc` goes back. The pair and language come from omnidiff's own layer at
//! the node when it found one, then from an existing human layer, then from the human's pairing of
//! the node and `diff::embedded::detect`. The session state of the file underneath is kept in a
//! [`LayerFrame`]; the decoded `Code`s live in `run_event_loop`, beside the host's.
//!
//! Back in the host, its `t` view draws each layer's painting where the string spells it
//! ([`LayerPaint`]), read-only: the layer's own painting is what grading reads there, so it is
//! changed by entering the layer, not by painting over the escaped text.

use crate::*;
use omnidiff::diff::embedded;
use omnidiff::test::helper::human_mapping::HumanEmbeddedLayer;

/// The layer `Enter` on `node` (on `side`) opens, or why there is none.
pub(crate) fn layer_request(
    app: &App,
    node: Node,
    side: Side,
    caches: &Caches,
    before: &Code,
    after: &Code,
) -> std::result::Result<LayerRequest, String> {
    let host = before.metadata.language.unwrap_or_default();
    if before.ast.is_none() || after.ast.is_none() {
        return Err("No trees to find a string in".to_string());
    }
    let node_cache = NodeCache::build(before, after);
    let (before_id, after_id) = match side {
        Side::Before => (Some(node.id()), None),
        Side::After => (None, Some(node.id())),
    };

    // omnidiff's own layer at this string, whichever side the cursor is on.
    if let Some(found) = app.algo_diff.as_ref().and_then(|diff| {
        diff.embedded
            .iter()
            .find(|layer| Some(layer.before_id) == before_id || Some(layer.after_id) == after_id)
    }) && let (Some(before_node), Some(after_node)) = (
        node_cache.before.get(&found.before_id),
        node_cache.after.get(&found.after_id),
    ) {
        return Ok(LayerRequest {
            before_path: path_for_node(*before_node),
            after_path: path_for_node(*after_node),
            language: found.language,
        });
    }

    // A layer the human already has, with this string on the cursor's side.
    let path = path_for_node(node);
    if let Some(layer) = app.mapping.embedded.iter().find(|layer| match side {
        Side::Before => layer.before_path == path,
        Side::After => layer.after_path == path,
    }) && let Some(language) = omnidiff::code::language::from_stable_name(&layer.language)
    {
        return Ok(LayerRequest {
            before_path: layer.before_path.clone(),
            after_path: layer.after_path.clone(),
            language,
        });
    }

    // The human's pairing, and the string's context.
    if !embedded::is_host_string(host, node) {
        return Err(format!(
            "Enter opens the code in a string; a {} is not a string {} can hold code in",
            node.kind(),
            omnidiff::code::language::human_name(host)
        ));
    }
    let partner_id = match side {
        Side::Before => caches.before_match.get(&node.id()),
        Side::After => caches.after_match.get(&node.id()),
    }
    .copied()
    .ok_or("Match this string with its partner first (m), then Enter opens the code in it")?;
    let (before_node, after_node) = match side {
        Side::Before => (node, node_cache.after.get(&partner_id).copied()),
        Side::After => (
            node_cache.before.get(&partner_id).copied().unwrap_or(node),
            Some(node),
        ),
    };
    let after_node = after_node.ok_or("The string's partner is not in the after tree")?;
    let language = embedded::detect(host, before_node, &before.contents)
        .or_else(|| embedded::detect(host, after_node, &after.contents))
        .ok_or("Nothing around this string says what language it holds")?;
    Ok(LayerRequest {
        before_path: path_for_node(before_node),
        after_path: path_for_node(after_node),
        language,
    })
}

/// The innermost string around `(row, column)` of `code` that can hold embedded code.
pub(crate) fn host_string_at<'c>(
    code: &'c Code,
    source: &str,
    row: usize,
    column: usize,
) -> Option<Node<'c>> {
    let host = code.metadata.language.unwrap_or_default();
    let offset = TextPaintState::byte_offset(source, row, column)?;
    let mut node = code
        .ast
        .as_ref()?
        .root_node()
        .descendant_for_byte_range(offset, offset)?;
    while !embedded::is_host_string(host, node) {
        node = node.parent()?;
    }
    Some(node)
}

/// Opens the layer `request` names: returns its decoded pair, and moves the host file's session
/// state into a new [`LayerFrame`].
pub(crate) fn enter_layer(
    app: &mut App,
    before: &Code,
    after: &Code,
    request: &LayerRequest,
) -> Result<(Code, Code)> {
    let (Some(before_tree), Some(after_tree)) = (before.ast.as_ref(), after.ast.as_ref()) else {
        bail!("no trees to find the strings in");
    };
    let before_node = node_for_path(before_tree.root_node(), &path_refs(&request.before_path))?;
    let after_node = node_for_path(after_tree.root_node(), &path_refs(&request.after_path))?;
    let host = before.metadata.language.unwrap_or_default();
    let decoded = embedded::layer(
        before,
        after,
        before_node,
        after_node,
        host,
        request.language,
    )
    .with_context(|| {
        format!(
            "the strings do not decode and parse as {}",
            omnidiff::code::language::human_name(request.language)
        )
    })?;

    let language_name = omnidiff::code::language::stable_name(request.language);
    let existing = app.mapping.embedded.iter().position(|layer| {
        layer.before_path == request.before_path && layer.after_path == request.after_path
    });
    let created = existing.is_none();
    let index = existing.unwrap_or_else(|| {
        app.mapping.embedded.push(HumanEmbeddedLayer {
            before_path: request.before_path.clone(),
            after_path: request.after_path.clone(),
            language: language_name.to_string(),
            mapping: HumanMapping::default(),
        });
        app.mapping.embedded.len() - 1
    });
    let inner = app.mapping.embedded[index].mapping.clone();
    // A plain-text layer (no language detected) has no name to give but that.
    let what = if request.language == Language::Unknown {
        "Text".to_string()
    } else {
        omnidiff::code::language::human_name(request.language)
    };
    let label = format!(
        "{what} in the string at line {}",
        before_node.start_position().row + 1
    );

    let inner_before_root = starting_cursor_id(&decoded.before);
    let inner_after_root = starting_cursor_id(&decoded.after);
    let text_solution = starting_solution(&inner);
    // Entered from the `t` view, the layer opens in its own; the host's comes back on leaving.
    let text_view = match app.modal.take() {
        Some(Modal::TextView { state }) => Some(state),
        other => {
            app.modal = other;
            None
        }
    };
    if text_view.is_some() {
        app.modal = Some(Modal::TextView {
            state: TextPaintState::default(),
        });
    }
    let frame = LayerFrame {
        text_view,
        mapping: std::mem::replace(&mut app.mapping, inner),
        index,
        created,
        label: label.clone(),
        focus: app.focus,
        before: std::mem::replace(&mut app.before, PanelState::new(inner_before_root)),
        after: std::mem::replace(&mut app.after, PanelState::new(inner_after_root)),
        undo_stack: std::mem::take(&mut app.undo_stack),
        redo_stack: std::mem::take(&mut app.redo_stack),
        algo_diff: app.algo_diff.take(),
        text_solution: std::mem::replace(&mut app.text_solution, text_solution),
    };
    app.layers.push(frame);
    app.focus = Focus::Before;
    app.algo_diff_pending = None;
    app.algo_text_spans = None;
    app.text_overlay = TextOverlay::default();
    app.clear_multi_select();
    app.status = Some(format!(
        "Opened {label}: map and paint it as a file of its own; q or Esc goes back"
    ));
    Ok((decoded.before, decoded.after))
}

/// Closes the innermost layer, writing its mapping back into the host's; a layer opened here and
/// left with nothing in it is dropped again.
pub(crate) fn leave_layer(app: &mut App) {
    let Some(frame) = app.layers.pop() else {
        return;
    };
    let inner = std::mem::replace(&mut app.mapping, frame.mapping);
    let untouched = inner.entries.is_empty()
        && inner.groups.is_empty()
        && inner.text_mappings.is_empty()
        && inner.embedded.is_empty();
    if frame.created && untouched {
        app.mapping.embedded.remove(frame.index);
    } else {
        app.mapping.embedded[frame.index].mapping = inner;
    }
    app.focus = frame.focus;
    app.before = frame.before;
    app.after = frame.after;
    app.undo_stack = frame.undo_stack;
    app.redo_stack = frame.redo_stack;
    app.algo_diff = frame.algo_diff;
    app.algo_diff_pending = None;
    app.algo_text_spans = None;
    app.text_solution = frame.text_solution;
    if let Some(state) = frame.text_view {
        app.modal = Some(Modal::TextView { state });
    }
    app.text_overlay = TextOverlay::default();
    app.clear_multi_select();
    app.status = Some(format!("Back from {}", frame.label));
}

/// One painted layer's paintings, projected onto the host files for the host's `t` view.
#[derive(Debug, Clone, Default)]
pub(crate) struct LayerPaint {
    /// Per painting name, both sides' painted spans in host rows and columns.
    pub(crate) paintings: Vec<(String, PaintedSpans)>,
}

/// Both sides' painted spans with their verdicts, as `painted_spans` gives one side's.
pub(crate) type PaintedSpans = [Vec<(HumanTextSpan, HumanTextVerdict)>; 2];

impl LayerPaint {
    /// The layer's painting named `solution`, else its first: the host and the layer need not use
    /// the same names.
    pub(crate) fn for_solution(&self, solution: &str) -> Option<&PaintedSpans> {
        self.paintings
            .iter()
            .find(|(name, _)| name == solution)
            .or(self.paintings.first())
            .map(|(_, spans)| spans)
    }
}

/// Every painted layer of `mapping`, projected onto `before`/`after`; a layer whose strings no
/// longer decode is left out.
pub(crate) fn project_layer_paint(
    mapping: &HumanMapping,
    before: &Code,
    after: &Code,
) -> Vec<LayerPaint> {
    mapping
        .embedded
        .iter()
        .filter(|layer| !layer.mapping.text_mappings.is_empty())
        .filter_map(|layer| {
            let decoded = layer.resolve(before, after).ok()?;
            let sides = [
                (
                    &decoded.before.contents,
                    &decoded.before_offsets,
                    &before.contents,
                ),
                (
                    &decoded.after.contents,
                    &decoded.after_offsets,
                    &after.contents,
                ),
            ];
            let paintings = layer
                .mapping
                .text_mappings
                .iter()
                .map(|named| {
                    let mut spans: PaintedSpans = [Vec::new(), Vec::new()];
                    for entry in &named.mapping.entries {
                        let Ok(verdict) =
                            entry.verdict(&decoded.before.contents, &decoded.after.contents)
                        else {
                            continue;
                        };
                        for (side, inner) in [&entry.before, &entry.after].into_iter().enumerate() {
                            let (text, offsets, host) = sides[side];
                            for span in inner {
                                let start = TextPaintState::byte_offset(
                                    text,
                                    span.start_row,
                                    span.start_column,
                                );
                                let end = TextPaintState::byte_offset(
                                    text,
                                    span.end_row,
                                    span.end_column,
                                );
                                let (Some(start), Some(end)) = (start, end) else {
                                    continue;
                                };
                                spans[side].extend(
                                    spans_of_bytes(host, offsets[start]..offsets[end])
                                        .into_iter()
                                        .map(|span| (span, verdict)),
                                );
                            }
                        }
                    }
                    (named.name.clone(), spans)
                })
                .collect();
            Some(LayerPaint { paintings })
        })
        .collect()
}

/// `range` of `source` as one span per row it touches, line breaks left out.
fn spans_of_bytes(source: &str, range: std::ops::Range<usize>) -> Vec<HumanTextSpan> {
    let mut spans = Vec::new();
    let mut line_start = 0;
    for (row, line) in source.split('\n').enumerate() {
        let line_end = line_start + line.len();
        let (from, to) = (range.start.max(line_start), range.end.min(line_end));
        if from < to {
            spans.push(HumanTextSpan {
                start_row: row,
                start_column: from - line_start,
                end_row: row,
                end_column: to - line_start,
            });
        }
        if line_end >= range.end {
            break;
        }
        line_start = line_end + 1;
    }
    spans
}
