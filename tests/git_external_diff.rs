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

//! `omnidiff --headless` as git's `GIT_EXTERNAL_DIFF`, end to end: real git, in a throwaway
//! repository, handing the real binary its temporary files. The titles must name the file being
//! diffed, not git's `git-blob-XXXX/` temporaries (issue #59).

use std::path::Path;
use std::process::{Command, Output};

/// `git` in `repo`, with the user's and the system's configuration kept out.
fn git(repo: &Path, args: &[&str]) -> Command {
    let mut command = Command::new("git");
    command
        .args(args)
        .current_dir(repo)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_AUTHOR_NAME", "test")
        .env("GIT_AUTHOR_EMAIL", "test@example.com")
        .env("GIT_COMMITTER_NAME", "test")
        .env("GIT_COMMITTER_EMAIL", "test@example.com")
        .env("OMNIDIFF_CONFIG", repo.join(".omnidiff.toml"));
    command
}

fn run(repo: &Path, args: &[&str]) -> Output {
    let output = git(repo, args).output().expect("git runs");
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

/// `git diff ARGS` with omnidiff as the external diff; its stdout.
fn external_diff(repo: &Path, args: &[&str]) -> String {
    let mut command = git(repo, &["diff"]);
    // git runs this through `sh`, which would eat the backslashes of a Windows path unquoted.
    command.args(args).env(
        "GIT_EXTERNAL_DIFF",
        format!("'{}' --headless", env!("CARGO_BIN_EXE_omnidiff")),
    );
    let output = command.output().expect("git diff runs");
    assert!(
        output.status.success(),
        "git diff failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("utf-8 output")
}

/// A repository with `f.rs` committed twice, the second time changed.
fn repository() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("temp dir");
    let repo = dir.path();
    run(repo, &["init", "-q"]);
    std::fs::write(repo.join("f.rs"), "fn main() {\n    let a = 1;\n}\n").unwrap();
    run(repo, &["add", "f.rs"]);
    run(repo, &["commit", "-qm", "one"]);
    std::fs::write(repo.join("f.rs"), "fn main() {\n    let a = 2;\n}\n").unwrap();
    run(repo, &["commit", "-qam", "two"]);
    dir
}

/// Both sides come from the object database, so both files git hands over are temporaries.
#[test]
fn a_diff_of_two_commits_is_titled_with_the_files_path() {
    let dir = repository();
    let out = external_diff(dir.path(), &["HEAD~1", "HEAD", "--", "f.rs"]);
    assert!(out.contains("=== before: f.rs ===\n"), "{out}");
    assert!(out.contains("=== after: f.rs ===\n"), "{out}");
    assert!(!out.contains("git-blob-"), "{out}");
}

/// An unstaged change hands over the work tree file itself for the after side, a temporary for
/// the before side; both are titled alike.
#[test]
fn a_work_tree_change_is_titled_alike_on_both_sides() {
    let dir = repository();
    std::fs::write(dir.path().join("f.rs"), "fn main() {\n    let a = 3;\n}\n").unwrap();
    let out = external_diff(dir.path(), &["--", "f.rs"]);
    assert!(out.contains("=== before: f.rs ===\n"), "{out}");
    assert!(out.contains("=== after: f.rs ===\n"), "{out}");
}

/// A rename names the old path before and the new path after.
#[test]
fn a_rename_is_titled_with_the_old_and_the_new_path() {
    let dir = repository();
    run(dir.path(), &["mv", "f.rs", "g.rs"]);
    std::fs::write(
        dir.path().join("g.rs"),
        "fn main() {\n    let a = 2;\n    let b = 3;\n}\n",
    )
    .unwrap();
    run(dir.path(), &["commit", "-qam", "rename"]);
    let out = external_diff(dir.path(), &["-M", "HEAD~1", "HEAD"]);
    assert!(out.contains("=== before: f.rs ===\n"), "{out}");
    assert!(out.contains("=== after: g.rs ===\n"), "{out}");
}

/// An added file's missing side is `/dev/null`, as `git diff` labels it.
#[test]
fn an_added_file_has_dev_null_before() {
    let dir = repository();
    std::fs::write(dir.path().join("new.rs"), "fn new() {}\n").unwrap();
    run(dir.path(), &["add", "new.rs"]);
    let out = external_diff(dir.path(), &["--cached", "--", "new.rs"]);
    assert!(out.contains("=== before: /dev/null ===\n"), "{out}");
    assert!(out.contains("=== after: new.rs ===\n"), "{out}");
}

/// A root file named like a flag: git passes its path with no `--` in front, so clap must not
/// read it as one - an exit 2 would make git abandon the whole diff, every later file with it.
#[test]
fn files_named_like_flags_are_diffed_not_parsed() {
    let dir = repository();
    for name in ["-x.rs", "--help", "git"] {
        std::fs::write(dir.path().join(name), "fn a() {}\n").unwrap();
    }
    run(dir.path(), &["add", "--", "-x.rs", "--help", "git"]);
    run(dir.path(), &["commit", "-qm", "flags"]);
    for name in ["-x.rs", "--help", "git"] {
        std::fs::write(dir.path().join(name), "fn b() {}\n").unwrap();
    }
    let out = external_diff(dir.path(), &[]);
    for name in ["-x.rs", "--help", "git"] {
        assert!(
            out.contains(&format!("=== after: {name} ===\n")),
            "{name}: {out}"
        );
    }
    assert!(!out.contains("Usage:"), "{out}");
}

/// The same, with the binary run by hand as git would run it: `GIT_DIFF_PATH_TOTAL` set, git's
/// seven arguments, and a path that is a flag.
#[test]
fn a_flag_shaped_path_in_gits_argument_form_is_a_path() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (before, after) = (dir.path().join("a.rs"), dir.path().join("b.rs"));
    std::fs::write(&before, "fn a() {}\n").unwrap();
    std::fs::write(&after, "fn b() {}\n").unwrap();
    let hex = |digit: char| digit.to_string().repeat(40);
    let output = Command::new(env!("CARGO_BIN_EXE_omnidiff"))
        .arg("--headless")
        .arg("-x.rs")
        .arg(&before)
        .args([hex('1').as_str(), "100644"])
        .arg(&after)
        .args([hex('2').as_str(), "100644"])
        .env("GIT_DIFF_PATH_TOTAL", "1")
        .env("GIT_DIFF_PATH_COUNTER", "1")
        .env("OMNIDIFF_CONFIG", dir.path().join("config.toml"))
        .output()
        .expect("omnidiff runs");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "{stdout}{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(stdout.contains("=== before: -x.rs ===\n"), "{stdout}");
}

