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
use super::*;

/// `(kind, qualified name, old qualified name)` of every change, unchanged ones included.
fn changes(diff: &Diff) -> Vec<(ChangeKind, String, Option<String>)> {
    diff.all_definitions()
        .map(|c| {
            (
                c.kind(),
                c.qualified_name().to_string(),
                c.before().map(|d| d.qualified_name().to_string()),
            )
        })
        .collect()
}

const BEFORE: &str = r#"/// A parser.
pub struct Parser {
    pos: usize,
}

impl Parser {
    pub fn parse(&self) -> u8 {
        1
    }

    fn tokenize(&self) {
        let x = 1;
        let y = x + 2;
        println!("{y}");
    }

    /// Docs.
    fn documented(&self) {}

    fn dropped(&self) -> u32 {
        let unused = 41;
        unused + 1
    }
}

fn helper() {}

fn mover() {
    let a = 1;
    let b = a * 2;
}

mod inner {}
"#;

const AFTER: &str = r#"/// A parser.
#[derive(Debug)]
pub struct Parser {
    pos: usize,
}

impl Parser {
    pub fn parse(&self) -> u8 {
        2
    }

    fn lex(&self) {
        let x = 1;
        let y = x + 2;
        println!("{y}");
    }

    /// Better docs.
    fn documented(&self) {}

    fn peek(&self) -> Option<char> {
        None
    }
}

mod inner {
    fn mover() {
        let a = 1;
        let b = a * 2;
    }
}

fn helper() {}
"#;

/// A caller can diff on one thread and read on another.
#[test]
fn the_results_cross_threads() {
    fn shareable<T: Send + Sync>() {}
    shareable::<Diff>();
    shareable::<DefinitionChange>();
    shareable::<Error>();
}

#[test]
fn every_definition_is_told_what_happened_to_it() {
    let diff = Diff::new(BEFORE, AFTER, Language::Rust);
    let s = |x: &str| x.to_string();
    let some = |x: &str| Some(x.to_string());
    assert_eq!(
        changes(&diff),
        [
            // `#[derive(Debug)]` is part of the struct.
            (ChangeKind::Modified, s("Parser"), some("Parser")),
            (ChangeKind::Modified, s("impl Parser"), some("impl Parser")),
            (
                ChangeKind::Modified,
                s("Parser::parse"),
                some("Parser::parse")
            ),
            (
                ChangeKind::Renamed,
                s("Parser::lex"),
                some("Parser::tokenize")
            ),
            (
                ChangeKind::Modified,
                s("Parser::documented"),
                some("Parser::documented")
            ),
            (
                ChangeKind::Removed,
                s("Parser::dropped"),
                some("Parser::dropped")
            ),
            (ChangeKind::Added, s("Parser::peek"), None),
            (ChangeKind::Modified, s("inner"), some("inner")),
            (ChangeKind::Moved, s("inner::mover"), some("mover")),
            (ChangeKind::Unchanged, s("helper"), some("helper")),
        ]
    );
}

#[test]
fn the_flags_tell_a_rename_from_a_rename_with_edits_and_a_comment_from_code() {
    let diff = Diff::new(BEFORE, AFTER, Language::Rust);
    let find = |name: &str| {
        diff.all_definitions()
            .find(|c| c.qualified_name() == name)
            .unwrap_or_else(|| panic!("no {name}"))
    };
    let lex = find("Parser::lex");
    assert!(lex.is_renamed() && !lex.is_modified() && !lex.is_moved());
    let documented = find("Parser::documented");
    assert!(documented.is_modified() && documented.is_cosmetic_only());
    let parse = find("Parser::parse");
    assert!(parse.is_modified() && !parse.is_cosmetic_only());
    let mover = find("inner::mover");
    assert!(mover.is_moved() && !mover.is_modified());
}

#[test]
fn the_filters_agree_with_the_full_list() {
    let diff = Diff::new(BEFORE, AFTER, Language::Rust);
    let names = |it: &mut dyn Iterator<Item = &DefinitionChange>| -> Vec<String> {
        it.map(|c| c.qualified_name().to_string()).collect()
    };
    assert_eq!(
        names(&mut diff.changed_functions()),
        [
            "Parser::parse",
            "Parser::lex",
            "Parser::documented",
            "Parser::dropped",
            "Parser::peek",
            "inner::mover"
        ]
    );
    assert_eq!(names(&mut diff.changed_types()), ["Parser"]);
    assert_eq!(names(&mut diff.changed_modules()), ["inner"]);
    assert_eq!(
        names(&mut diff.changed_top_level_definitions()),
        ["Parser", "impl Parser", "inner"]
    );
    assert_eq!(
        diff.changed_definitions().count(),
        diff.all_definitions().count() - 1
    );
}

