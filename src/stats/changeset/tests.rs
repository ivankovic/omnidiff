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

/// `count` distinct lines of code, long enough that four make a block.
fn lines(tag: &str, count: usize) -> String {
    (0..count)
        .map(|i| format!("let {tag}_value_{i} = compute_{tag}(input_{i}) + offset_{tag}_{i};\n"))
        .collect()
}

fn file(path: &str, before: Option<&str>, after: Option<&str>) -> ChangedFile {
    ChangedFile {
        path: path.to_string(),
        before: before.map(|text| Blob::of(text.as_bytes())),
        after: after.map(|text| Blob::of(text.as_bytes())),
    }
}

/// A file with an edit no signal reads anything into.
fn bystander() -> ChangedFile {
    file(
        "notes.rs",
        Some("let version = 1;\n"),
        Some("let version = 2;\n"),
    )
}

fn rename_hint(old: &str, new: &str, similarity: u8) -> VcsHint {
    VcsHint {
        status: 'R',
        path: new.to_string(),
        old_path: Some(old.to_string()),
        similarity: Some(similarity),
    }
}

fn classified(files: &[ChangedFile]) -> Classification {
    classify(files, &[], &NoUntouched)
}

#[test]
fn nothing_special_is_control() {
    let before = lines("a", 10);
    let after = before.replace("offset_a_3", "offset_a_33");
    let result = classified(&[file("a.rs", Some(&before), Some(&after)), bystander()]);
    assert!(result.signals.is_empty());
    assert!(result.signals.is_empty());
    assert!(result.untouched.is_empty());
}

#[test]
fn a_blob_under_another_path_is_a_rename_unedited() {
    let text = lines("a", 10);
    let result = classified(&[
        file("old.rs", Some(&text), None),
        file("new.rs", None, Some(&text)),
        bystander(),
    ]);
    // The file moved whole: its blocks are not also a block move.
    assert_eq!(result.signals, vec![Signal::RenameUnedited]);
    // An empty file is every empty file.
    let result = classified(&[
        file("a/__init__.py", Some(""), None),
        file("b/__init__.py", None, Some("")),
    ]);
    assert!(result.signals.is_empty());
}

#[test]
fn a_picture_or_binary_under_another_path_is_a_non_code_move() {
    let mut png = Vec::new();
    image::RgbaImage::from_pixel(4, 4, image::Rgba([9, 0, 0, 255]))
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .unwrap();
    for bytes in [png, vec![0u8, 0xff, 0x80, 0x81, 0x00, 0x9f]] {
        let moved = |path: &str, before: bool| ChangedFile {
            path: path.to_string(),
            before: before.then(|| Blob::of(&bytes)),
            after: (!before).then(|| Blob::of(&bytes)),
        };
        let result = classified(&[
            moved("old/logo", true),
            moved("new/logo", false),
            bystander(),
        ]);
        assert_eq!(result.signals, vec![Signal::NonCodeMove]);
    }
}

#[test]
fn a_rename_edited_is_what_the_vcs_reports_below_full_similarity() {
    let before = lines("a", 12);
    let after = before.replace("offset_a_11", "offset_a_111");
    let files = [
        file("old.rs", Some(&before), None),
        file("new.rs", None, Some(&after)),
        bystander(),
    ];
    let result = classify(&files, &[rename_hint("old.rs", "new.rs", 93)], &NoUntouched);
    assert_eq!(result.signals, vec![Signal::RenameEdited]);

    // Unreported, the blocks are all that says where the file went.
    assert_eq!(classified(&files).signals, vec![Signal::BlockMove]);
    // Full similarity is a rename unedited, and the blobs say so themselves.
    let result = classify(
        &files,
        &[rename_hint("old.rs", "new.rs", 100)],
        &NoUntouched,
    );
    assert_eq!(result.signals, vec![Signal::BlockMove]);
    // A hint about files the changeset does not hold is about nothing.
    let result = classify(
        &files,
        &[rename_hint("gone.rs", "new.rs", 80)],
        &NoUntouched,
    );
    assert_eq!(result.signals, vec![Signal::BlockMove]);
}

