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
use anyhow::{Context, Result};
use metrohash::MetroHash64;

use crate::code::similarity::SimilaritySketch;
use std::hash::Hasher;

use crate::code::metadata::NodeRecord;
use crate::code::{ASTMetadata, Code, Language};
use crate::diff::nodes::is_commutative_container;

/// Inserts `node_id -> hash` into `forward` and appends `node_id` to `reverse`'s bucket.
fn record_hash(
    forward: &mut rustc_hash::FxHashMap<usize, u64>,
    reverse: &mut rustc_hash::FxHashMap<u64, Vec<usize>>,
    node_id: usize,
    hash: u64,
) {
    forward.insert(node_id, hash);
    reverse.entry(hash).or_default().push(node_id);
}

/// Fills `metadata`'s four hash maps (full, structural, kind-and-value, kind-only; see
/// [`ASTMetadata`]), their reverse maps, and the similarity sketches. Errors if `code` is unparsed.
///
/// Speed matters (every file is hashed) and security does not, hence MetroHash. Node ids are only
/// stable within one parse, which is all the maps need.
pub fn hash_code(code: &Code, metadata: &mut ASTMetadata) -> Result<()> {
    let ast = code
        .ast
        .as_ref()
        .context("AST must be parsed before hashing")?;
    let nodes = crate::code::metadata::collect_nodes(ast.root_node());
    hash_nodes(
        &nodes,
        code.contents.as_bytes(),
        metadata.language,
        metadata,
    );
    Ok(())
}

/// [`hash_code`] over an already collected node table (see `metadata::collect_nodes`). Returns
/// each node's `ASTNodeMetadata::owned_text_hash`, by table index: it reads the same gaps.
///
/// Visits the table in reverse preorder (post-order, children right to left), so each child's
/// hash is computed once and read by index: linear, not quadratic, on deep trees. That order is
/// also the order of the `*_hash_to_node` buckets, the last-resort tie-break of
/// `solve_moved_subtrees` and `hash_tree_matching`, so changing it changes diffs.
///
/// Each node's gaps are classified once ([`classify_gaps`]) and every hash reads that, and the
/// per-node buffers (sorted child hashes, the sketch pool) are reused scratch: this runs on every
/// node of every file, and per-node allocations and repeated gap scans dominated it.
pub(crate) fn hash_nodes(
    nodes: &[NodeRecord],
    source: &[u8],
    language: Language,
    metadata: &mut ASTMetadata,
) -> Vec<u64> {
    metadata.node_to_full_hash.clear();
    metadata.full_hash_to_node.clear();
    metadata.node_to_structural_hash.clear();
    metadata.structural_hash_to_node.clear();
    metadata.node_to_kind_and_value_hash.clear();
    metadata.kind_and_value_hash_to_node.clear();
    metadata.node_to_kind_only_hash.clear();
    metadata.kind_only_hash_to_node.clear();
    metadata.node_to_similarity_sketch.clear();

    let n = nodes.len();
    metadata.node_to_full_hash.reserve(n);
    metadata.node_to_structural_hash.reserve(n);
    metadata.node_to_kind_and_value_hash.reserve(n);
    metadata.node_to_kind_only_hash.reserve(n);
    metadata.node_to_similarity_sketch.reserve(n);

    let mut full = vec![0u64; n];
    let mut structural = vec![0u64; n];
    let mut kind_and_value = vec![0u64; n];
    let mut kind_only = vec![0u64; n];
    let mut sketches = vec![SimilaritySketch::default(); n];
    let mut owned_text_hashes = vec![0u64; n];

    let mut gaps = Vec::new();
    let mut sorted_hashes = Vec::new();
    let mut pool = Vec::new();

    for (index, record) in nodes.iter().enumerate().rev() {
        let any_content = classify_gaps(record, source, nodes, &mut gaps);
        let commutative = is_commutative_container(record.kind, &language);

        let full_hash = compute_full_hash(record, source, nodes, &full, &gaps);
        let structural_hash = compute_structural_hash(record, &structural);
        let kind_and_value_hash = compute_kind_and_value_hash(
            record,
            source,
            nodes,
            &kind_and_value,
            &gaps,
            commutative,
            &mut sorted_hashes,
        );
        let kind_only_hash =
            compute_kind_only_hash(record, &kind_only, commutative, &mut sorted_hashes);
        full[index] = full_hash;
        structural[index] = structural_hash;
        kind_and_value[index] = kind_and_value_hash;
        kind_only[index] = kind_only_hash;

        record_hash(
            &mut metadata.node_to_full_hash,
            &mut metadata.full_hash_to_node,
            record.id,
            full_hash,
        );
        record_hash(
            &mut metadata.node_to_structural_hash,
            &mut metadata.structural_hash_to_node,
            record.id,
            structural_hash,
        );
        record_hash(
            &mut metadata.node_to_kind_and_value_hash,
            &mut metadata.kind_and_value_hash_to_node,
            record.id,
            kind_and_value_hash,
        );
        record_hash(
            &mut metadata.node_to_kind_only_hash,
            &mut metadata.kind_only_hash_to_node,
            record.id,
            kind_only_hash,
        );

        // Owned gap text counts as a leaf: in tree-sitter-yaml a quoted scalar's leaves are just
        // its quotes, so without it every such string sketches identically.
        let sketch = if record.children.is_empty() {
            SimilaritySketch::leaf(full_hash)
        } else {
            pool.clear();
            for &child in &record.children {
                pool.extend_from_slice(sketches[child].as_slice());
            }
            if any_content {
                let own_text_hash = compute_owned_text_hash(record, source, nodes, &gaps);
                pool.extend_from_slice(SimilaritySketch::leaf(own_text_hash).as_slice());
                owned_text_hashes[index] = owned_text_hash(record, source, nodes, &gaps);
            }
            SimilaritySketch::from_pool(&mut pool)
        };
        sketches[index] = sketch.clone();
        metadata.node_to_similarity_sketch.insert(record.id, sketch);
    }

    owned_text_hashes
}

