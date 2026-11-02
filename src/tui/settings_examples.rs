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
//! The settings page's examples: one small before/after pair per language, written for this
//! purpose (never sampled from a repository, for licensing), so that each highlighting option
//! visibly changes something in it. The page shows the example in the language of the open diff,
//! Rust when nothing is open, and plain text for a language without a grammar.
//!
//! Each example lives in `settings_examples/<directory>/before.<ext>.example` and
//! `after.<ext>.example`: real files, so they read and edit as code, with a suffix that keeps
//! every tool from taking them for sources of this crate. The tests hold every example to two
//! promises: it parses without an error, and each option changes how it is highlighted - toggled
//! from the `Minimal` preset or from the `Full` one - unless [`SettingsExample::no_effect`] says
//! why the option cannot show in that language.

use crate::code::Language;
use crate::diff::text::{RangeMatch, RenderOptions, TextOperation, ranges_for_options};
use crate::diff::text_range::{SourceColumn, SourceRow, SourceText};

/// One language's example pair.
#[derive(Debug, Clone, Copy)]
pub struct SettingsExample {
    pub language: Language,
    /// A file name whose extension names the language, for the diff's language detection.
    pub file_name: &'static str,
    pub before: &'static str,
    pub after: &'static str,
    /// Options (by [`RenderOptions::options`] index) that cannot change anything in this
    /// language, each with the reason the page shows instead of an effect.
    pub no_effect: &'static [(usize, &'static str)],
}

macro_rules! example {
    ($directory:literal, $extension:literal, $language:expr, $no_effect:expr) => {
        SettingsExample {
            language: $language,
            file_name: concat!("example.", $extension),
            before: include_str!(concat!(
                "settings_examples/",
                $directory,
                "/before.",
                $extension,
                ".example"
            )),
            after: include_str!(concat!(
                "settings_examples/",
                $directory,
                "/after.",
                $extension,
                ".example"
            )),
            no_effect: $no_effect,
        }
    };
}

/// Every example, one per language with a grammar, then plain text.
pub const EXAMPLES: &[SettingsExample] = &[
    example!("rust", "rs", Language::Rust, &[]),
    example!("vim", "vim", Language::Vimscript, &[]),
    example!("css", "css", Language::CSS, &[]),
    example!("html", "html", Language::HTML, MARKUP_NO_EFFECT),
    example!("xml", "xml", Language::XML, MARKUP_NO_EFFECT),
    example!("json", "json", Language::JSON, &[(6, JSON_NO_NAMES)]),
    example!("yaml", "yaml", Language::YAML, &[]),
    example!("text", "txt", Language::Unknown, PLAIN_TEXT_NO_EFFECT),
];

/// What plain text cannot show. Without a grammar a file is compared line by line, and only the
/// indentation of whole added and removed lines is up to the options.
const PLAIN_TEXT_NO_EFFECT: &[(usize, &str)] = &[
    (1, NO_SYNTAX),
    (2, NO_SYNTAX),
    (3, NO_SYNTAX),
    (4, NO_SYNTAX),
    (5, NO_SYNTAX),
    (6, NO_SYNTAX),
    (7, NO_SYNTAX),
];

/// Why an option has nothing to show in a file without a grammar.
const NO_SYNTAX: &str = "Text without a grammar is compared line by line, with no names, brackets \
    or moves to tell apart, so this setting changes nothing in it.";

/// What HTML and XML cannot show: their structure is tags and attributes, not punctuation, and no
/// edit we tried had the two sides measure a moved tag or attribute differently.
const MARKUP_NO_EFFECT: &[(usize, &str)] = &[
    (
        1,
        "Markup is built from angle brackets, quotes and equals signs, not the brackets, \
        commas, colons and semicolons this setting covers, so it has nothing to show here.",
    ),
    (
        5,
        "Both sides measure a moved tag or attribute the same way, so there is no second, \
        larger account of a move to show.",
    ),
];

/// Why JSON has no renamed name to show: every key is a string.
const JSON_NO_NAMES: &str = "JSON has no names, only strings: a changed key or value shows the \
    letters that differ whichever way this is set.";

/// The example in `language`: its own, plain text for a language without one, Rust for none.
pub fn example_for(language: Option<Language>) -> &'static SettingsExample {
    let wanted = language.unwrap_or(Language::Rust);
    EXAMPLES
        .iter()
        .find(|example| example.language == wanted)
        .or_else(|| {
            EXAMPLES
                .iter()
                .find(|example| example.language == Language::Unknown)
        })
        .expect("the plain-text example is always there")
}

/// Each side's highlight per byte under `options`: the operation painting it, or `None`.
pub type Painted = [Vec<Option<TextOperation>>; 2];

/// How `example` is highlighted under `options`, byte by byte, with the text it was built from.
pub fn painted(
    example: &SettingsExample,
    options: RenderOptions,
) -> Option<(Painted, [String; 2])> {
    let path = std::path::Path::new(example.file_name);
    let (data, _) =
        crate::tui::app::compute_diff_of_texts(path, example.before, path, example.after, options)
            .ok()?;
    let before = ranges_for_options(&data.before_ranges, &data.before_contents, options);
    let after = ranges_for_options(&data.after_ranges, &data.after_contents, options);
    Some((
        [
            labels(&data.before_contents, &before),
            labels(&data.after_contents, &after),
        ],
        [data.before_contents, data.after_contents],
    ))
}

/// `ranges` as one label per byte of `text`; unpainted and `Identical` bytes are `None`.
fn labels(text: &str, ranges: &[RangeMatch]) -> Vec<Option<TextOperation>> {
    let source = SourceText::new(text);
    let mut labels = vec![None; text.len()];
    for range in ranges {
        if matches!(
            range.operation,
            TextOperation::Identical | TextOperation::NotYetSet
        ) {
            continue;
        }
        let at = |row: usize, column: usize| {
            source
                .byte_index(SourceRow::from_raw(row), SourceColumn::from_raw(column))
                .map(|offset| offset.get())
        };
        let (Some(start), Some(end)) = (
            at(range.source.start_row, range.source.start_column),
            at(range.source.end_row, range.source.end_column),
        ) else {
            continue;
        };
        for label in &mut labels[start.min(text.len())..end.min(text.len())] {
            *label = Some(range.operation.clone());
        }
    }
    labels
}

#[cfg(test)]
mod tests;
