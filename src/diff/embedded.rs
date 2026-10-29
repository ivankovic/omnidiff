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
//! Code of one language held in a string of another: TSX in a JSON `"content"`, shell in a YAML
//! `run: |`, CSS in a `styled.div` template. Diffed as the host's string, a three-line edit of the
//! embedded file is one leaf whose text changed somewhere in 3 KB, painted as a single changed
//! middle from the first difference to the last; and when the host escapes newlines (`\n` in JSON)
//! it is all one line, with no layout for the whitespace rules to read.
//!
//! **Each embedded pair is diffed as a file of its own.** After the host diff, [`solve`] looks at
//! every matched pair of host strings that changed. Where [`decode`] can recover the embedded text
//! and [`detect`] can name its language from the string's context, both sides are parsed as that
//! language, and if both parse without an error the pair is diffed by the whole pipeline
//! ([`super::Diff::from_code`]) and kept as an [`EmbeddedDiff`] in [`super::ASTDiff::embedded`].
//! The host mapping is left as it is: the embedded diff is a layer under it, not a replacement.
//!
//! - **Separate trees, not spliced ones.** Every per-node structure is keyed by tree-sitter node id,
//!   one tree per side, so the embedded code gets its own [`Code`] with its own tree and metadata.
//!   Nodes are never matched across layers: code moved out of a string into the host is a delete
//!   in one layer and an insert in the other.
//! - **Decoded text, with a byte map back.** The embedded code is the text the host *means*:
//!   escapes resolved (`\n` is a newline, `\"` a quote), a YAML block's indentation removed. The
//!   decoded text is what gets parsed, diffed, painted by a human and graded, so every rule about
//!   rows and whitespace applies to it unchanged. [`Decoded::offsets`] maps each decoded byte to
//!   where it came from in the host file, which is how [`super::text`] paints the host file from
//!   the embedded diff.
//! - **Interpolations become filler.** A template substitution (`${props.size}`) or a GitHub
//!   expression (`${{ matrix.os }}`) is not code of the embedded language. Each byte of one becomes
//!   `_`, the same length, so it parses as a word and maps back one byte to one byte.
//! - **Context first, and a clean parse as the gate.** The language comes from where the string
//!   sits ([`detect`]): a sibling `"path"` naming a file, a `run:` key, a `css` tag. Content alone is
//!   consulted only for markup and for C preprocessor text (a shell `'...'` string of `#include`
//!   lines). A detection that is wrong almost never survives the gate, since both sides must parse
//!   without an error.
//! - **Text when nothing names a language.** A changed multi-line string whose language is not
//!   detected, or does not parse cleanly, is still a layer: plain text ([`Language::Unknown`], no
//!   tree), diffed line by line as a file without a grammar is
//!   ([`super::text::plain_text_line_diff`]). An awk program in a shell `'...'` string then paints
//!   its deleted lines one by one, not one span from its first change to its last. A one-line
//!   string gains nothing from this and is left to the host.
//! - **Only changed pairs.** A string inserted or deleted whole is painted whole either way, so it
//!   gets no embedded layer.
use tree_sitter::Node;

use crate::code::{Code, Language};
use crate::diff::{ASTDiff, ASTMappingOperation, PassCtx};

/// One embedded pair: the host string nodes it lives in, both sides' decoded code, and their diff.
#[derive(Debug, Clone)]
pub struct EmbeddedDiff {
    /// The host string node in the before tree.
    pub before_id: usize,
    /// The host string node in the after tree.
    pub after_id: usize,
    pub language: Language,
    pub before: Code,
    pub after: Code,
    /// [`Decoded::offsets`] of `before`, into the host before file.
    pub before_offsets: Vec<usize>,
    /// [`Decoded::offsets`] of `after`, into the host after file.
    pub after_offsets: Vec<usize>,
    pub diff: ASTDiff,
}

/// The text a host string holds, and where each of its bytes came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decoded {
    pub text: String,
    /// `text.len() + 1` absolute byte offsets into the host file: entry `i` is where the host's
    /// spelling of decoded byte `i` starts, and the last entry is where the string's content ends.
    /// Every byte of a character an escape produced maps to the escape's start. A range of decoded
    /// bytes `a..b` is therefore host bytes `offsets[a]..offsets[b]`.
    pub offsets: Vec<usize>,
}

