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
use super::*;
use crate::diff::text::{TextDiff, TextOperation};
use crate::test::helper::find_first_of_kind;
use anyhow::{Context, Result};

/// The first node of `kind` in `code`, and `code`'s text.
fn first_of_kind<'c>(code: &'c Code, kind: &str) -> Node<'c> {
    find_first_of_kind(code.ast.as_ref().unwrap().root_node(), kind).unwrap()
}

/// Every decoded byte range `a..b` must map to host bytes that spell exactly that text.
fn assert_offsets_spell(decoded: &Decoded, source: &str, spelled: &[(&str, &str)]) {
    assert_eq!(decoded.offsets.len(), decoded.text.len() + 1);
    for (meant, host) in spelled {
        let at = decoded.text.find(meant).unwrap();
        let from = decoded.offsets[at];
        let to = decoded.offsets[at + meant.len()];
        assert_eq!(&source[from..to], *host, "decoded {meant:?}");
    }
}

#[test]
fn json_escapes_decode_and_map_back_to_their_spelling() {
    let source = r#"{"content": "a \"b\"\n\tcé 😀 d"}"#;
    let code = Code::from_string(source, &Language::JSON);
    let pair = first_of_kind(&code, "pair");
    let string = pair.child_by_field_name("value").unwrap();
    let decoded = decode(Language::JSON, string, source).unwrap();
    assert_eq!(decoded.text, "a \"b\"\n\tcé 😀 d");
    assert_offsets_spell(
        &decoded,
        source,
        &[
            ("\"b\"", r#"\"b\""#),
            ("\n\t", r"\n\t"),
            ("é", r"é"),
            ("😀", r"😀"),
            (" d", " d"),
        ],
    );
}

#[test]
fn yaml_block_scalars_lose_their_indentation_and_github_expressions() {
    let source = "steps:\n  - run: |\n      make ${{ matrix.target }}\n        echo done\n";
    let code = Code::from_string(source, &Language::YAML);
    let block = first_of_kind(&code, "block_scalar");
    let decoded = decode(Language::YAML, block, source).unwrap();
    assert_eq!(
        decoded.text,
        format!(
            "make {}\n  echo done",
            "_".repeat("${{ matrix.target }}".len())
        )
    );
    assert_offsets_spell(
        &decoded,
        source,
        &[("make", "make"), ("  echo done", "  echo done")],
    );
    assert_eq!(
        detect(Language::YAML, block, source),
        Some(Language::ShellScript)
    );
}

#[test]
fn template_substitutions_become_filler_of_the_same_length() {
    let source = "const A = styled.div`\n  color: ${(p) => p.c};\n  margin: 0;\n`;\n";
    let code = Code::from_string(source, &Language::JavaScript);
    let template = first_of_kind(&code, "template_string");
    let decoded = decode(Language::JavaScript, template, source).unwrap();
    assert_eq!(
        decoded.text,
        format!(
            "\n  color: {};\n  margin: 0;\n",
            "_".repeat("${(p) => p.c}".len())
        )
    );
    assert_eq!(
        detect(Language::JavaScript, template, source),
        Some(Language::CSS)
    );
}

#[test]
fn a_string_without_context_is_not_embedded_code() {
    let source = r#"{"description": "import * as React from \"react\"\n"}"#;
    let code = Code::from_string(source, &Language::JSON);
    let pair = first_of_kind(&code, "pair");
    let string = pair.child_by_field_name("value").unwrap();
    assert_eq!(detect(Language::JSON, string, source), None);
}

#[test]
fn a_shell_raw_string_of_preprocessor_lines_is_c() {
    let source = "includes='\n#include <sys/types.h>\n#define X 1\n'\nscript='echo #include'\n";
    let code = Code::from_string(source, &Language::ShellScript);
    let root = code.ast.as_ref().unwrap().root_node();
    let mut strings = Vec::new();
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if node.kind() == "raw_string" {
            strings.push(node);
        }
        let mut cursor = node.walk();
        stack.extend(node.children(&mut cursor));
    }
    strings.sort_by_key(|node| node.start_byte());
    let [includes, script] = strings[..] else {
        panic!("two raw strings, got {strings:?}");
    };
    let decoded = decode(Language::ShellScript, includes, source).unwrap();
    assert_eq!(decoded.text, "\n#include <sys/types.h>\n#define X 1\n");
    assert_offsets_spell(&decoded, source, &[("#define X 1", "#define X 1")]);
    assert_eq!(
        detect(Language::ShellScript, includes, source),
        Some(Language::C)
    );
    // A `#` further in is not a directive.
    assert_eq!(detect(Language::ShellScript, script, source), None);
}

/// Every node of `kind` in `code`, in source order.
fn all_of_kind<'c>(code: &'c Code, kind: &str) -> Vec<Node<'c>> {
    let mut found = Vec::new();
    let mut stack = vec![code.ast.as_ref().unwrap().root_node()];
    while let Some(node) = stack.pop() {
        if node.kind() == kind {
            found.push(node);
        }
        let mut cursor = node.walk();
        stack.extend(node.children(&mut cursor));
    }
    found.sort_by_key(|node| node.start_byte());
    found
}