#[test]
fn three_files_under_one_prefix_substitution_are_a_directory_move() {
    let moves = directory_moves(
        &[
            "src/old/a.rs",
            "src/old/sub/b.rs",
            "src/old/c.rs",
            "src/other/d.rs",
        ],
        &[
            "lib/new/a.rs",
            "lib/new/sub/b.rs",
            "lib/new/c.rs",
            "lib/elsewhere/d.rs",
        ],
    );
    assert_eq!(
        moves,
        vec![("src/old".to_string(), "lib/new".to_string(), 3)]
    );
    // Two are two renames; and a file name alone does not make a move of a file left in place.
    assert!(directory_moves(&["old/a.rs", "old/b.rs"], &["new/a.rs", "new/b.rs"]).is_empty());
    // Out of the root.
    let moves = directory_moves(
        &["a.rs", "b.rs", "c.rs"],
        &["src/a.rs", "src/b.rs", "src/c.rs"],
    );
    assert_eq!(moves, vec![(String::new(), "src".to_string(), 3)]);

    let files: Vec<ChangedFile> = ["a", "b", "c"]
        .iter()
        .flat_map(|name| {
            let before = lines(name, 8);
            let after = before.replace("input_0", "argument_0");
            [
                file(&format!("old/{name}.rs"), Some(&before), None),
                file(&format!("new/{name}.rs"), None, Some(&after)),
            ]
        })
        .collect();
    let hints: Vec<VcsHint> = ["a", "b", "c"]
        .iter()
        .map(|name| rename_hint(&format!("old/{name}.rs"), &format!("new/{name}.rs"), 90))
        .collect();
    let result = classify(&files, &hints, &NoUntouched);
    assert_eq!(
        result.signals,
        vec![Signal::DirectoryMove, Signal::RenameEdited]
    );
}

#[test]
fn a_block_removed_from_one_file_and_added_to_another_is_a_block_move() {
    let (keep, block, other) = (lines("keep", 6), lines("moved", 5), lines("other", 6));
    // Reindented and spread out with blank lines and braces on the way.
    let landed: String = block
        .lines()
        .map(|line| format!("        {line}\n\n    }}\n"))
        .collect();
    let result = classified(&[
        file("a.rs", Some(&format!("{keep}{block}")), Some(&keep)),
        file("b.rs", Some(&other), Some(&format!("{other}{landed}"))),
    ]);
    assert_eq!(result.signals, vec![Signal::BlockMove]);
    assert!(
        result.evidence[0].contains("a.rs -> b.rs"),
        "{:?}",
        result.evidence
    );

    // Three lines are not a block.
    let short = lines("moved", 3);
    let result = classified(&[
        file("a.rs", Some(&format!("{keep}{short}")), Some(&keep)),
        file("b.rs", Some(&other), Some(&format!("{other}{short}"))),
    ]);
    assert!(result.signals.is_empty());
}

#[test]
fn lines_of_structure_do_not_make_a_block() {
    let (keep, other) = (lines("keep", 6), lines("other", 6));
    let structure = "}\n});\nelse\n    return;\n{\n} else {\nend\n*/\n";
    let result = classified(&[
        file("a.rs", Some(&format!("{keep}{structure}")), Some(&keep)),
        file("b.rs", Some(&other), Some(&format!("{other}{structure}"))),
    ]);
    assert!(result.signals.is_empty());
    assert!(is_trivial("  "));
    assert!(is_trivial("});"));
    assert!(is_trivial("} else {"));
    assert!(!is_trivial("x += 1;"));
    for import in [
        "import os",
        "from a import b",
        "use std::fmt;",
        "#include <stdio.h>",
        "using System;",
    ] {
        assert!(is_trivial(import), "{import}");
    }
    assert!(!is_trivial("user = load()"));
    for string in ["\"net/http\"", "zap \"go.uber.org/zap\"", "\"a\","] {
        assert!(is_trivial(string), "{string}");
    }
    assert!(!is_trivial("\"key\": \"value\","));
    assert!(!is_trivial("name = \"value\""));
}

#[test]
fn a_block_moved_inside_its_file_is_nothing_across_files() {
    let (top, block, bottom) = (lines("top", 6), lines("moved", 6), lines("bottom", 6));
    let result = classified(&[
        file(
            "a.rs",
            Some(&format!("{top}{block}{bottom}")),
            Some(&format!("{top}{bottom}{block}")),
        ),
        bystander(),
    ]);
    assert!(result.signals.is_empty());
}