/// A path is a hostile commit's to choose, escape sequences and all; it must reach the terminal
/// as visible text, not as a command to it.
#[cfg(unix)]
#[test]
fn a_path_with_terminal_escapes_is_printed_escaped() {
    let dir = repository();
    let text = "evil\u{1b}]0;pwned\u{7}\u{1b}[2J.rs";
    let binary = "bin\u{1b}[31m\u{202e}.dat";
    std::fs::write(dir.path().join(text), "fn a() {}\n").unwrap();
    std::fs::write(dir.path().join(binary), [0u8, 159, 146, 150]).unwrap();
    run(dir.path(), &["add", "--", text, binary]);
    run(dir.path(), &["commit", "-qm", "escapes"]);
    std::fs::write(dir.path().join(text), "fn b() {}\n").unwrap();
    std::fs::write(dir.path().join(binary), [0u8, 1, 2, 3]).unwrap();
    let out = external_diff(dir.path(), &["--", text, binary]);
    let plain = out.replace("\u{1b}[0m", "").replace("\u{1b}[90m", "");
    assert!(
        plain.contains("=== after: evil\\x1b]0;pwned\\x07\\x1b[2J.rs ===\n"),
        "{out:?}"
    );
    assert!(
        plain.contains("Binary file bin\\x1b[31m\\u{202e}.dat differs\n"),
        "{out:?}"
    );
    assert!(!out.contains("\u{1b}]"), "{out:?}");
    assert!(!out.contains("\u{1b}[2J"), "{out:?}");
    assert!(!out.contains('\u{202e}'), "{out:?}");
}