impl Decoded {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            text: String::with_capacity(capacity),
            offsets: Vec::with_capacity(capacity + 1),
        }
    }

    /// Appends `text`, every byte of it spelled at host offset `at`.
    fn push_from(&mut self, text: &str, at: usize) {
        self.text.push_str(text);
        self.offsets.extend(std::iter::repeat_n(at, text.len()));
    }

    /// Appends host bytes `from..to` of `source` verbatim, one to one.
    fn push_verbatim(&mut self, source: &str, from: usize, to: usize) {
        self.text.push_str(&source[from..to]);
        self.offsets.extend(from..to);
    }

    /// Appends `to - from` bytes of `_` for host bytes `from..to`, one to one.
    fn push_filler(&mut self, from: usize, to: usize) {
        self.text.extend(std::iter::repeat_n('_', to - from));
        self.offsets.extend(from..to);
    }

    fn finish(mut self, end: usize) -> Self {
        self.offsets.push(end);
        self
    }
}

/// Keys whose YAML value is a shell command: CI steps (`run`), GitLab jobs (`script`), Compose and
/// Kubernetes (`command`, `entrypoint`).
const COMMAND_KEYS: &[&str] = &[
    "run",
    "script",
    "before_script",
    "after_script",
    "command",
    "entrypoint",
    "cmd",
];

/// JSON keys whose string value names the file a sibling `"content"` holds (shadcn's registry,
/// VS Code snippets, notebook-style bundles).
const PATH_KEYS: &[&str] = &["path", "file", "filename", "fileName", "name", "target"];

/// Tags of a JavaScript tagged template whose body is CSS (styled-components, Emotion, Lit).
fn is_css_tag(tag: &str) -> bool {
    matches!(
        tag,
        "css" | "keyframes" | "createGlobalStyle" | "injectGlobal" | "global"
    ) || tag.starts_with("styled.")
        || tag.starts_with("styled(")
}

/// Tags whose body is HTML (Lit, htm, common `html` helpers).
fn is_html_tag(tag: &str) -> bool {
    matches!(tag, "html" | "svg")
}

/// Matches every changed pair of host strings that holds embedded code; see the module doc. Runs
/// after the host pipeline, which it does not change.
pub(crate) fn solve(ctx: &PassCtx, ast_diff: &mut ASTDiff) {
    let host = ctx.language();
    if !hosts_embedded_code(host) {
        return;
    }
    let Some(before_tree) = ctx.before.ast.as_ref() else {
        return;
    };
    let mut found = Vec::new();
    let mut stack = vec![before_tree.root_node()];
    while let Some(node) = stack.pop() {
        if let Some(embedded) = pair_at(ctx, ast_diff, host, node) {
            // The outermost host string wins: whatever is inside it is the embedded layer's.
            found.push(embedded);
            continue;
        }
        let mut cursor = node.walk();
        let children: Vec<_> = node.children(&mut cursor).collect();
        stack.extend(children.into_iter().rev());
    }
    ast_diff.embedded = found;
}

/// The embedded diff for `before_node` and its partner, if they are a changed host string pair
/// holding the same detectable, cleanly parsing language.
fn pair_at(
    ctx: &PassCtx,
    ast_diff: &ASTDiff,
    host: Language,
    before_node: Node,
) -> Option<EmbeddedDiff> {
    if !is_host_string(host, before_node) {
        return None;
    }
    let after_id = *ast_diff.before_node_map.get(&before_node.id())?;
    if after_id == 0 {
        return None;
    }
    let mapping = ast_diff.mapping.get(&(before_node.id(), after_id))?;
    if mapping.operation == ASTMappingOperation::Identical {
        return None;
    }
    let after_node = *ctx.node_cache.after.get(&after_id)?;
    if !is_host_string(host, after_node) {
        return None;
    }
    let at = |language| {
        layer(
            ctx.before,
            ctx.after,
            before_node,
            after_node,
            host,
            language,
        )
    };
    let mut embedded = detect(host, before_node, &ctx.before.contents)
        .or_else(|| detect(host, after_node, &ctx.after.contents))
        .and_then(at)
        .or_else(|| at(Language::Unknown))?;
    if embedded.before.contents == embedded.after.contents {
        return None;
    }
    if embedded.language != Language::Unknown {
        embedded.diff = super::Diff::from_code(&embedded.before, &embedded.after).ast?;
    }
    Some(embedded)
}