#[test]
fn one_file_into_two_is_a_split_and_two_into_one_a_merge() {
    let (first, second) = (lines("first", 8), lines("second", 8));
    let whole = format!("{first}{second}");
    let result = classified(&[
        file("whole.rs", Some(&whole), None),
        file("first.rs", None, Some(&first)),
        file("second.rs", None, Some(&second)),
    ]);
    assert_eq!(
        result.signals,
        vec![Signal::BlockMove, Signal::SplitOrMerge]
    );
    assert!(
        result
            .evidence
            .iter()
            .any(|line| line.contains("split of whole.rs"))
    );

    let result = classified(&[
        file("first.rs", Some(&first), None),
        file("second.rs", Some(&second), None),
        file("whole.rs", None, Some(&whole)),
    ]);
    assert!(result.signals.contains(&Signal::SplitOrMerge));
    assert!(
        result
            .evidence
            .iter()
            .any(|line| line.contains("merge of whole.rs"))
    );
}

#[test]
fn a_block_added_that_another_file_keeps_is_a_copy() {
    let (source, block, target) = (lines("source", 6), lines("shared", 12), lines("target", 6));
    // From a changed file that keeps it.
    let kept = format!("{source}{block}");
    let result = classified(&[
        file(
            "a.rs",
            Some(&kept),
            Some(&kept.replace("input_0", "argument_0")),
        ),
        file("b.rs", Some(&target), Some(&format!("{target}{block}"))),
    ]);
    assert_eq!(result.signals, vec![Signal::Copy]);
    assert!(result.untouched.is_empty());

    // From a file the commit does not touch: recorded, for the fixture to hold.
    let files = [
        file("b.rs", Some(&target), Some(&format!("{target}{block}"))),
        bystander(),
    ];
    let tree =
        |paths: &[&str]| UntouchedTexts::new(paths.iter().map(|path| (*path, kept.as_str())));
    let result = classify(&files, &[], &tree(&["lib/source.rs"]));
    assert_eq!(result.signals, vec![Signal::Copy]);
    assert_eq!(result.untouched, vec!["lib/source.rs"]);

    // Another language's file is not searched for sources, and what many files hold is no copy.
    assert!(
        classify(&files, &[], &tree(&["source.py"]))
            .signals
            .is_empty()
    );
    let everywhere = tree(&["x.rs", "y.rs", "z.rs"]);
    assert!(classify(&files, &[], &everywhere).signals.is_empty());
}

#[test]
fn a_short_repeated_idiom_is_not_a_copy() {
    let (source, target) = (lines("source", 6), lines("target", 6));
    let idiom = lines("idiom", BLOCK_LINES + COPY_BLOCKS - 2);
    let kept = format!("{source}{idiom}");
    let result = classify(
        &[
            file("b.rs", Some(&target), Some(&format!("{target}{idiom}"))),
            bystander(),
        ],
        &[],
        &UntouchedTexts::new([("a.rs", kept.as_str())]),
    );
    assert!(result.signals.is_empty());
}

#[test]
fn a_header_every_file_has_moves_nothing() {
    let header = lines("licence", 8);
    let old = format!("{header}{}", lines("old", 8));
    let new = format!("{header}{}", lines("new", 8));
    let files = [
        file("old.rs", Some(&old), None),
        file("new.rs", None, Some(&new)),
    ];
    let texts: Vec<(&str, String)> = ["x.rs", "y.rs", "z.rs"]
        .iter()
        .map(|path| (*path, format!("{header}{}", lines(path, 3))))
        .collect();
    let others = UntouchedTexts::new(texts.iter().map(|(path, text)| (*path, text.as_str())));
    assert!(classify(&files, &[], &others).signals.is_empty());
    // Without the rest of the tree to say so, it reads as moved.
    assert!(classified(&files).signals.contains(&Signal::BlockMove));

    // Nor does a header that three files of the changeset itself lose and gain.
    let files: Vec<ChangedFile> = ["a", "b", "c"]
        .iter()
        .flat_map(|name| {
            [
                file(
                    &format!("{name}_old.rs"),
                    Some(&format!("{header}{}", lines(&format!("{name}old"), 8))),
                    None,
                ),
                file(
                    &format!("{name}_new.rs"),
                    None,
                    Some(&format!("{header}{}", lines(&format!("{name}new"), 8))),
                ),
            ]
        })
        .collect();
    assert!(classified(&files).signals.is_empty());
}

/// Twelve lines and the same with every third line's call renamed: no four in a row survive.
fn edited_pair() -> (String, String) {
    let block = lines("moved", 12);
    let edited: String = block
        .lines()
        .enumerate()
        .map(|(i, line)| {
            if i % 3 == 0 {
                format!("{}\n", line.replace("compute_moved", "derive_moved"))
            } else {
                format!("{line}\n")
            }
        })
        .collect();
    (block, edited)
}