#[test]
fn a_csharp_raw_string_loses_its_closing_indentation() {
    let indented =
        "class A { string q = \"\"\"\n        SELECT a\n\n          FROM b\n        \"\"\"; }\n";
    let flush = "class A { string q = \"\"\"\nSELECT a\n\n  FROM b\n\"\"\"; }\n";
    let text = |source: &str| {
        let code = Code::from_string(source, &Language::CSharp);
        let string = all_of_kind(&code, "raw_string_literal")[0];
        let decoded = decode(Language::CSharp, string, source).unwrap();
        assert_offsets_spell(&decoded, source, &[("FROM b", "FROM b")]);
        decoded.text
    };
    assert_eq!(text(indented), "SELECT a\n\n  FROM b");
    assert_eq!(
        text(indented),
        text(flush),
        "re-indenting the literal changes nothing in it"
    );
}

#[test]
fn python_strings_resolve_escapes_unless_raw_and_ansible_docs_are_yaml() {
    let source = "DOCUMENTATION = '''\nmodule: x\\tab \\x41\n'''\nRAW = r'a\\n'\nF = f'{x}!'\n";
    let code = Code::from_string(source, &Language::Python);
    let strings = all_of_kind(&code, "string");
    let text = |index: usize| decode(Language::Python, strings[index], source).unwrap();
    let documentation = text(0);
    assert_eq!(documentation.text, "\nmodule: x\tab A\n");
    assert_offsets_spell(&documentation, source, &[("\tab", "\\tab"), ("A", "\\x41")]);
    assert_eq!(text(1).text, "a\\n");
    assert_eq!(text(2).text, "___!");
    assert_eq!(
        detect(Language::Python, strings[0], source),
        Some(Language::YAML)
    );
    assert_eq!(detect(Language::Python, strings[1], source), None);
}

/// nomad's `mkerrors.sh` lists each platform's C headers in a shell `'...'` string and filters
/// constants with an awk program in another; the commit deletes a few headers and a few awk
/// lines. Each changed header list is diffed as C, and the awk program, with no grammar, as text:
/// either way a deleted line is painted alone.
#[test]
fn shell_strings_of_c_headers_and_awk_are_diffed_as_c_and_text() -> Result<()> {
    let pair = crate::test::helper::handmade_test_code_pair(
        "shellscript-nomad-software-vend-huge-multi-line-strings-updated",
    )?;
    let (before, after) = (&pair.0, &pair.1);
    let ast = crate::diff::diff_code(before, after)
        .ast
        .context("no AST diff")?;
    let languages: Vec<Language> = ast.embedded.iter().map(|layer| layer.language).collect();
    assert_eq!(
        languages,
        [
            Language::C,
            Language::C,
            Language::C,
            Language::C,
            Language::Unknown
        ]
    );

    let node_cache = crate::diff::NodeCache::build(before, after);
    let deleted: Vec<&str> = TextDiff::from(before, after, &ast, &node_cache)
        .all(0)
        .iter()
        .filter(|range| range.operation == TextOperation::Delete)
        .filter_map(|range| {
            let row = before.contents.lines().nth(range.source.start_row)?;
            (range.source.start_row == range.source.end_row)
                .then(|| &row[range.source.start_column..range.source.end_column])
        })
        .collect();
    // A text layer's line keeps its indentation, as a file without a grammar paints it.
    for line in ["#include <sys/disk.h>", "\t\t$2 ~ /^DIOC/ ||"] {
        assert!(deleted.contains(&line), "{line:?} not in {deleted:#?}");
    }
    Ok(())
}

/// shadcn's registry stores each component as a JSON string; the commit adds one `phosphor="..."`
/// attribute in three places. The TSX layer must find exactly those three insertions, and the JSON
/// file must be painted at each one alone, not from the first change to the last.
#[test]
fn shadcn_tsx_in_a_json_string_is_diffed_as_tsx() -> Result<()> {
    let pair = crate::test::helper::handmade_test_code_pair(
        "json-shadcn-ui-ui-react-code-in-string-constant",
    )?;
    let (before, after) = (&pair.0, &pair.1);
    let diff = crate::diff::diff_code(before, after);
    let ast = diff.ast.context("no AST diff")?;
    assert_eq!(ast.embedded.len(), 1);
    let layer = &ast.embedded[0];
    assert_eq!(layer.language, Language::TSX);
    assert!(
        layer
            .before
            .contents
            .starts_with("import * as React from \"react\"\n\n")
    );

    let node_cache = crate::diff::NodeCache::build(before, after);
    let text = TextDiff::from(before, after, &ast, &node_cache);
    let painted: Vec<&str> = text
        .all(1)
        .iter()
        .filter(|range| {
            !matches!(
                range.operation,
                TextOperation::Identical | TextOperation::NotYetSet
            ) && !range.source.is_empty()
        })
        .map(|range| {
            let row = after.contents.lines().nth(range.source.start_row).unwrap();
            assert_eq!(range.source.start_row, range.source.end_row);
            &row[range.source.start_column..range.source.end_column]
        })
        .collect();
    assert_eq!(
        painted,
        [
            r#"phosphor=\"CaretLeftIcon\""#,
            r#"phosphor=\"CaretRightIcon\""#,
            r#"phosphor=\"DotsThreeIcon\""#,
        ]
    );
    Ok(())
}