/// Both sides of the embedded layer at a host pair, parsed as `language`, with an empty diff;
/// `None` if either side does not decode or does not parse cleanly. [`Language::Unknown`] is a
/// plain-text layer: never parsed, and only for a string of several lines. What [`solve`] diffs,
/// and what a human mapping's embedded layer is resolved against, so the two always see the same
/// text.
pub fn layer(
    before: &Code,
    after: &Code,
    before_node: Node,
    after_node: Node,
    host: Language,
    language: Language,
) -> Option<EmbeddedDiff> {
    let before_decoded = decode(host, before_node, &before.contents)?;
    let after_decoded = decode(host, after_node, &after.contents)?;
    let (before_code, after_code) = if language == Language::Unknown {
        let lines = |text: &str| text.trim_end_matches('\n').contains('\n');
        if !lines(&before_decoded.text) && !lines(&after_decoded.text) {
            return None;
        }
        (
            Code::from_string(&before_decoded.text, &language),
            Code::from_string(&after_decoded.text, &language),
        )
    } else {
        (
            parsed_cleanly(&before_decoded.text, language)?,
            parsed_cleanly(&after_decoded.text, language)?,
        )
    };
    Some(EmbeddedDiff {
        before_id: before_node.id(),
        after_id: after_node.id(),
        language,
        before: before_code,
        after: after_code,
        before_offsets: before_decoded.offsets,
        after_offsets: after_decoded.offsets,
        diff: ASTDiff::default(),
    })
}

/// `text` parsed as `language`, or `None` if the parse has an error or the text is blank.
fn parsed_cleanly(text: &str, language: Language) -> Option<Code> {
    if text.trim().is_empty() {
        return None;
    }
    let code = Code::from_string(text, &language);
    let ast = code.ast.as_ref()?;
    if ast.root_node().has_error() {
        return None;
    }
    Some(code)
}

/// Whether [`is_host_string`] knows any string kind of `host`.
fn hosts_embedded_code(host: Language) -> bool {
    matches!(
        host,
        Language::JSON
            | Language::YAML
            | Language::JavaScript
            | Language::TypeScript
            | Language::TSX
            | Language::ShellScript
            | Language::Python
            | Language::CSharp
    )
}

/// Whether `node` is a string [`decode`] can read in `host`.
pub fn is_host_string(host: Language, node: Node) -> bool {
    match host {
        Language::JSON => node.kind() == "string",
        Language::YAML => node.kind() == "block_scalar",
        Language::JavaScript | Language::TypeScript | Language::TSX => {
            node.kind() == "template_string"
        }
        Language::ShellScript => node.kind() == "raw_string",
        Language::Python => node.kind() == "string",
        Language::CSharp => node.kind() == "raw_string_literal",
        _ => false,
    }
}

/// The text the host string `node` holds, with its byte map; `None` for a node that is not one of
/// `host`'s strings, or whose spelling is malformed.
pub fn decode(host: Language, node: Node, source: &str) -> Option<Decoded> {
    if !is_host_string(host, node) {
        return None;
    }
    match host {
        Language::JSON => decode_json_string(node, source),
        Language::YAML => decode_yaml_block_scalar(node, source),
        Language::ShellScript => decode_shell_raw_string(node, source),
        Language::Python => decode_python_string(node, source),
        Language::CSharp => decode_csharp_raw_string(node, source),
        _ => decode_template_string(node, source),
    }
}