#[test]
fn lines_that_leave_one_file_and_arrive_edited_in_another_are_moved_and_edited() {
    let (block, edited) = edited_pair();
    let (keep, other) = (lines("keep", 6), lines("other", 6));
    let result = classified(&[
        file("a.rs", Some(&format!("{keep}{block}")), Some(&keep)),
        file("b.rs", Some(&other), Some(&format!("{other}{edited}"))),
    ]);
    assert_eq!(result.signals, vec![Signal::MovedAndEdited]);

    // Unrelated lines of the same length are not.
    let unrelated = lines("unrelated", 12);
    let result = classified(&[
        file("a.rs", Some(&format!("{keep}{block}")), Some(&keep)),
        file("b.rs", Some(&other), Some(&format!("{other}{unrelated}"))),
    ]);
    assert!(result.signals.is_empty());
}

#[test]
fn one_edit_made_to_two_files_is_not_a_move() {
    let (block, edited) = edited_pair();
    let (keep, other) = (lines("keep", 6), lines("other", 6));
    let result = classified(&[
        file(
            "a.rs",
            Some(&format!("{keep}{block}")),
            Some(&format!("{keep}{edited}")),
        ),
        file(
            "b.rs",
            Some(&format!("{other}{block}")),
            Some(&format!("{other}{edited}")),
        ),
    ]);
    assert!(result.signals.is_empty());
}

#[test]
fn one_stem_in_two_languages_is_cross_language() {
    assert_eq!(
        cross_language_pairs(
            &["src/Greeter.java", "src/Other.java"],
            &["kt/Greeter.kt", "x/Other.java"]
        ),
        vec![("src/Greeter.java", "kt/Greeter.kt")]
    );
    let result = classified(&[
        file("Greeter.java", Some("class Greeter {}\n"), None),
        file("Greeter.kt", None, Some("class Greeter\n")),
    ]);
    assert_eq!(result.signals, vec![Signal::CrossLanguage]);
    // Renamed and not yet translated: the same blob.
    let result = classified(&[
        file("Greeter.java", Some("class Greeter {}\n"), None),
        file("Greeter.kt", None, Some("class Greeter {}\n")),
    ]);
    assert_eq!(result.signals, vec![Signal::RenameUnedited]);
    // A script replaced by a workflow of its name was not translated.
    assert!(cross_language_pairs(&["scripts/deploy.js"], &["ci/deploy.yml"]).is_empty());
}

#[test]
fn a_changeset_carries_every_signal_it_fires_and_none_ranks_first() {
    // A directory renamed, one of its files translated and a function moved out of another.
    let moved = lines("moved", 6);
    let mut files: Vec<ChangedFile> = ["a", "b", "c"]
        .iter()
        .flat_map(|name| {
            let text = lines(name, 8);
            [
                file(&format!("old/{name}.rs"), Some(&text), None),
                file(&format!("new/{name}.rs"), None, Some(&text)),
            ]
        })
        .collect();
    files.push(file("old/Greeter.java", Some("class Greeter {}\n"), None));
    files.push(file("new/Greeter.kt", None, Some("class Greeter\n")));
    let keep = lines("keep", 6);
    files.push(file("lib.rs", Some(&format!("{keep}{moved}")), Some(&keep)));
    files.push(file("util.rs", None, Some(&moved)));
    let result = classified(&files);
    assert_eq!(
        result.signals,
        vec![
            Signal::BlockMove,
            Signal::CrossLanguage,
            Signal::DirectoryMove,
            Signal::RenameUnedited
        ]
    );
    assert_eq!(
        signals_column(&result),
        "block-move;cross-language;directory-move;rename-unedited"
    );
    assert_eq!(
        signals_column(&classified(&[bystander(), bystander()])),
        CONTROL
    );
    // The names are sorted as the signals are, and a bucket is a file count's.
    let names: Vec<&str> = Signal::ALL.iter().map(|signal| signal.name()).collect();
    assert!(names.is_sorted() && !names.contains(&CONTROL));
    assert_eq!(
        [2, 3, 4, 8, 9, 20].map(file_bucket),
        ["2-3", "2-3", "4-8", "4-8", "9-20", "9-20"]
    );
}

#[test]
fn changed_lines_and_main_language_summarise_a_changeset() {
    let before = lines("a", 10);
    let after = before.replace("offset_a_3", "offset_a_33");
    let files = [
        file("a.rs", Some(&before), Some(&after)),
        file("b.rs", None, Some("one\ntwo\nthree\n")),
        file("c.py", Some("one\ntwo\n"), None),
        file("README", None, Some("text\n")),
    ];
    // One line replaced, three added, two removed, one added.
    assert_eq!(changed_lines(&files), 2 + 3 + 2 + 1);
    assert_eq!(main_language(&files), "Rust");
    assert_eq!(main_language(&files[3..]), "Other");
}

