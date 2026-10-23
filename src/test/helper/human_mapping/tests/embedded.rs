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
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU Affero General Public License for more details.
 *
 *  You should have received a copy of the GNU Affero General Public License
 *  along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */
//! The `embedded` layer of a human mapping: stored, loaded, turned into an [`ASTDiff`] and graded,
//! on the shadcn registry fixture whose JSON string holds a TSX file.
use super::*;
use crate::diff::text::{TextDiff, TextOperation};

const SHADCN: &str = "json-shadcn-ui-ui-react-code-in-string-constant";

/// The JSON string holding the TSX, on each side.
const HOST_PATH: &[&str] = &[
    "object:1", "pair:4", "array:1", "object:1", "pair:2", "string:2",
];

/// `diff` as human entries, one per mapped pair, for a ground truth that agrees with omnidiff.
fn entries_of(diff: &ASTDiff, before: &Code, after: &Code) -> Vec<HumanMappingEntry> {
    let cache = NodeCache::build(before, after);
    let path = |nodes: &rustc_hash::FxHashMap<usize, Node>, id: usize| {
        nodes
            .get(&id)
            .map(|node| crate::test::helper::path_for_node(*node))
    };
    let mut entries: Vec<HumanMappingEntry> = diff
        .mapping
        .iter()
        .map(|(&(before_id, after_id), mapping)| HumanMappingEntry {
            operation: match mapping.operation {
                ASTMappingOperation::Identical => HumanOperation::Identical,
                ASTMappingOperation::Update => HumanOperation::Update,
                ASTMappingOperation::Delete => HumanOperation::Delete,
                ASTMappingOperation::DeleteWithChildren => HumanOperation::DeleteWithChildren,
                ASTMappingOperation::Insert => HumanOperation::Insert,
                ASTMappingOperation::InsertWithChildren => HumanOperation::InsertWithChildren,
                _ => HumanOperation::MatchButNotIdentical,
            },
            before_path: path(&cache.before, before_id),
            after_path: path(&cache.after, after_id),
        })
        .collect();
    entries.sort_by(|a, b| (&a.before_path, &a.after_path).cmp(&(&b.before_path, &b.after_path)));
    entries
}

/// The fixture's own host mapping plus a TSX layer copied from omnidiff's embedded diff, and
/// omnidiff's diff.
fn shadcn_with_a_layer() -> Result<(Code, Code, HumanMapping, ASTDiff)> {
    let (before, after) = code_pair(SHADCN)?;
    let ast = crate::diff::diff_code(&before, &after)
        .ast
        .context("no AST diff")?;
    let found = ast.embedded.first().context("no embedded layer")?;
    let host_path: Vec<String> = HOST_PATH.iter().map(|step| step.to_string()).collect();
    let mut mapping = load_with(SHADCN, &before, &after)?;
    mapping.embedded = vec![HumanEmbeddedLayer {
        before_path: host_path.clone(),
        after_path: host_path,
        language: crate::code::language::stable_name(found.language).to_string(),
        mapping: HumanMapping {
            entries: entries_of(&found.diff, &found.before, &found.after),
            ..Default::default()
        },
    }];
    Ok((before, after, mapping, ast))
}

#[test]
fn an_embedded_layer_round_trips_through_the_file() -> Result<()> {
    let (before, after, mapping, _) = shadcn_with_a_layer()?;
    let stored = StoredMapping::collapse(&mapping, Some((&before, &after)));
    let layer = stored.embedded.first().context("stored layer")?;
    assert_eq!(layer.language, "TSX");
    assert!(
        layer.mapping.has_subtrees(),
        "the TSX layer's unchanged subtrees collapse like any file's"
    );
    let json = serde_json::to_string(&stored)?;
    let read: StoredMapping = serde_json::from_str(&json)?;
    let expanded = read.expand(Some((&before, &after)))?;
    assert_eq!(compact(&expanded), compact(&mapping));
    Ok(())
}