/// A JSON string between its quotes, with RFC 8259's escapes resolved.
fn decode_json_string(node: Node, source: &str) -> Option<Decoded> {
    let (start, end) = (node.start_byte(), node.end_byte());
    let raw = source.get(start..end)?;
    if raw.len() < 2 || !raw.starts_with('"') || !raw.ends_with('"') {
        return None;
    }
    let (from, to) = (start + 1, end - 1);
    let bytes = source.as_bytes();
    let mut decoded = Decoded::with_capacity(to - from);
    let mut i = from;
    while i < to {
        if bytes[i] != b'\\' {
            let run_end = (i..to).find(|&j| bytes[j] == b'\\').unwrap_or(to);
            decoded.push_verbatim(source, i, run_end);
            i = run_end;
            continue;
        }
        let escape = *bytes.get(i + 1)?;
        let (character, length) = match escape {
            b'"' => ('"', 2),
            b'\\' => ('\\', 2),
            b'/' => ('/', 2),
            b'b' => ('\u{8}', 2),
            b'f' => ('\u{c}', 2),
            b'n' => ('\n', 2),
            b'r' => ('\r', 2),
            b't' => ('\t', 2),
            b'u' => json_unicode_escape(source, i, to)?,
            _ => return None,
        };
        let mut buffer = [0u8; 4];
        decoded.push_from(character.encode_utf8(&mut buffer), i);
        i += length;
    }
    Some(decoded.finish(to))
}

/// The character a `\uXXXX` escape at `at` spells, a surrogate pair's two escapes taken together,
/// and how many bytes it spans.
fn json_unicode_escape(source: &str, at: usize, end: usize) -> Option<(char, usize)> {
    let unit = |offset: usize| -> Option<u32> {
        let digits = source.get(offset + 2..offset + 6)?;
        if offset + 6 > end || !source[offset..].starts_with("\\u") {
            return None;
        }
        u32::from_str_radix(digits, 16).ok()
    };
    let high = unit(at)?;
    if (0xD800..0xDC00).contains(&high) {
        let low = unit(at + 6)?;
        if !(0xDC00..0xE000).contains(&low) {
            return None;
        }
        let code = 0x10000 + ((high - 0xD800) << 10) + (low - 0xDC00);
        return Some((char::from_u32(code)?, 12));
    }
    Some((char::from_u32(high)?, 6))
}

/// A YAML block scalar's lines (`|` or `>`, after its header line) with their common indentation
/// removed, and every `${{ ... }}` GitHub expression made filler. A folded (`>`) block is kept line
/// for line rather than folded: the embedded code is the text as written, which is what a reader
/// sees and paints.
fn decode_yaml_block_scalar(node: Node, source: &str) -> Option<Decoded> {
    let (start, end) = (node.start_byte(), node.end_byte());
    let raw = source.get(start..end)?;
    let header_end = start + raw.find('\n')? + 1;
    let body = &source[header_end..end];
    let indent = body
        .split('\n')
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start_matches(' ').len())
        .min()?;
    let mut decoded = Decoded::with_capacity(body.len());
    let mut line_start = header_end;
    while line_start < end {
        let line_end = source[line_start..end]
            .find('\n')
            .map_or(end, |at| line_start + at);
        let line = &source[line_start..line_end];
        let content_start =
            line_start + indent.min(line.len() - line.trim_start_matches(' ').len());
        push_with_expressions_as_filler(&mut decoded, source, content_start, line_end);
        if line_end < end {
            decoded.push_verbatim(source, line_end, line_end + 1);
        }
        line_start = line_end + 1;
    }
    // The block's last newline belongs to the YAML, not to the code.
    let content_end = if decoded.text.ends_with('\n') {
        decoded.text.pop();
        decoded.offsets.pop().unwrap_or(end)
    } else {
        end
    };
    Some(decoded.finish(content_end))
}

/// Host bytes `from..to` verbatim, except that each `${{ ... }}` is filler.
fn push_with_expressions_as_filler(decoded: &mut Decoded, source: &str, from: usize, to: usize) {
    let mut i = from;
    while i < to {
        let Some(open) = source[i..to].find("${{").map(|at| i + at) else {
            decoded.push_verbatim(source, i, to);
            return;
        };
        let Some(close) = source[open..to].find("}}").map(|at| open + at + 2) else {
            decoded.push_verbatim(source, i, to);
            return;
        };
        decoded.push_verbatim(source, i, open);
        decoded.push_filler(open, close);
        i = close;
    }
}