/// The source range of each gap around `record`'s children, in order: before the first child,
/// between each pair, after the last. A leaf has one, its whole span.
fn gap_ranges<'a>(
    record: &'a NodeRecord,
    nodes: &'a [NodeRecord],
) -> impl Iterator<Item = (usize, usize)> + 'a {
    let starts = std::iter::once(record.start_byte)
        .chain(record.children.iter().map(|&child| nodes[child].end_byte));
    let ends = record
        .children
        .iter()
        .map(|&child| nodes[child].start_byte)
        .chain(std::iter::once(record.end_byte));
    starts.zip(ends)
}

/// Fills `gaps` with whether each of `record`'s gaps ([`gap_ranges`]) holds content - non-empty,
/// valid UTF-8 and not all whitespace - and returns whether any does. Only content gaps are
/// hashed: reformatting keeps every hash.
fn classify_gaps(
    record: &NodeRecord,
    source: &[u8],
    nodes: &[NodeRecord],
    gaps: &mut Vec<bool>,
) -> bool {
    gaps.clear();
    let mut any_content = false;
    for (start, end) in gap_ranges(record, nodes) {
        let content = start < end
            && std::str::from_utf8(&source[start..end]).is_ok_and(|text| !text.trim().is_empty());
        any_content |= content;
        gaps.push(content);
    }
    any_content
}

/// Hashes into `hasher` each content gap of `record` (see [`classify_gaps`]), untrimmed so
/// embedded whitespace still counts, with `between(hasher, i)` called after gap `i` for every
/// child `i`.
fn hash_gaps_around_children(
    hasher: &mut MetroHash64,
    record: &NodeRecord,
    source: &[u8],
    nodes: &[NodeRecord],
    gaps: &[bool],
    mut between: impl FnMut(&mut MetroHash64, usize),
) {
    for (i, ((start, end), &content)) in gap_ranges(record, nodes).zip(gaps).enumerate() {
        if content {
            hasher.write(&source[start..end]);
        }
        if i < record.children.len() {
            between(hasher, i);
        }
    }
}

/// The full hash: a Merkle hash of kind, child count, each child's hash, and the gap text this node
/// owns around its children (a leaf's whole span). `child_hashes` is indexed by table index.
///
/// Gaps rather than the whole span, so reformatting keeps the hash; but a gap can be content
/// (tree-sitter-r leaves a string's body outside its only child), so only an all-whitespace gap is
/// skipped, and a kept gap is hashed untrimmed so embedded whitespace still counts.
fn compute_full_hash(
    record: &NodeRecord,
    source_code: &[u8],
    nodes: &[NodeRecord],
    child_hashes: &[u64],
    gaps: &[bool],
) -> u64 {
    let mut hasher = MetroHash64::new();

    hasher.write(record.kind_id.to_le_bytes().as_slice());
    hasher.write(record.children.len().to_le_bytes().as_slice());

    hash_gaps_around_children(
        &mut hasher,
        record,
        source_code,
        nodes,
        gaps,
        |hasher, i| {
            hasher.write(child_hashes[record.children[i]].to_le_bytes().as_slice());
        },
    );

    hasher.finish()
}