#[test]
fn name_status_output_parses_into_hints() {
    let output = b"M\0src/a.rs\0R087\0old/b.rs\0new/b.rs\0A\0c.rs\0D\0d.rs\0C100\0e.rs\0f.rs\0";
    let hints = parse_name_status(output);
    assert_eq!(hints.len(), 5);
    assert_eq!(hints[1], rename_hint("old/b.rs", "new/b.rs", 87));
    assert_eq!(
        (hints[0].status, hints[0].path.as_str(), &hints[0].old_path),
        ('M', "src/a.rs", &None)
    );
    assert_eq!((hints[4].status, hints[4].similarity), ('C', Some(100)));
    assert!(parse_name_status(b"").is_empty());
}

#[test]
fn only_plain_relative_paths_are_safe() {
    for path in ["a.rs", "src/deep/a.rs", "a b/c.rs", "..a/b"] {
        assert!(is_safe_path(path), "{path}");
    }
    for path in [
        "",
        "/etc/passwd",
        "../a",
        "a/../../b",
        "a/./b",
        "a//b",
        "a/",
        "a\\..\\b",
        "a\0b",
        ".git/config",
        "a/.git/hooks/x",
    ] {
        assert!(!is_safe_path(path), "{path}");
    }
}

#[test]
fn rows_survive_the_csv_and_name_their_signals_and_untouched_files() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("changesets.csv");
    assert!(read_rows(&path)?.is_empty());
    let row = |kind: &str, commit: &str| Row {
        name: fixture_name(kind, "Example-Repo.git", &"f".repeat(40), commit),
        kind: kind.to_string(),
        repository: "Example-Repo.git".to_string(),
        commit: commit.to_string(),
        parent: "f".repeat(40),
        root: if kind == KIND_FOLDERS { "src/a, b" } else { "" }.to_string(),
        signals: "block-move;copy".to_string(),
        untouched_paths: "src/a, b.rs;c.rs".to_string(),
        files: 3,
        changed_lines: 42,
        main_language: "Rust".to_string(),
        bucket: "2-3".to_string(),
        dataset: "pilot".to_string(),
        status: "SAMPLED".to_string(),
        comment: "a \"quoted\" note".to_string(),
    };
    let rows = vec![
        row(KIND_FOLDERS, &"a".repeat(40)),
        row(KIND_COMMIT, &"b".repeat(40)),
    ];
    write_rows(&path, rows.clone())?;
    let read = read_rows(&path)?;
    assert_eq!(
        read,
        vec![rows[1].clone(), rows[0].clone()],
        "sorted by kind"
    );
    assert_eq!(read[0].name, "commit-x-example-repo-bbbbbbbb");
    assert_eq!(read[1].name, "folders-x-example-repo-ffffffff-aaaaaaaa");
    assert_eq!(read[0].untouched(), vec!["src/a, b.rs", "c.rs"]);
    assert_eq!(read[0].signal_names(), vec!["block-move", "copy"]);
    let mut control = rows[1].clone();
    control.signals = CONTROL.to_string();
    control.untouched_paths.clear();
    assert_eq!(control.signal_names(), vec![CONTROL]);
    assert!(control.untouched().is_empty());
    let header = std::fs::read_to_string(&path)?;
    assert!(header.starts_with(&CSV_HEADER.join(",")));
    Ok(())
}

/// Commits `entries` (path, bytes, mode) as a tree on top of `parent`.
fn commit_tree(
    repo: &Repository,
    parent: Option<Oid>,
    entries: &[(&str, &[u8], i32)],
) -> Result<Oid> {
    let mut index = git2::Index::new()?;
    for (path, bytes, mode) in entries {
        let id = if *mode == 0o160000 {
            Oid::from_str(&"1".repeat(40))?
        } else {
            repo.blob(bytes)?
        };
        index.add(&git2::IndexEntry {
            ctime: git2::IndexTime::new(0, 0),
            mtime: git2::IndexTime::new(0, 0),
            dev: 0,
            ino: 0,
            mode: *mode as u32,
            uid: 0,
            gid: 0,
            file_size: 0,
            id,
            flags: 0,
            flags_extended: 0,
            path: path.as_bytes().to_vec(),
        })?;
    }
    let tree = repo.find_tree(index.write_tree_to(repo)?)?;
    let signature = git2::Signature::now("t", "t@example.com")?;
    let parents: Vec<git2::Commit> = parent
        .map(|id| repo.find_commit(id))
        .transpose()?
        .into_iter()
        .collect();
    let parents: Vec<&git2::Commit> = parents.iter().collect();
    Ok(repo.commit(None, &signature, &signature, "c", &tree, &parents)?)
}