/// A template literal between its backticks, its escapes resolved and every `${...}` substitution
/// made filler.
fn decode_template_string(node: Node, source: &str) -> Option<Decoded> {
    let (start, end) = (node.start_byte(), node.end_byte());
    let raw = source.get(start..end)?;
    if raw.len() < 2 || !raw.starts_with('`') || !raw.ends_with('`') {
        return None;
    }
    let (from, to) = (start + 1, end - 1);
    let mut decoded = Decoded::with_capacity(to - from);
    let mut i = from;
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        let (child_start, child_end) = (child.start_byte(), child.end_byte());
        if child_start < i || child_end > to {
            continue;
        }
        match child.kind() {
            "template_substitution" => {
                decoded.push_verbatim(source, i, child_start);
                decoded.push_filler(child_start, child_end);
            }
            "escape_sequence" => {
                decoded.push_verbatim(source, i, child_start);
                let spelled = &source[child_start..child_end];
                let character = match spelled.as_bytes().get(1) {
                    Some(b'n') => '\n',
                    Some(b't') => '\t',
                    Some(b'r') => '\r',
                    Some(b'`') => '`',
                    Some(b'$') => '$',
                    Some(b'\\') => '\\',
                    Some(b'\'') => '\'',
                    Some(b'"') => '"',
                    // `\u{...}`, `\x..` and the rest are rare in embedded code: kept as spelled.
                    _ => {
                        decoded.push_verbatim(source, child_start, child_end);
                        i = child_end;
                        continue;
                    }
                };
                let mut buffer = [0u8; 4];
                decoded.push_from(character.encode_utf8(&mut buffer), child_start);
            }
            _ => continue,
        }
        i = child_end;
    }
    decoded.push_verbatim(source, i, to);
    Some(decoded.finish(to))
}

/// A shell `'...'` string between its quotes: nothing in it is special, so it is its own text.
fn decode_shell_raw_string(node: Node, source: &str) -> Option<Decoded> {
    let (start, end) = (node.start_byte(), node.end_byte());
    let raw = source.get(start..end)?;
    if raw.len() < 2 || !raw.starts_with('\'') || !raw.ends_with('\'') {
        return None;
    }
    let mut decoded = Decoded::with_capacity(raw.len() - 2);
    decoded.push_verbatim(source, start + 1, end - 1);
    Some(decoded.finish(end - 1))
}

/// A C# raw string literal between its `"""` delimiters. One of several lines means what the
/// compiler makes of it: the opening and closing lines dropped, and the closing delimiter's
/// indentation taken off every line - so re-indenting the whole literal changes nothing in it.
fn decode_csharp_raw_string(node: Node, source: &str) -> Option<Decoded> {
    let child = |kind: &str| {
        let mut cursor = node.walk();
        node.children(&mut cursor)
            .find(|child| child.kind() == kind)
    };
    let (from, to) = (
        child("raw_string_start")?.end_byte(),
        child("raw_string_end")?.start_byte(),
    );
    let body = source.get(from..to)?;
    let mut decoded = Decoded::with_capacity(body.len());
    let (Some(first), Some(last)) = (body.find('\n'), body.rfind('\n')) else {
        decoded.push_verbatim(source, from, to);
        return Some(decoded.finish(to));
    };
    let (content_start, content_end) = (from + first + 1, from + last);
    let indent = &source[content_end + 1..to];
    if !indent.chars().all(char::is_whitespace) || content_start > content_end {
        return None;
    }
    let mut line_start = content_start;
    loop {
        let line_end = source[line_start..content_end]
            .find('\n')
            .map_or(content_end, |at| line_start + at);
        let line = &source[line_start..line_end];
        // A blank line may be shorter than the indentation.
        let strip = if line.starts_with(indent) {
            indent.len()
        } else {
            line.len() - line.trim_start().len()
        };
        decoded.push_verbatim(source, line_start + strip, line_end);
        if line_end == content_end {
            break;
        }
        decoded.push_verbatim(source, line_end, line_end + 1);
        line_start = line_end + 1;
    }
    Some(decoded.finish(content_end))
}