/// A hash of the gap text an internal node owns around its children, for its similarity sketch:
/// the gaps alone, so the sketch does not depend on whether a grammar made a scalar's body a child
/// or gap text. Only meaningful when some gap holds content.
fn compute_owned_text_hash(
    record: &NodeRecord,
    source_code: &[u8],
    nodes: &[NodeRecord],
    gaps: &[bool],
) -> u64 {
    let mut hasher = MetroHash64::new();
    hasher.write(record.kind_id.to_le_bytes().as_slice());
    hash_gaps_around_children(&mut hasher, record, source_code, nodes, gaps, |_, _| {});
    hasher.finish()
}

/// `ASTNodeMetadata::owned_text_hash` of an internal node with a content gap: like
/// [`compute_owned_text_hash`] without the kind, and never 0, which is reserved for "owns no
/// text".
fn owned_text_hash(
    record: &NodeRecord,
    source_code: &[u8],
    nodes: &[NodeRecord],
    gaps: &[bool],
) -> u64 {
    let mut hasher = MetroHash64::new();
    hash_gaps_around_children(&mut hasher, record, source_code, nodes, gaps, |_, _| {});
    match hasher.finish() {
        0 => 1,
        hash => hash,
    }
}

/// The structural hash: kinds and child counts only, in order. `child_hashes` is indexed by table
/// index.
fn compute_structural_hash(record: &NodeRecord, child_hashes: &[u64]) -> u64 {
    let mut hasher = MetroHash64::new();

    hasher.write(record.kind_id.to_le_bytes().as_slice());
    hasher.write(record.children.len().to_le_bytes().as_slice());

    for &child in &record.children {
        hasher.write(child_hashes[child].to_le_bytes().as_slice());
    }

    hasher.finish()
}

/// Like `compute_full_hash`, but a commutative container's children are hashed in sorted order.
/// Children's hashes come from this same function, so the order-independence reaches every
/// ancestor: the `enum_item` around a reordered `enum_variant_list` keeps its hash. That requires
/// `hash_tree_matching::pair_children_for_descent` to pair such children by hash, not position.
/// `sorted` is scratch.
fn compute_kind_and_value_hash(
    record: &NodeRecord,
    source_code: &[u8],
    nodes: &[NodeRecord],
    child_hashes: &[u64],
    gaps: &[bool],
    commutative: bool,
    sorted: &mut Vec<u64>,
) -> u64 {
    let mut hasher = MetroHash64::new();
    hasher.write(record.kind_id.to_le_bytes().as_slice());
    hasher.write(record.children.len().to_le_bytes().as_slice());

    if commutative {
        // Gap text is a document-order artifact, so it is dropped here.
        write_sorted_child_hashes(&mut hasher, record, child_hashes, sorted);
    } else {
        hash_gaps_around_children(
            &mut hasher,
            record,
            source_code,
            nodes,
            gaps,
            |hasher, i| {
                hasher.write(child_hashes[record.children[i]].to_le_bytes().as_slice());
            },
        );
    }

    hasher.finish()
}

/// Writes `record`'s children's hashes into `hasher` in ascending order, through scratch `sorted`.
fn write_sorted_child_hashes(
    hasher: &mut MetroHash64,
    record: &NodeRecord,
    child_hashes: &[u64],
    sorted: &mut Vec<u64>,
) {
    sorted.clear();
    sorted.extend(record.children.iter().map(|&child| child_hashes[child]));
    sorted.sort_unstable();
    for hash in sorted.iter() {
        hasher.write(hash.to_le_bytes().as_slice());
    }
}

