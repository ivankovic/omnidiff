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

//! Every language with a grammar holds up at `size_limits check`'s sizes: more than 32,768 lines,
//! 300 levels of nesting, a 40,000-element line and a 100,000-character token, each with one change
//! the diff must report (`src/bin/size_limits/check.rs`). The real `omnidiff` binary runs each pair
//! in a child process under a time and memory cap, so a grammar that aborts, a stack overflow or a
//! runaway allocation fails one test instead of the whole run. One test per language, so nextest
//! runs them side by side.
#![cfg(unix)]

// The binary's modules; what only its `search` and `parse` use goes unused here.
#[path = "../src/bin/size_limits/check.rs"]
#[allow(dead_code)]
mod check;
#[path = "../src/bin/size_limits/generate.rs"]
#[allow(dead_code)]
mod generate;
#[path = "../src/bin/size_limits/probe.rs"]
#[allow(dead_code)]
mod probe;

use std::path::Path;
use std::time::Duration;

use omnidiff::code::Language;

/// Generous next to the 1-2 s a language takes, for a loaded CI runner; a debug-built `ci` profile
/// is optimised (see Cargo.toml), so its times are the release build's.
const LIMITS: probe::Limits = probe::Limits {
    timeout: Duration::from_secs(120),
    memory: 6 << 30,
};

fn check(language: Language) {
    let template = generate::templates()
        .into_iter()
        .find(|t| t.language == language)
        .expect("a template for every language tested");
    let tools = probe::Tools {
        omnidiff: Path::new(env!("CARGO_BIN_EXE_omnidiff")),
        parser: Path::new(env!("CARGO_BIN_EXE_size_limits")),
    };
    let scratch = tempfile::tempdir().unwrap();
    let findings = check::check_template(
        &tools,
        &template,
        &generate::Axis::ALL,
        1,
        LIMITS,
        scratch.path(),
    )
    .expect("the probes ran");
    let report: Vec<String> = findings
        .iter()
        .map(|f| format!("{} {} n={}: {}", f.language, f.axis.name(), f.n, f.problem))
        .collect();
    assert!(report.is_empty(), "{}", report.join("\n"));
}

macro_rules! languages {
    ($($test:ident => $language:ident),* $(,)?) => {
        $(
            #[test]
            fn $test() {
                check(Language::$language);
            }
        )*

        /// A grammar added without a test here would go unchecked.
        #[test]
        fn every_template_is_tested() {
            let tested = [$(Language::$language),*];
            for template in generate::templates() {
                assert!(
                    tested.contains(&template.language),
                    "no size_limits test for {:?}",
                    template.language
                );
            }
            // And every language with a grammar has a template.
            use strum::IntoEnumIterator;
            for language in Language::iter() {
                if omnidiff::code::language::to_treesitter(&language).is_some() {
                    assert!(tested.contains(&language), "no size_limits template for {language:?}");
                }
            }
        }
    };
}

languages! {
    c => C,
    cpp => CPP,
    css => CSS,
    csharp => CSharp,
    go => Go,
    html => HTML,
    json => JSON,
    java => Java,
    javascript => JavaScript,
    kotlin => Kotlin,
    lua => LUA,
    php => PHP,
    perl => Perl,
    python => Python,
    r => R,
    ruby => Ruby,
    rust => Rust,
    scala => Scala,
    sql => SQL,
    shell => ShellScript,
    swift => Swift,
    tsx => TSX,
    typescript => TypeScript,
    vimscript => Vimscript,
    vue => Vue,
    yaml => YAML,
    xml => XML,
}