/// A Python string between its quotes: escapes resolved unless it is raw (`r'...'`), and each
/// `{...}` of an f-string as filler. Bytes (`b'...'`) are not text, so not decoded.
fn decode_python_string(node: Node, source: &str) -> Option<Decoded> {
    let child = |kind: &str| {
        let mut cursor = node.walk();
        node.children(&mut cursor)
            .find(|child| child.kind() == kind)
    };
    let (open, close) = (child("string_start")?, child("string_end")?);
    let prefix = open.utf8_text(source.as_bytes()).ok()?.to_ascii_lowercase();
    if prefix.contains('b') {
        return None;
    }
    let raw = prefix.contains('r');
    let (from, to) = (open.end_byte(), close.start_byte());
    // The escapes and interpolations, in order: everything between them is the string's own text.
    let mut special = Vec::new();
    let mut stack = vec![node];
    while let Some(current) = stack.pop() {
        match current.kind() {
            "escape_sequence" | "escape_interpolation" if !raw => special.push(current),
            "interpolation" => special.push(current),
            _ => {
                let mut cursor = current.walk();
                stack.extend(current.children(&mut cursor));
            }
        }
    }
    special.sort_by_key(|node| node.start_byte());
    let mut decoded = Decoded::with_capacity(to.saturating_sub(from));
    let mut i = from;
    for node in special {
        let (start, end) = (node.start_byte(), node.end_byte());
        if start < i || end > to {
            continue;
        }
        decoded.push_verbatim(source, i, start);
        let text = &source[start..end];
        match node.kind() {
            "interpolation" => decoded.push_filler(start, end),
            _ => match python_escape(text) {
                Some(meant) => decoded.push_from(&meant, start),
                None => decoded.push_verbatim(source, start, end),
            },
        }
        i = end;
    }
    decoded.push_verbatim(source, i, to);
    Some(decoded.finish(to))
}

/// What a Python escape sequence (`\n`, `\x41`, `{{` in an f-string) means; `None` for one kept
/// as written (`\N{...}`, or anything unrecognised).
fn python_escape(text: &str) -> Option<String> {
    if text == "{{" || text == "}}" {
        return Some(text[..1].to_string());
    }
    let rest = text.strip_prefix('\\')?;
    let code = |digits: &str, radix| {
        u32::from_str_radix(digits, radix)
            .ok()
            .and_then(char::from_u32)
    };
    let meant = match rest.chars().next()? {
        '\n' | '\r' => return Some(String::new()),
        'n' => '\n',
        't' => '\t',
        'r' => '\r',
        'a' => '\x07',
        'b' => '\x08',
        'f' => '\x0c',
        'v' => '\x0b',
        '\\' | '\'' | '"' => rest.chars().next()?,
        'x' | 'u' | 'U' => code(&rest[1..], 16)?,
        '0'..='7' => code(rest, 8)?,
        _ => return None,
    };
    Some(meant.to_string())
}

/// The language of the code the host string `node` holds, judged from where it sits; see the
/// module doc. `None` when nothing says.
pub fn detect(host: Language, node: Node, source: &str) -> Option<Language> {
    match host {
        Language::JSON => detect_in_json(node, source),
        Language::YAML => detect_in_yaml(node, source),
        Language::JavaScript | Language::TypeScript | Language::TSX => {
            detect_in_template(node, source)
        }
        Language::ShellScript => detect_in_shell(node, source),
        Language::Python => detect_in_python(node, source),
        _ => None,
    }
}