/// Like `compute_structural_hash`, with `compute_kind_and_value_hash`'s order-independence for
/// commutative containers. The single "same shape, any leaf values" tier: coarser than separate
/// ignore-identifiers / ignore-literals tiers, a deliberate precision trade.
fn compute_kind_only_hash(
    record: &NodeRecord,
    child_hashes: &[u64],
    commutative: bool,
    sorted: &mut Vec<u64>,
) -> u64 {
    let mut hasher = MetroHash64::new();
    hasher.write(record.kind_id.to_le_bytes().as_slice());
    hasher.write(record.children.len().to_le_bytes().as_slice());

    if commutative {
        write_sorted_child_hashes(&mut hasher, record, child_hashes, sorted);
    } else {
        for &child in &record.children {
            hasher.write(child_hashes[child].to_le_bytes().as_slice());
        }
    }

    hasher.finish()
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use std::collections::HashSet;

    use crate::test::helper;

    use super::*;

    #[test]
    fn hash_all_handmade_codes() -> Result<()> {
        let codes = helper::handmade_test_code()?;

        for (_, code) in codes {
            let mut metadata = ASTMetadata::default();
            hash_code(&code, &mut metadata)?;

            assert!(!metadata.node_to_full_hash.is_empty());
            assert!(!metadata.full_hash_to_node.is_empty());

            assert_eq!(
                metadata.node_to_full_hash.len(),
                metadata
                    .full_hash_to_node
                    .values()
                    .map(|set| set.len())
                    .sum::<usize>()
            );

            for (node_id, hash) in &metadata.node_to_full_hash {
                if let Some(node_set) = metadata.full_hash_to_node.get(hash) {
                    assert!(
                        node_set.contains(node_id),
                        "Node {} with hash {} not found in reverse map",
                        node_id,
                        hash
                    );
                } else {
                    panic!(
                        "Hash {} from node {} not found in reverse map",
                        hash, node_id
                    );
                }
            }

            assert!(!metadata.node_to_structural_hash.is_empty());
            assert!(!metadata.structural_hash_to_node.is_empty());

            assert_eq!(
                metadata.node_to_structural_hash.len(),
                metadata
                    .structural_hash_to_node
                    .values()
                    .map(|set| set.len())
                    .sum::<usize>()
            );

            for (node_id, hash) in &metadata.node_to_structural_hash {
                if let Some(node_set) = metadata.structural_hash_to_node.get(hash) {
                    assert!(
                        node_set.contains(node_id),
                        "Node {} with structural hash {} not found in reverse map",
                        node_id,
                        hash
                    );
                } else {
                    panic!(
                        "Structural hash {} from node {} not found in reverse map",
                        hash, node_id
                    );
                }
            }
        }

        Ok(())
    }

    #[test]
    fn test_full_vs_structural_hashing() -> Result<()> {
        let codes = helper::handmade_test_code()?;

        for (filename, code) in &codes {
            let mut metadata = ASTMetadata::default();
            hash_code(code, &mut metadata)?;

            let full_hash_count = metadata.full_hash_to_node.len();
            let structural_hash_count = metadata.structural_hash_to_node.len();

            assert!(
                structural_hash_count <= full_hash_count,
                "For file {}: Structural hashes ({}) should be <= full hashes ({})",
                filename,
                structural_hash_count,
                full_hash_count
            );

            let mut found_different_content_same_structure = false;

            for node_set in metadata.structural_hash_to_node.values() {
                if node_set.len() > 1 {
                    let mut full_hashes = HashSet::new();
                    for node_id in node_set {
                        if let Some(full_hash) = metadata.node_to_full_hash.get(node_id) {
                            full_hashes.insert(full_hash);
                        }
                    }

                    if full_hashes.len() > 1 {
                        found_different_content_same_structure = true;
                        break;
                    }
                }
            }

            // Above 20 nodes; the JavaScript and TypeScript hello worlds are too trivial.
            if metadata.node_to_full_hash.len() > 20 {
                assert!(
                    found_different_content_same_structure,
                    "For file {}: Expected to find nodes with same structure but different content in non-trivial code",
                    filename
                );
            }
        }

        Ok(())
    }

    fn root_hashes(source: &str, language: Language) -> (u64, u64) {
        let code = Code::from_string(source, &language);
        let root = code.ast.as_ref().unwrap().root_node().id();
        let mut metadata = ASTMetadata {
            language,
            ..Default::default()
        };
        hash_code(&code, &mut metadata).unwrap();
        (
            metadata.node_to_full_hash[&root],
            metadata.node_to_kind_and_value_hash[&root],
        )
    }

    #[test]
    fn full_hash_ignores_reindentation() {
        let (flat, _) = root_hashes("fn f() {\n    x();\n}\n", Language::Rust);
        let (deeper, _) = root_hashes("fn f() {\n        x();\n}\n", Language::Rust);
        assert_eq!(flat, deeper);
    }

    #[test]
    fn full_hash_counts_string_content_a_grammar_leaves_in_a_gap() {
        // tree-sitter-r's `string_content` has the `\n` escape as its only child.
        let (hello, _) = root_hashes("x <- \"Hello, World!\\n\"\n", Language::R);
        let (other, _) = root_hashes("x <- \"Goodbye, World!\\n\"\n", Language::R);
        assert_ne!(hello, other);
    }

    #[test]
    fn kind_and_value_hash_of_an_ancestor_survives_reordering_a_commutative_container() {
        let (full_ab, kv_ab) = root_hashes("enum E { A, B }\n", Language::Rust);
        let (full_ba, kv_ba) = root_hashes("enum E { B, A }\n", Language::Rust);
        assert_ne!(full_ab, full_ba);
        assert_eq!(kv_ab, kv_ba);
    }

    #[test]
    fn test_identical_code_produces_same_hashes() -> Result<()> {
        let codes = helper::handmade_test_code()?;

        let code = codes
            .get("hello-world.rs")
            .ok_or_else(|| anyhow::anyhow!("Test file 'hello-world.rs' not found"))?;
        let mut metadata1 = ASTMetadata::default();
        let mut metadata2 = ASTMetadata::default();
        hash_code(code, &mut metadata1)?;
        hash_code(code, &mut metadata2)?;

        assert_eq!(metadata1.node_to_full_hash, metadata2.node_to_full_hash);
        assert_eq!(metadata1.full_hash_to_node, metadata2.full_hash_to_node);
        assert_eq!(
            metadata1.node_to_structural_hash,
            metadata2.node_to_structural_hash
        );
        assert_eq!(
            metadata1.structural_hash_to_node,
            metadata2.structural_hash_to_node
        );

        Ok(())
    }

    #[test]
    fn test_different_code_structural_similarity() -> Result<()> {
        let codes = helper::handmade_test_code()?;

        // Same structure, different string content.
        let code1 = codes
            .get("hello-world.rs")
            .ok_or_else(|| anyhow::anyhow!("Test file 'hello-world.rs' not found"))?;
        let code2 = codes
            .get("zdravo-svijete.rs")
            .ok_or_else(|| anyhow::anyhow!("Test file 'zdravo-svijete.rs' not found"))?;
        let mut metadata1 = ASTMetadata::default();
        let mut metadata2 = ASTMetadata::default();
        hash_code(code1, &mut metadata1)?;
        hash_code(code2, &mut metadata2)?;

        assert_ne!(metadata1.node_to_full_hash, metadata2.node_to_full_hash);

        // Node ids differ, so compare how many nodes share each structural hash.
        assert_eq!(
            metadata1.structural_hash_to_node.len(),
            metadata2.structural_hash_to_node.len(),
            "Different number of unique structural hashes"
        );

        for (hash1, nodes1) in &metadata1.structural_hash_to_node {
            if let Some(nodes2) = metadata2.structural_hash_to_node.get(hash1) {
                assert_eq!(
                    nodes1.len(),
                    nodes2.len(),
                    "Different number of nodes for structural hash {:?}",
                    hash1
                );
            } else {
                panic!(
                    "Structural hash {:?} found in first code but not in second",
                    hash1
                );
            }
        }

        Ok(())
    }

    pub fn benchmark_hash_code(code: &Code, iterations: usize) -> Result<std::time::Duration> {
        use std::time::Instant;

        let start = Instant::now();

        for _ in 0..iterations {
            let mut metadata = ASTMetadata::default();
            hash_code(code, &mut metadata)?;
        }

        let duration = start.elapsed();
        Ok(duration)
    }

    #[test]
    fn test_benchmark_function_works() -> Result<()> {
        let codes = helper::handmade_test_code()?;

        let code = codes
            .get("hello-world.rs")
            .ok_or_else(|| anyhow::anyhow!("Test file 'hello-world.rs' not found"))?;

        let duration = benchmark_hash_code(code, 1000)?;

        assert!(
            duration.as_millis() < 2000,
            "Benchmark took too long: {:?}",
            duration
        );

        assert!(
            duration.as_nanos() > 0,
            "Benchmark duration should be measurable"
        );

        Ok(())
    }
}