#[test]
fn a_changeset_holds_regular_files_somebody_wrote() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let repo = Repository::init(dir.path())?;
    let generated = b"// Code generated by protoc. DO NOT EDIT.\nlet x = 2;\n";
    let first = commit_tree(
        &repo,
        None,
        &[
            ("src/a.rs", b"let a = 1;\n", 0o100644),
            ("src/b.rs", b"let b = 1;\n", 0o100644),
            ("src/mode.rs", b"let m = 1;\n", 0o100644),
            ("link", b"src/a.rs", 0o120000),
            ("module", b"", 0o160000),
            ("Cargo.lock", b"version = 1\n", 0o100644),
            ("third_party/x.rs", b"let x = 1;\n", 0o100644),
            (".github/workflows/ci.yml", b"on: push\n", 0o100644),
        ],
    )?;
    let second = commit_tree(
        &repo,
        Some(first),
        &[
            ("src/a.rs", b"let a = 2;\n", 0o100644),
            ("src/c.rs", b"let c = 1;\n", 0o100644),
            ("src/mode.rs", b"let m = 1;\n", 0o100755),
            ("src/gen.rs", generated, 0o100644),
            ("link", b"src/c.rs", 0o120000),
            ("Cargo.lock", b"version = 2\n", 0o100644),
            ("third_party/x.rs", b"let x = 2;\n", 0o100644),
            (".github/workflows/ci.yml", b"on: pull_request\n", 0o100644),
        ],
    )?;
    let before = repo.find_commit(first)?.tree()?;
    let after = repo.find_commit(second)?.tree()?;
    let files = load_changes(&repo, &before, &after)?.unwrap();
    let paths: Vec<(&str, bool, bool)> = files
        .iter()
        .map(|file| {
            (
                file.path.as_str(),
                file.before.is_some(),
                file.after.is_some(),
            )
        })
        .collect();
    // Not the link, the submodule, the mode change, the lock file, the vendored file, the
    // workflow or the generated file.
    assert_eq!(
        paths,
        vec![
            ("src/a.rs", true, true),
            ("src/b.rs", true, false),
            ("src/c.rs", false, true),
        ]
    );
    assert!(regular_blob(&repo, &after, "link").is_err());
    assert!(regular_blob(&repo, &after, "src").is_err());
    assert!(regular_blob(&repo, &after, "../x").is_err());
    assert_eq!(regular_blob(&repo, &after, "src/c.rs")?, b"let c = 1;\n");

    // One file left is not a changeset.
    let third = commit_tree(
        &repo,
        Some(second),
        &[("src/a.rs", b"let a = 3;\n", 0o100644)],
    )?;
    let alone = repo.find_commit(third)?.tree()?;
    let one = commit_tree(
        &repo,
        Some(third),
        &[("src/a.rs", b"let a = 4;\n", 0o100644)],
    )?;
    let one = repo.find_commit(one)?.tree()?;
    assert_eq!(
        load_changes(&repo, &alone, &one)?.unwrap_err(),
        Skip::TooFewFiles
    );
    let large = vec![b'a'; MAX_FILE_BYTES + 1];
    let big = commit_tree(
        &repo,
        Some(third),
        &[
            ("src/a.rs", &large, 0o100644),
            ("src/b.rs", b"b\n", 0o100644),
        ],
    )?;
    let big = repo.find_commit(big)?.tree()?;
    assert_eq!(
        load_changes(&repo, &alone, &big)?.unwrap_err(),
        Skip::FileTooLarge
    );
    Ok(())
}