/// A pair's value whose object has a sibling `"path"` (or [`PATH_KEYS`]) naming a file of a known
/// language: `{"path": "ui/pagination.tsx", "content": "import ..."}`.
fn detect_in_json(node: Node, source: &str) -> Option<Language> {
    let pair = node.parent().filter(|parent| parent.kind() == "pair")?;
    if pair.child_by_field_name("value")?.id() != node.id() {
        return None;
    }
    let object = pair.parent()?;
    let mut cursor = object.walk();
    for sibling in object.named_children(&mut cursor) {
        if sibling.kind() != "pair" || sibling.id() == pair.id() {
            continue;
        }
        let Some(key) = sibling.child_by_field_name("key") else {
            continue;
        };
        let key_text = json_string_text(key, source);
        if !PATH_KEYS.contains(&key_text.as_str()) {
            continue;
        }
        let Some(value) = sibling.child_by_field_name("value") else {
            continue;
        };
        let path = json_string_text(value, source);
        if let Some(language) =
            crate::code::language::language_for_path(std::path::Path::new(&path))
        {
            return Some(language);
        }
    }
    None
}

/// A JSON string's decoded text, or empty if it is not one.
fn json_string_text(node: Node, source: &str) -> String {
    decode(Language::JSON, node, source)
        .map(|decoded| decoded.text)
        .unwrap_or_default()
}

/// A block scalar that is the value of a [`COMMAND_KEYS`] key is shell.
fn detect_in_yaml(node: Node, source: &str) -> Option<Language> {
    let mut parent = node.parent()?;
    while parent.kind() == "block_node" {
        parent = parent.parent()?;
    }
    if parent.kind() != "block_mapping_pair" {
        return None;
    }
    let key = parent.child_by_field_name("key")?;
    let key_text = key.utf8_text(source.as_bytes()).ok()?.trim();
    COMMAND_KEYS
        .contains(&key_text)
        .then_some(Language::ShellScript)
}

/// A tagged template whose tag names CSS or HTML, or an untagged one holding markup.
fn detect_in_template(node: Node, source: &str) -> Option<Language> {
    if let Some(call) = node
        .parent()
        .filter(|parent| parent.kind() == "call_expression")
        && let Some(tag) = call.child_by_field_name("function")
        && call
            .child_by_field_name("arguments")
            .is_some_and(|arguments| arguments.id() == node.id())
    {
        let tag_text = tag.utf8_text(source.as_bytes()).ok()?;
        let tag_text: String = tag_text.chars().filter(|c| !c.is_whitespace()).collect();
        if is_css_tag(&tag_text) {
            return Some(Language::CSS);
        }
        if is_html_tag(&tag_text) {
            return Some(Language::HTML);
        }
        return None;
    }
    let text = node.utf8_text(source.as_bytes()).ok()?;
    let body = text.trim_matches('`').trim();
    (body.starts_with('<') && body.ends_with('>')).then_some(Language::HTML)
}

/// A shell string whose first line of text is a C preprocessor directive: the header lists a build
/// script assembles (`includes_Linux='#include <sys/types.h> ...'`). Nothing around such a string
/// names its language, but no shell text reads this way.
fn detect_in_shell(node: Node, source: &str) -> Option<Language> {
    let text = node.utf8_text(source.as_bytes()).ok()?;
    let first = text
        .trim_matches('\'')
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())?;
    let directive = first.strip_prefix('#')?.trim_start();
    ["include", "define", "if", "undef", "pragma"]
        .iter()
        .any(|keyword| directive.starts_with(keyword))
        .then_some(Language::C)
}

/// The module-level strings an Ansible module documents itself in: YAML by convention.
const ANSIBLE_DOC_NAMES: &[&str] = &["DOCUMENTATION", "EXAMPLES", "RETURN"];

/// A string assigned to one of [`ANSIBLE_DOC_NAMES`] is YAML.
fn detect_in_python(node: Node, source: &str) -> Option<Language> {
    let assignment = node
        .parent()
        .filter(|parent| parent.kind() == "assignment")?;
    if assignment.child_by_field_name("right")?.id() != node.id() {
        return None;
    }
    let name = assignment
        .child_by_field_name("left")?
        .utf8_text(source.as_bytes())
        .ok()?;
    ANSIBLE_DOC_NAMES.contains(&name).then_some(Language::YAML)
}

/// The host-file byte offset of decoded byte offset `decoded_offset`, through [`Decoded::offsets`].
pub(crate) fn project_offset(offsets: &[usize], decoded_offset: usize) -> usize {
    offsets[decoded_offset.min(offsets.len() - 1)]
}

#[cfg(test)]
mod tests;