#[test]
fn a_file_diffed_against_itself_has_no_changes() {
    let diff = Diff::new(AFTER, AFTER, Language::Rust);
    assert!(!diff.has_changes());
    assert_eq!(diff.changed_definitions().count(), 0);
    assert!(diff.ranges(Side::Before, Preset::Minimal).is_empty());
    assert!(diff.ranges(Side::After, Preset::Extended).is_empty());
}

#[test]
fn ranges_are_bytes_and_positions_of_the_side_asked_for() {
    let before = "fn f() -> u8 {\n    1\n}\n";
    let after = "fn f() -> u8 {\n    2\n}\n";
    let diff = Diff::new(before, after, Language::Rust);
    let [change] = diff
        .ranges(Side::After, Preset::Minimal)
        .try_into()
        .expect("one change");
    assert_eq!(change.operation, Operation::Update);
    assert_eq!(&after[change.span.bytes.clone()], "2");
    assert_eq!(change.span.start, Position { row: 1, column: 4 });
    assert_eq!(change.other, None);
    let [change] = diff
        .ranges(Side::Before, Preset::Minimal)
        .try_into()
        .expect("one change");
    assert_eq!(&before[change.span.bytes.clone()], "1");
}

#[test]
fn a_move_says_where_the_text_is_in_the_other_side() {
    let body =
        "    let a = compute(1, 2, 3);\n    let b = a * a + 7;\n    println!(\"{a} {b}\");\n";
    let before = format!("fn first() {{\n{body}}}\n\nfn second() {{}}\n");
    let after = format!("fn second() {{}}\n\nfn first() {{\n{body}}}\n");
    let diff = Diff::new(&before, &after, Language::Rust);
    let moves: Vec<TextChange> = diff
        .ranges(Side::After, Preset::Minimal)
        .into_iter()
        .filter(|c| c.operation == Operation::Move)
        .collect();
    assert!(
        !moves.is_empty(),
        "{:?}",
        diff.ranges(Side::After, Preset::Minimal)
    );
    for m in &moves {
        let other = m.other.as_ref().expect("a move has its other side");
        assert_eq!(
            after[m.span.bytes.clone()].trim(),
            before[other.bytes.clone()].trim()
        );
    }
}

#[test]
fn an_insertion_is_only_in_the_after_side() {
    let before = "fn f() {}\n";
    let after = "fn f() {}\nfn g() {}\n";
    let diff = Diff::new(before, after, Language::Rust);
    let changes = diff.ranges(Side::After, Preset::Minimal);
    assert!(!changes.is_empty());
    assert!(
        changes
            .iter()
            .all(|c| c.operation == Operation::Insert && c.other.is_none())
    );
    assert!(diff.ranges(Side::Before, Preset::Minimal).is_empty());
}

#[test]
fn a_pair_without_a_grammar_is_diffed_line_by_line() {
    let diff = Diff::new("a\nb\nc\n", "a\nB\nc\n", Language::Unknown);
    assert!(!diff.is_structural());
    let ranges = diff.ranges(Side::After, Preset::Minimal);
    assert_eq!(ranges.len(), 1);
    assert_eq!(ranges[0].span.start.row, 1);
}

#[test]
fn the_enclosing_definition_is_the_innermost() {
    let diff = Diff::new(BEFORE, AFTER, Language::Rust);
    let offset = AFTER.find("None").expect("in peek");
    let d = diff
        .enclosing_definition(Side::After, offset)
        .expect("inside a method");
    assert_eq!(d.qualified_name(), "Parser::peek");
    assert_eq!(d.kind(), DefinitionKind::Method);
    assert!(
        diff.enclosing_definition(Side::After, AFTER.len())
            .is_none()
    );
}

#[test]
fn files_takes_the_language_from_the_name_and_an_empty_side_from_the_other() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let src = dir.path().join("src");
    std::fs::create_dir(&src)?;
    std::fs::write(dir.path().join("Cargo.toml"), "")?;
    let (empty, added) = (dir.path().join("empty"), src.join("parser.rs"));
    std::fs::write(&empty, "")?;
    std::fs::write(&added, "pub fn parse() {}\n")?;

    let diff = Diff::files(&empty, &added)?;
    assert_eq!(diff.language(), Language::Rust);
    assert!(diff.is_structural());
    let names: Vec<(ChangeKind, &str)> = diff
        .changed_definitions()
        .map(|c| (c.kind(), c.qualified_name()))
        .collect();
    assert_eq!(names, [(ChangeKind::Added, "crate::parser::parse")]);
    Ok(())
}