#[test]
fn the_index_finds_an_untouched_source_in_the_tree_it_is_asked_about() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let repo = Repository::init(dir.path())?;
    let (block, target) = (lines("shared", 12), lines("target", 6));
    let source = format!("{}{block}", lines("source", 6));
    let copied = format!("{target}{block}");
    let first = commit_tree(
        &repo,
        None,
        &[
            ("lib/source.rs", source.as_bytes(), 0o100644),
            ("src/target.rs", target.as_bytes(), 0o100644),
            ("notes.rs", b"let version = 1;\n", 0o100644),
        ],
    )?;
    let second = commit_tree(
        &repo,
        Some(first),
        &[
            ("lib/source.rs", source.as_bytes(), 0o100644),
            ("src/target.rs", copied.as_bytes(), 0o100644),
            ("notes.rs", b"let version = 2;\n", 0o100644),
        ],
    )?;
    // Later the source loses the block; the index still knows the path once held it.
    let third = commit_tree(
        &repo,
        Some(second),
        &[
            ("lib/source.rs", lines("source", 6).as_bytes(), 0o100644),
            ("src/target.rs", copied.as_bytes(), 0o100644),
            ("notes.rs", b"let version = 2;\n", 0o100644),
        ],
    )?;
    let tree_of = |id: Oid| -> Result<Tree> { Ok(repo.find_commit(id)?.tree()?) };
    let (before, after, later) = (tree_of(first)?, tree_of(second)?, tree_of(third)?);
    let files = load_changes(&repo, &before, &after)?.unwrap();
    let mut index = BlockIndex::default();
    // Newest first, as a walk goes.
    index.add_tree(&repo, &later);
    index.add_tree(&repo, &before);
    let result = classify(&files, &[], &index.view(&repo, &before, &files));
    assert_eq!(result.signals, vec![Signal::Copy]);
    assert_eq!(result.untouched, vec!["lib/source.rs"]);

    // Seen from the tree where the source no longer holds it, there is no source.
    let block = *blocks_of(&block).iter().next().unwrap();
    assert!(matches!(
        index.view(&repo, &later, &[]).holders(block),
        Holders::Paths(paths) if paths == ["src/target.rs"]
    ));
    assert!(matches!(
        index.view(&repo, &before, &[]).holders(block),
        Holders::Paths(paths) if paths == ["lib/source.rs"]
    ));
    Ok(())
}

#[test]
fn git_reports_a_rename_with_its_similarity() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let repo = Repository::init(dir.path())?;
    let before = lines("a", 12);
    let after = before.replace("offset_a_11", "offset_a_111");
    let first = commit_tree(&repo, None, &[("old.rs", before.as_bytes(), 0o100644)])?;
    let second = commit_tree(
        &repo,
        Some(first),
        &[("new.rs", after.as_bytes(), 0o100644)],
    )?;
    let hints = vcs_hints(&repo, first, second)?;
    assert_eq!(hints.len(), 1);
    assert_eq!((hints[0].status, hints[0].path.as_str()), ('R', "new.rs"));
    assert_eq!(hints[0].old_path.as_deref(), Some("old.rs"));
    assert!(
        hints[0]
            .similarity
            .is_some_and(|similarity| (50..100).contains(&similarity))
    );
    Ok(())
}

/// A repository of two commits whose trees differ in `src/app/` (two files changed, one added)
/// and in `docs/` (one file), with a link, a large unchanged file and plenty of bystanders.
fn folder_repository(repo: &Repository) -> Result<(Oid, Oid)> {
    let large = vec![b'x'; MAX_FILE_BYTES + 1];
    let others: Vec<(String, Vec<u8>)> = (0..6)
        .map(|i| {
            (
                format!("vendor/lib{i}.rs"),
                format!("let v = {i};\n").into_bytes(),
            )
        })
        .collect();
    let tree = |app: &[(&str, &str)], docs: &str| -> Vec<(String, Vec<u8>, i32)> {
        let mut entries: Vec<(String, Vec<u8>, i32)> = vec![
            (
                "docs/guide.md".to_string(),
                docs.as_bytes().to_vec(),
                0o100644,
            ),
            (
                "src/app/same.rs".to_string(),
                b"let same = 1;\n".to_vec(),
                0o100644,
            ),
            (
                "src/app/logo.bin".to_string(),
                vec![0, 159, 146, 150],
                0o100644,
            ),
            ("src/app/big.dat".to_string(), large.clone(), 0o100644),
            ("src/app/link".to_string(), b"same.rs".to_vec(), 0o120000),
            (
                "src/other.rs".to_string(),
                b"let other = 1;\n".to_vec(),
                0o100644,
            ),
        ];
        entries.extend(app.iter().map(|(name, text)| {
            (
                format!("src/app/{name}"),
                text.as_bytes().to_vec(),
                0o100644,
            )
        }));
        entries.extend(
            others
                .iter()
                .map(|(path, bytes)| (path.clone(), bytes.clone(), 0o100644)),
        );
        entries
    };
    let commit = |parent: Option<Oid>, entries: Vec<(String, Vec<u8>, i32)>| -> Result<Oid> {
        let borrowed: Vec<(&str, &[u8], i32)> = entries
            .iter()
            .map(|(path, bytes, mode)| (path.as_str(), bytes.as_slice(), *mode))
            .collect();
        commit_tree(repo, parent, &borrowed)
    };
    let (keep, moved) = (lines("keep", 6), lines("moved", 6));
    let first = commit(
        None,
        tree(
            &[
                ("a.rs", &format!("{keep}{moved}")),
                ("deep/b.rs", "let b = 1;\n"),
            ],
            "one\n",
        ),
    )?;
    let second = commit(
        Some(first),
        tree(
            &[
                ("a.rs", &keep),
                ("deep/b.rs", "let b = 2;\n"),
                ("deep/c.rs", &moved),
            ],
            "two\n",
        ),
    )?;
    Ok((first, second))
}

