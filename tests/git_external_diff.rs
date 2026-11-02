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
    command.args(args).env(
        "GIT_EXTERNAL_DIFF",
        format!("{} --headless", env!("CARGO_BIN_EXE_omnidiff")),
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