#[test]
fn files_reports_what_it_cannot_read() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let binary = dir.path().join("a.bin");
    std::fs::write(&binary, [0xff, 0xfe, 0x00])?;
    let missing = dir.path().join("missing.rs");
    assert!(matches!(
        Diff::files(&binary, &binary),
        Err(Error::NotText { .. })
    ));
    assert!(matches!(
        Diff::files(&missing, &missing),
        Err(Error::Io { .. })
    ));
    Ok(())
}

/// Every diff of the fixture corpus, through the public API: nothing panics, every definition
/// sits inside its file and its parent, every definition of both sides is in exactly one change,
/// and a side diffed against itself has no changed definition.
#[test]
#[cfg(feature = "test-fixtures")]
fn the_corpus_holds_the_definition_invariants() -> anyhow::Result<()> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/test/data/diffs");
    let mut pairs = Vec::new();
    for dataset in std::fs::read_dir(&root)? {
        for case in std::fs::read_dir(dataset?.path())? {
            let case = case?.path();
            let side = |prefix: &str| -> Option<std::path::PathBuf> {
                std::fs::read_dir(&case).ok()?.find_map(|e| {
                    let path = e.ok()?.path();
                    let name = path.file_name()?.to_str()?;
                    (name.starts_with(prefix) && name.ends_with(".test")).then_some(path)
                })
            };
            if let (Some(before), Some(after)) = (side("before."), side("after.")) {
                pairs.push((case, before, after));
            }
        }
    }
    assert!(
        pairs.len() > 1000,
        "found only {} fixture pairs",
        pairs.len()
    );

    let failures = std::sync::Mutex::new(Vec::new());
    let next = std::sync::atomic::AtomicUsize::new(0);
    let started = std::time::Instant::now();
    std::thread::scope(|scope| {
        for _ in 0..std::thread::available_parallelism().map_or(4, |n| n.get()) {
            scope.spawn(|| {
                while let Some((case, before, after)) =
                    pairs.get(next.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
                {
                    // `before.rs.test` names its language `before.rs`.
                    let name = |p: &std::path::Path| p.with_extension("");
                    let (Some(b), Some(a)) = (
                        std::fs::read(before).ok().and_then(|x| decode_text(&x)),
                        std::fs::read(after).ok().and_then(|x| decode_text(&x)),
                    ) else {
                        continue;
                    };
                    let diff = Diff::from_texts(&b, &a, &name(before), &name(after));
                    if let Err(problem) = check(&diff) {
                        failures
                            .lock()
                            .unwrap()
                            .push(format!("{}: {problem}", case.display()));
                    }
                    let same = Diff::from_texts(&a, &a, &name(after), &name(after));
                    if let Some(c) = same.changed_definitions().next() {
                        failures.lock().unwrap().push(format!(
                            "{}: against itself, {} is {}",
                            case.display(),
                            c.qualified_name(),
                            c.kind()
                        ));
                    }
                }
            });
        }
    });
    eprintln!(
        "{} pairs in {:.1}s",
        pairs.len(),
        started.elapsed().as_secs_f64()
    );
    let failures = failures.into_inner().unwrap();
    assert!(
        failures.is_empty(),
        "{} failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
    Ok(())
}

#[cfg(feature = "test-fixtures")]
fn check(diff: &Diff) -> Result<(), String> {
    for side in [Side::Before, Side::After] {
        let text = diff.text(side);
        let definitions = diff.definitions_of(side);
        for d in definitions {
            let span = &d.span().bytes;
            if span.end > text.len() || span.start > span.end {
                return Err(format!(
                    "{} spans {span:?} of {} bytes",
                    d.qualified_name(),
                    text.len()
                ));
            }
            if d.name().is_empty() || d.qualified_name().is_empty() {
                return Err(format!("a {} at {span:?} has no name", d.kind()));
            }
            let name = &d.name_span().bytes;
            if name.start < span.start || name.end > span.end {
                return Err(format!("{}'s name lies outside it", d.qualified_name()));
            }
            if let Some(p) = d.parent {
                let parent = &definitions[p];
                if !(parent.node_bytes.start <= d.node_bytes.start
                    && d.node_bytes.end <= parent.node_bytes.end)
                    || d.depth() != parent.depth() + 1
                {
                    return Err(format!(
                        "{} is not inside {}",
                        d.qualified_name(),
                        parent.qualified_name()
                    ));
                }
            }
        }
    }
    let count = |side: Side| {
        diff.all_definitions()
            .filter(|c| match side {
                Side::Before => c.before().is_some(),
                Side::After => c.after().is_some(),
            })
            .count()
    };
    for side in [Side::Before, Side::After] {
        if count(side) != diff.definitions_of(side).len() {
            return Err(format!(
                "{side:?}: {} definitions, {} in changes",
                diff.definitions_of(side).len(),
                count(side)
            ));
        }
    }
    Ok(())
}