#[test]
fn the_folder_is_the_deepest_directory_with_the_most_changed_files_under_the_cap() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let repo = Repository::init(dir.path())?;
    let (first, second) = folder_repository(&repo)?;
    let before = repo.find_commit(first)?.tree()?;
    let after = repo.find_commit(second)?.tree()?;
    // Everything changed is under the root, which fits: 4 changed files, 16 a side at most.
    assert_eq!(
        choose_folder(&repo, &before, &after, 200)?,
        Ok(String::new())
    );
    // Too small for the root (16 files) or `src` (9): `src/app`, with 3 of the 4 changes.
    assert_eq!(
        choose_folder(&repo, &before, &after, 8)?,
        Ok("src/app".to_string())
    );
    // `src/app/deep` holds two changed files in two.
    assert_eq!(
        choose_folder(&repo, &before, &after, 2)?,
        Ok("src/app/deep".to_string())
    );
    assert_eq!(
        choose_folder(&repo, &before, &after, 1)?,
        Err(Skip::NoFolder)
    );
    // Nothing changed between a tree and itself.
    assert_eq!(
        choose_folder(&repo, &after, &after, 200)?,
        Err(Skip::NoFolder)
    );
    Ok(())
}

#[test]
fn a_folder_is_two_complete_trees_with_what_is_left_out_listed() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let repo = Repository::init(dir.path())?;
    let (first, second) = folder_repository(&repo)?;
    let before = repo.find_commit(first)?.tree()?;
    let after = repo.find_commit(second)?.tree()?;
    let folder = load_folder(&repo, &before, &after, "src/app")?;
    assert_eq!(folder.root_name(), "src/app");
    let changed: Vec<(&str, bool, bool)> = folder
        .changed
        .iter()
        .map(|file| {
            (
                file.path.as_str(),
                file.before.is_some(),
                file.after.is_some(),
            )
        })
        .collect();
    // Relative to the folder; nothing of `docs/` or `vendor/`.
    assert_eq!(
        changed,
        vec![
            ("a.rs", true, true),
            ("deep/b.rs", true, true),
            ("deep/c.rs", false, true)
        ]
    );
    let unchanged: Vec<&str> = folder
        .unchanged
        .iter()
        .map(|(path, _)| path.as_str())
        .collect();
    assert_eq!(
        unchanged,
        vec!["logo.bin", "same.rs"],
        "a binary file is held as it is"
    );
    let omitted: Vec<(&str, &str, &str, Option<u64>)> = folder
        .omitted
        .iter()
        .map(|omitted| {
            (
                omitted.path.as_str(),
                omitted.side,
                omitted.reason,
                omitted.bytes,
            )
        })
        .collect();
    let large = Some(MAX_FILE_BYTES as u64 + 1);
    assert_eq!(
        omitted,
        vec![
            ("big.dat", "after", "size", large),
            ("big.dat", "before", "size", large),
            ("link", "after", "symlink", Some(7)),
            ("link", "before", "symlink", Some(7)),
        ]
    );
    assert_eq!(folder.omitted[0].id.len(), 40);

    // Classified over the folder, with no hints: the function went from `a.rs` to `deep/c.rs`.
    let result = classify(&folder.changed, &[], &folder.untouched());
    assert_eq!(result.signals, vec![Signal::BlockMove]);

    assert_eq!(load_folder(&repo, &before, &after, "")?.root_name(), ".");
    assert!(load_folder(&repo, &before, &after, "../x").is_err());
    // A folder one side does not have is every file added.
    let folder = load_folder(
        &repo,
        &repo.find_tree(repo.treebuilder(None)?.write()?)?,
        &after,
        "docs",
    )?;
    assert_eq!(folder.changed.len(), 1);
    assert!(folder.changed[0].before.is_none());
    Ok(())
}
