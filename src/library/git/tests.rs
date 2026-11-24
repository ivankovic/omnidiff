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
//! Repositories built with the `git` binary, so what gix reads is what git wrote.

use super::*;
use crate::library::ChangeKind;

struct Fixture {
    dir: tempfile::TempDir,
}

impl Fixture {
    fn new() -> Self {
        let fixture = Self {
            dir: tempfile::tempdir().expect("a temporary directory"),
        };
        fixture.git(&["init", "-q", "-b", "main"]);
        fixture
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }

    fn git(&self, args: &[&str]) {
        let status = std::process::Command::new("git")
            .args(args)
            .current_dir(self.path())
            .env("GIT_AUTHOR_NAME", "t")
            .env("GIT_AUTHOR_EMAIL", "t@example.com")
            .env("GIT_COMMITTER_NAME", "t")
            .env("GIT_COMMITTER_EMAIL", "t@example.com")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .status()
            .expect("git runs");
        assert!(status.success(), "git {args:?}");
    }

    fn write(&self, path: &str, contents: &[u8]) {
        let path = self.path().join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }

    fn commit(&self, message: &str) {
        self.git(&["add", "-A"]);
        self.git(&["commit", "-q", "-m", message]);
    }
}

fn statuses(set: &ChangeSet) -> Vec<(FileStatus, String)> {
    set.files()
        .iter()
        .map(|f| (f.status(), f.path().display().to_string()))
        .collect()
}

fn definitions(set: &ChangeSet) -> Vec<(String, ChangeKind, String)> {
    set.changed_definitions()
        .map(|(file, change)| {
            (
                file.path().display().to_string(),
                change.kind(),
                change.qualified_name().to_string(),
            )
        })
        .collect()
}

/// A change set can be built on one thread and its files diffed on others.
#[test]
fn the_results_cross_threads() {
    fn shareable<T: Send + Sync>() {}
    shareable::<Repository>();
    shareable::<ChangeSet>();
    shareable::<FileChange>();
}

const PARSER: &str = "package parser\n\nfunc Parse() int {\n\treturn 1\n}\n\nfunc Lex() {}\n";

#[test]
fn a_commit_is_diffed_against_its_parent_and_names_come_from_its_tree() -> Result<(), Error> {
    let f = Fixture::new();
    f.write("go.mod", b"module example.com/acme\n");
    f.write("parser/parser.go", PARSER.as_bytes());
    f.write("old.txt", b"a\nb\nc\nd\ne\nf\ng\nh\n");
    f.write("gone.rs", b"fn gone() {}\n");
    f.commit("one");
    f.write(
        "parser/parser.go",
        PARSER.replace("return 1", "return 2").as_bytes(),
    );
    std::fs::rename(f.path().join("old.txt"), f.path().join("new.txt")).unwrap();
    std::fs::remove_file(f.path().join("gone.rs")).unwrap();
    f.write("logo.png", &[0x89, b'P', b'N', b'G', 0xff, 0x00, 0xfe]);
    f.commit("two");

    let repo = Repository::open(f.path().join("parser"))?;
    let set = repo.commit("HEAD")?;
    let mut files = statuses(&set);
    files.sort();
    assert_eq!(
        files,
        [
            (FileStatus::Added, "logo.png".to_string()),
            (FileStatus::Deleted, "gone.rs".to_string()),
            (FileStatus::Modified, "parser/parser.go".to_string()),
            (FileStatus::Renamed, "new.txt".to_string()),
        ]
    );
    let mut changed = definitions(&set);
    changed.sort();
    assert_eq!(
        changed,
        [
            (
                "gone.rs".to_string(),
                ChangeKind::Removed,
                "gone".to_string()
            ),
            (
                "parser/parser.go".to_string(),
                ChangeKind::Modified,
                "example.com/acme/parser.Parse".to_string()
            ),
        ]
    );
    let png = set
        .files()
        .iter()
        .find(|f| f.path() == Path::new("logo.png"))
        .unwrap();
    assert!(matches!(png.diff(), Err(Error::NotText { .. })));
    let renamed = set
        .files()
        .iter()
        .find(|f| f.status() == FileStatus::Renamed)
        .unwrap();
    assert_eq!(renamed.before_path(), Some(Path::new("old.txt")));
    assert!(!renamed.diff().expect("text").has_changes());

    // The first commit, against the empty tree, and the same range by name.
    let first = repo.commit("HEAD~1")?;
    assert!(
        first
            .files()
            .iter()
            .all(|f| f.status() == FileStatus::Added)
    );
    assert_eq!(statuses(&repo.range("HEAD~1", "HEAD")?).len(), 4);
    Ok(())
}

#[test]
fn staged_and_unstaged_changes_are_apart() -> Result<(), Error> {
    let f = Fixture::new();
    f.write(
        "src/lib.rs",
        b"pub fn a() -> u8 {\n    1\n}\n\npub fn b() {}\n",
    );
    f.commit("one");
    f.write(
        "src/lib.rs",
        b"pub fn a() -> u8 {\n    2\n}\n\npub fn b() {}\n",
    );
    f.git(&["add", "src/lib.rs"]);
    f.write(
        "src/lib.rs",
        b"pub fn a() -> u8 {\n    2\n}\n\npub fn b() {\n    a();\n}\n",
    );
    f.write("src/new.rs", b"pub fn fresh() {}\n");
    f.write(".gitignore", b"ignored.rs\n");
    f.write("ignored.rs", b"fn hidden() {}\n");

    let repo = Repository::open(f.path())?;
    assert_eq!(
        definitions(&repo.staged()?),
        [(
            "src/lib.rs".to_string(),
            ChangeKind::Modified,
            "crate::a".to_string()
        )]
    );
    assert_eq!(
        definitions(&repo.unstaged()?),
        [
            (
                "src/lib.rs".to_string(),
                ChangeKind::Modified,
                "crate::b".to_string()
            ),
            (
                "src/new.rs".to_string(),
                ChangeKind::Added,
                "crate::new::fresh".to_string()
            ),
        ]
    );
    let unstaged = repo.unstaged()?;
    let paths: Vec<String> = unstaged
        .files()
        .iter()
        .map(|f| f.path().display().to_string())
        .collect();
    assert_eq!(paths, [".gitignore", "src/lib.rs", "src/new.rs"]);
    Ok(())
}

#[test]
fn a_repository_without_commits_stages_against_nothing() -> Result<(), Error> {
    let f = Fixture::new();
    f.write("a.py", b"def f():\n    pass\n");
    f.git(&["add", "a.py"]);
    let repo = Repository::open(f.path())?;
    assert_eq!(
        definitions(&repo.staged()?),
        [("a.py".to_string(), ChangeKind::Added, "a.f".to_string())]
    );
    Ok(())
}

#[test]
fn a_bad_revision_is_an_error() -> Result<(), Error> {
    let f = Fixture::new();
    f.write("a.txt", b"a\n");
    f.commit("one");
    let repo = Repository::open(f.path())?;
    assert!(matches!(repo.commit("no-such-branch"), Err(Error::Git(_))));
    assert!(matches!(
        Repository::open(tempfile::tempdir().unwrap().path()),
        Err(Error::Git(_))
    ));
    Ok(())
}
