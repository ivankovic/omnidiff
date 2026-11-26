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

//! The sizes every language must handle, and the probes that hold it to them: `size_limits check`
//! and `tests/size_limits.rs` both run [`check_template`]. Also compiled into the test with
//! `#[path]`.

use std::path::Path;

use anyhow::Result;

use omnidiff::code::Language;

use super::generate::{Axis, Template, generate, shape};
use super::probe::{Limits, Outcome, Parse, Tools, probe};

/// The sizes `check` holds every language to, one per axis. Each is past a limit some grammar or
/// the engine is known to have had: 20,000 statements is more than 32,768 lines (tree-sitter-yaml's
/// `int16_t` row, issue #114), 300 levels is past tree-sitter-yaml's 1,024-byte scanner state (about
/// 250), and a 40,000-element line is past an `int16_t` column and took the range walk more than a
/// minute while it rebuilt the row for every node on it (`diff::text`'s `RowPair`).
pub const CHECK_SIZES: [(Axis, usize); 4] = [
    (Axis::Lines, 20_000),
    (Axis::Depth, 300),
    (Axis::Width, 40_000),
    (Axis::Token, 100_000),
];

/// The size the generators are checked at: small enough that any `ERROR` node is the generator's.
pub const SMOKE_SIZE: usize = 8;

/// Where a language is known to fall short of [`CHECK_SIZES`]: it is checked at the smaller size
/// given here instead, so the check still catches it getting worse. Raise or drop an entry when
/// the limit is fixed.
///
/// Both entries are one cause: tree-sitter's `get_column` walks back to the start of the line on
/// every call (`ts_lexer__get_column`, tree-sitter 0.25), and these grammars' scanners call it for
/// every token, so a line takes time quadratic in its length to parse.
pub const KNOWN_LIMITS: &[(Language, Axis, usize, &str)] = &[
    (
        Language::Perl,
        Axis::Width,
        10_000,
        "16,000 list elements on a line take 8 s to parse, 0.04 s one element a line; 40,000 more \
         than a minute",
    ),
    (
        Language::Scala,
        Axis::Width,
        10_000,
        "20,000 list elements on a line take 23 s to parse; 40,000 about a minute",
    ),
];

/// The size `check` uses for `language` on `axis`.
pub fn check_size(language: Language, axis: Axis) -> usize {
    KNOWN_LIMITS
        .iter()
        .find(|(l, a, _, _)| *l == language && *a == axis)
        .map(|(_, _, n, _)| *n)
        .or_else(|| {
            CHECK_SIZES
                .iter()
                .find(|(a, _)| *a == axis)
                .map(|(_, n)| *n)
        })
        .expect("every axis has a check size")
}

/// A `check` finding: which language and size, and what went wrong.
pub struct Finding {
    pub language: String,
    pub axis: Axis,
    pub n: usize,
    pub problem: String,
}

/// The `check` probes for one template: the generator's smoke test at [`SMOKE_SIZE`] on every
/// axis, then [`check_size`] on each of `axes`.
pub fn check_template(
    tools: &Tools,
    template: &Template,
    axes: &[Axis],
    seed: u64,
    limits: Limits,
    scratch: &Path,
) -> Result<Vec<Finding>> {
    let name = omnidiff::code::language::stable_name(template.language).to_string();
    let mut findings = Vec::new();
    for &axis in axes {
        let smoke = probe(tools, template, axis, SMOKE_SIZE, seed, limits, scratch)?;
        if !matches!(
            smoke.parse,
            Parse::Tree {
                errors: false,
                covered: true,
                kept: true,
                ..
            }
        ) || smoke.outcome != Outcome::Ok
        {
            let text = generate(template, axis, SMOKE_SIZE, seed, false);
            findings.push(Finding {
                language: name.clone(),
                axis,
                n: SMOKE_SIZE,
                problem: format!(
                    "generator: {} / {} on\n{text}",
                    smoke.parse.summary(),
                    smoke.outcome.name()
                ),
            });
        }
    }
    for &axis in axes {
        let n = check_size(template.language, axis);
        let p = probe(tools, template, axis, n, seed, limits, scratch)?;
        if p.outcome.is_failure() {
            let text = generate(template, axis, n, seed, false);
            findings.push(Finding {
                language: name.clone(),
                axis,
                n,
                problem: format!(
                    "{} ({}; {}; {} ms)",
                    match &p.outcome {
                        Outcome::Crashed(how) => format!("crashed: {how}"),
                        other => other.name().to_string(),
                    },
                    shape(&text),
                    p.parse.summary(),
                    p.elapsed.as_millis()
                ),
            });
        }
    }
    Ok(findings)
}
