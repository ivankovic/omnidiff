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
//! How each language writes a qualified name: its separators, and the part that comes from where
//! the file sits.
//!
//! **Separators.** `::` in Rust, C, C++ and Perl; `.` in Python, Go, Java, Kotlin, Scala, C#,
//! Swift, JavaScript, TypeScript, Lua, R and Vim script. PHP joins namespaces with `\` and a
//! class to its members with `::` (`Acme\Parser::parse`); Ruby joins modules and classes with
//! `::`, an instance method with `#` and a singleton method with `.` (`Acme::Parser#parse`,
//! `Acme::Parser.make`), as Ruby's documentation does.
//!
//! **The file's own part.** Java, Kotlin, Scala, C#, PHP, Perl and C++ say in the file which
//! package or namespace it is in, so `definitions` reads that from the tree. Rust, Python and Go
//! do not: a Rust module path follows the file's place under `src/`, a Python module the
//! directories with an `__init__.py` above it, a Go import path the module in the nearest
//! `go.mod`. [`file_prefix`] works those out from the side's path and a [`ProjectFiles`], so a
//! diff of two files on disk and a diff of two blobs in a git tree name the same definition the
//! same way. Without a path there is no such part, and names are relative to the file. JavaScript
//! and TypeScript modules are named by their file path, not a dotted name, so they get none
//! either; C, Bash, Lua, R, Vim script and SQL have no namespaces.
//!
//! Rust paths start at `crate` (`crate::parser::Parser::parse`), the way code inside the crate
//! names them: a crate's name is in `Cargo.toml`, not in its paths, and binaries, tests and
//! examples are crates of their own.

use std::path::{Component, Path};

use super::definitions::{Definition, DefinitionKind, Found};
use crate::code::Language;

/// The files around the one being diffed, for [`file_prefix`]: the filesystem, or a git tree.
/// Paths are the ones given to [`Diff::with_paths`](super::Diff::with_paths), relative or not.
pub(crate) trait ProjectFiles {
    fn is_file(&self, path: &Path) -> bool;
    fn read(&self, path: &Path) -> Option<String>;
}

pub(crate) struct FileSystem;

impl ProjectFiles for FileSystem {
    fn is_file(&self, path: &Path) -> bool {
        path.is_file()
    }

    fn read(&self, path: &Path) -> Option<String> {
        std::fs::read_to_string(path).ok()
    }
}

/// The qualified-name prefix a file's location gives its definitions, if its language has one.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct FilePrefix(Option<String>);

pub(crate) fn file_prefix(
    path: Option<&Path>,
    language: Language,
    _contents: &str,
    files: &dyn ProjectFiles,
) -> FilePrefix {
    let Some(path) = path else {
        return FilePrefix(None);
    };
    FilePrefix(match language {
        Language::Rust => rust_module_path(path, files),
        Language::Python => python_module(path, files),
        Language::Go => go_import_path(path, files),
        _ => None,
    })
}

/// The prefix a top-level definition starts with: the file's location's, or a package or
/// namespace statement's in the file. Go has both, and its import path wins over the bare
/// package name, as `go doc` writes it.
pub(crate) fn join_file_prefix(
    prefix: &FilePrefix,
    statement: Option<&str>,
    language: Language,
) -> String {
    let from_path = prefix.0.as_deref();
    match language {
        Language::Go => from_path.or(statement),
        _ => statement.or(from_path),
    }
    .unwrap_or_default()
    .to_string()
}

pub(crate) fn join(base: &str, separator: &str, segment: &str) -> String {
    if base.is_empty() {
        segment.to_string()
    } else {
        format!("{base}{separator}{segment}")
    }
}

/// Between a scope and a type in it.
pub(crate) fn scope_separator(language: Language) -> &'static str {
    match language {
        Language::Rust | Language::C | Language::CPP | Language::Perl | Language::Ruby => "::",
        Language::PHP => "\\",
        _ => ".",
    }
}