#[test]
fn an_embedded_layer_is_graded_against_omnidiffs_layer() -> Result<()> {
    let (before, after, mut mapping, mut ast) = shadcn_with_a_layer()?;
    assert!(mapping_mismatches(&mapping, &before, &after, &ast)?.is_empty());

    // One inner decision the other way round: a single mismatch, pointed at the host string.
    let layer = &mut mapping.embedded[0].mapping;
    let unchanged = layer
        .entries
        .iter_mut()
        .find(|entry| entry.operation == HumanOperation::Identical)
        .context("an unchanged node")?;
    unchanged.operation = HumanOperation::MatchButNotIdentical;
    let mismatches = mapping_mismatches(&mapping, &before, &after, &ast)?;
    assert!(!mismatches.is_empty());
    assert!(
        mismatches
            .iter()
            .all(|mismatch| mismatch.message.starts_with("[embedded TSX at "))
    );

    // A layer omnidiff did not find is one mismatch, not one per inner entry.
    ast.embedded.clear();
    let mismatches = mapping_mismatches(&mapping, &before, &after, &ast)?;
    assert_eq!(mismatches.len(), 1, "{:?}", mismatches[0].message);
    Ok(())
}

#[test]
fn a_human_layer_paints_the_host_file_like_omnidiffs() -> Result<()> {
    let (before, after, mapping, _) = shadcn_with_a_layer()?;
    let human = as_ast_diff_for_mapping(&mapping, &before, &after)?;
    assert_eq!(human.embedded.len(), 1);
    let cache = NodeCache::build(&before, &after);
    let inserted = TextDiff::from(&before, &after, &human, &cache)
        .all(1)
        .into_iter()
        .filter(|range| range.operation == TextOperation::Insert)
        .count();
    assert_eq!(inserted, 3);
    Ok(())
}

/// The three inserted `phosphor="..."` attributes, painted in the decoded TSX as a reader would
/// paint the file: each attribute alone, not its indentation or its line break.
fn shadcn_layer_painting(decoded_after: &str) -> NamedTextMapping {
    let entries = decoded_after
        .lines()
        .enumerate()
        .filter_map(|(row, line)| {
            let column = line.find("phosphor=")?;
            Some(HumanTextEntry {
                operation: HumanTextOperation::Insert,
                before: Vec::new(),
                after: vec![HumanTextSpan {
                    start_row: row,
                    start_column: column,
                    end_row: row,
                    end_column: line.len(),
                }],
            })
        })
        .collect();
    NamedTextMapping {
        name: "Minimal".to_string(),
        mapping: HumanTextMapping { entries },
    }
}

#[test]
fn a_painted_layer_is_graded_in_the_host_files() -> Result<()> {
    use crate::diff::text::RenderOptions;

    let (before, after, mut mapping, ast) = shadcn_with_a_layer()?;
    // The host painting says nothing changed: every change is inside the string.
    mapping.text_mappings = vec![NamedTextMapping {
        name: "Minimal".to_string(),
        mapping: HumanTextMapping::default(),
    }];
    let diff = PaintingDiff::Ast {
        ast: Box::new(ast),
        node_cache: NodeCache::build(&before, &after),
    };
    let unpainted_layer =
        compare_painting_for_mapping(&mapping, RenderOptions::MINIMAL, &before, &after, &diff)?;
    assert!(
        unpainted_layer.mismatched_bytes > 0,
        "an unpainted layer leaves omnidiff's three insertions unexplained"
    );

    let decoded = mapping.embedded[0].resolve(&before, &after)?;
    mapping.embedded[0].mapping.text_mappings =
        vec![shadcn_layer_painting(&decoded.after.contents)];
    let painted_layer =
        compare_painting_for_mapping(&mapping, RenderOptions::MINIMAL, &before, &after, &diff)?;
    assert_eq!(painted_layer.mismatched_bytes, 0);
    Ok(())
}

#[test]
fn a_painted_layer_keeps_the_hosts_rules_off_its_bytes() -> Result<()> {
    use crate::test::helper::human_mapping::invariants::ground_truth_invariant_violations_for;

    let (before, after, mut mapping, _) = shadcn_with_a_layer()?;
    mapping.text_mappings = vec![NamedTextMapping {
        name: "Minimal".to_string(),
        mapping: HumanTextMapping::default(),
    }];
    // The host mapping inserts the string's leaves, which an empty host painting leaves unpainted.
    let unpainted = ground_truth_invariant_violations_for(&mapping, &before, &after)?;
    assert!(!unpainted.is_empty());

    let decoded = mapping.embedded[0].resolve(&before, &after)?;
    mapping.embedded[0].mapping.text_mappings =
        vec![shadcn_layer_painting(&decoded.after.contents)];
    let painted = ground_truth_invariant_violations_for(&mapping, &before, &after)?;
    assert!(
        painted.is_empty(),
        "{:#?}",
        painted.iter().map(ToString::to_string).collect::<Vec<_>>()
    );
    Ok(())
}