/// Between a parent of `parent` kind (`None` for the file's own prefix) and a `kind` child.
pub(crate) fn separator(
    language: Language,
    parent: Option<DefinitionKind>,
    kind: DefinitionKind,
    found: &Found,
) -> &'static str {
    match language {
        Language::PHP => match parent {
            None | Some(DefinitionKind::Module) => "\\",
            Some(_) => "::",
        },
        Language::Ruby if kind.is_function() => {
            if found.singleton {
                "."
            } else {
                "#"
            }
        }
        _ => scope_separator(language),
    }
}

/// Appends the parameter types to every function that shares its qualified name with another
/// in the same scope, in the languages that allow overloading: `com.acme.Parser.parse(int)`.
pub(crate) fn name_overloads(definitions: &mut [Definition], language: Language) {
    if !matches!(
        language,
        Language::Java
            | Language::Kotlin
            | Language::Scala
            | Language::CSharp
            | Language::CPP
            | Language::Swift
    ) {
        return;
    }
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for d in definitions.iter().filter(|d| d.kind().is_function()) {
        *counts.entry(d.qualified_name().to_string()).or_default() += 1;
    }
    for d in definitions.iter_mut() {
        if d.kind().is_function() && counts.get(d.qualified_name()).is_some_and(|&n| n > 1) {
            d.append_overload_key();
        }
    }
}

/// The directory holding `path`, and its ancestors, nearest first.
fn directories(path: &Path) -> impl Iterator<Item = &Path> {
    path.parent().into_iter().flat_map(Path::ancestors)
}

/// `crate::parser::lexer` for `src/parser/lexer.rs` of a crate: the part after the last `src`
/// directory. A file outside `src/` (`tests/`, `examples/`, `benches/`, `build.rs`) is a crate
/// root of its own, when the nearest `Cargo.toml` above it says it is in a crate at all.
fn rust_module_path(path: &Path, files: &dyn ProjectFiles) -> Option<String> {
    let parts: Vec<String> = path
        .components()
        .filter_map(|c| match c {
            Component::Normal(part) => Some(part.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect();
    let mut segments = vec!["crate".to_string()];
    let Some(src) = parts.iter().rposition(|p| p == "src") else {
        // Not under `src/`: a crate root, if it is in a crate.
        let in_crate = directories(path)
            .filter(|dir| !dir.as_os_str().is_empty())
            .any(|dir| files.is_file(&dir.join("Cargo.toml")));
        return in_crate.then(|| segments.join("::"));
    };
    // `src/lib.rs`, `src/main.rs` and every binary under `src/bin/` are crate roots.
    match &parts[src + 1..] {
        [file] if file == "lib.rs" || file == "main.rs" => {}
        [bin, ..] if bin == "bin" => {}
        [dirs @ .., file] => {
            segments.extend(dirs.iter().cloned());
            let stem = file.strip_suffix(".rs").unwrap_or(file);
            if stem != "mod" {
                segments.push(stem.to_string());
            }
        }
        [] => {}
    }
    Some(segments.join("::"))
}

/// `pkg.sub.module` for `pkg/sub/module.py` where `pkg/` and `pkg/sub/` hold an `__init__.py`;
/// the module's own name when no directory above it is a package.
fn python_module(path: &Path, files: &dyn ProjectFiles) -> Option<String> {
    let stem = path.file_stem()?.to_string_lossy().into_owned();
    let mut parts: Vec<String> = directories(path)
        .take_while(|dir| !dir.as_os_str().is_empty() && files.is_file(&dir.join("__init__.py")))
        .filter_map(|dir| Some(dir.file_name()?.to_string_lossy().into_owned()))
        .collect();
    parts.reverse();
    if stem != "__init__" {
        parts.push(stem);
    }
    (!parts.is_empty()).then(|| parts.join("."))
}

/// `example.com/acme/parser` for `parser/x.go` in the module `example.com/acme`: the nearest
/// `go.mod`'s `module` line and the directory under it.
fn go_import_path(path: &Path, files: &dyn ProjectFiles) -> Option<String> {
    let dir = path.parent()?;
    let root = directories(path).find(|d| files.is_file(&d.join("go.mod")))?;
    let module = files.read(&root.join("go.mod"))?.lines().find_map(|line| {
        let rest = line.trim().strip_prefix("module")?;
        rest.starts_with([' ', '\t'])
            .then(|| rest.trim().trim_matches('"').to_string())
    })?;
    let relative = dir.strip_prefix(root).ok()?;
    let tail: Vec<String> = relative
        .components()
        .filter_map(|c| match c {
            Component::Normal(part) => Some(part.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect();
    Some(if tail.is_empty() {
        module
    } else {
        format!("{module}/{}", tail.join("/"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::path::PathBuf;

    /// A project held in memory: path -> contents.
    struct Files(HashMap<PathBuf, String>);

    impl Files {
        fn of(entries: &[(&str, &str)]) -> Self {
            Self(
                entries
                    .iter()
                    .map(|(p, c)| (PathBuf::from(p), c.to_string()))
                    .collect(),
            )
        }
    }

    impl ProjectFiles for Files {
        fn is_file(&self, path: &Path) -> bool {
            self.0.contains_key(path)
        }

        fn read(&self, path: &Path) -> Option<String> {
            self.0.get(path).cloned()
        }
    }

    #[test]
    fn rust_paths_follow_the_file_under_src() {
        let files = Files::of(&[("proj/Cargo.toml", "")]);
        let path = |p: &str| rust_module_path(Path::new(p), &files);
        assert_eq!(path("proj/src/lib.rs").as_deref(), Some("crate"));
        assert_eq!(path("proj/src/main.rs").as_deref(), Some("crate"));
        assert_eq!(path("proj/src/parser.rs").as_deref(), Some("crate::parser"));
        assert_eq!(
            path("proj/src/parser/mod.rs").as_deref(),
            Some("crate::parser")
        );
        assert_eq!(
            path("proj/src/parser/lexer.rs").as_deref(),
            Some("crate::parser::lexer")
        );
        assert_eq!(path("proj/src/bin/tool.rs").as_deref(), Some("crate"));
        assert_eq!(path("proj/tests/api.rs").as_deref(), Some("crate"));
    }

    #[test]
    fn a_rust_path_not_on_disk_is_placed_by_its_src_directory() {
        let files = Files::of(&[]);
        assert_eq!(
            rust_module_path(Path::new("a/src/diff/text.rs"), &files).as_deref(),
            Some("crate::diff::text")
        );
        assert_eq!(rust_module_path(Path::new("text.rs"), &files), None);
    }

    #[test]
    fn python_modules_climb_the_packages() {
        let files = Files::of(&[
            ("root/pkg/__init__.py", ""),
            ("root/pkg/sub/__init__.py", ""),
        ]);
        let module = |p: &str| python_module(Path::new(p), &files);
        assert_eq!(
            module("root/pkg/sub/parser.py").as_deref(),
            Some("pkg.sub.parser")
        );
        assert_eq!(module("root/pkg/__init__.py").as_deref(), Some("pkg"));
        assert_eq!(module("root/script.py").as_deref(), Some("script"));
    }

    #[test]
    fn go_import_paths_come_from_go_mod() {
        let files = Files::of(&[("m/go.mod", "// c\nmodule example.com/acme\n\ngo 1.22\n")]);
        let import = |p: &str| go_import_path(Path::new(p), &files);
        assert_eq!(
            import("m/parser/x.go").as_deref(),
            Some("example.com/acme/parser")
        );
        assert_eq!(import("m/main.go").as_deref(), Some("example.com/acme"));
        assert_eq!(import("elsewhere/x.go"), None);
    }
}
