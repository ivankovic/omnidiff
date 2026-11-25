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

//! Synthetic before/after pairs of a chosen size, one generator per language with a grammar.
//!
//! A pair is a file grown along one [`Axis`] to size `n`, and the same file with one number (or,
//! on [`Axis::Token`], one character) changed where growing it is most likely to break something:
//! after the last line, at the bottom of the nesting, at the end of the long line or token. The
//! rest of each pair is identical on both sides, so a correct diff reports exactly that change.
//!
//! The files are varied by a seed, not random text: each language has a few statement shapes
//! ([`Template::units`]) that a seeded generator picks from line by line, and the numbers in them
//! are drawn from it too. Every shape is valid code, so at small sizes the file parses without
//! errors, which `size_limits check` asserts; whatever breaks as the file grows is a limit, not a
//! syntax error in the generator.
//!
//! This file is also compiled into `tests/size_limits.rs` with `#[path]`, so it depends on nothing
//! but `omnidiff::code::Language`.

use omnidiff::code::Language;
use std::fmt::Write;

/// The dimension a pair is grown along.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Axis {
    /// `n` top-level statements, a few lines each; the last one changes.
    Lines,
    /// `n` levels of the language's own nesting (blocks, elements, mappings); the innermost
    /// statement changes.
    Depth,
    /// One line holding a list of `n` elements; the last element changes.
    Width,
    /// One string literal (or text node) of `n` characters; the last character changes.
    Token,
}

impl Axis {
    pub const ALL: [Axis; 4] = [Axis::Lines, Axis::Depth, Axis::Width, Axis::Token];

    pub fn name(self) -> &'static str {
        match self {
            Axis::Lines => "lines",
            Axis::Depth => "depth",
            Axis::Width => "width",
            Axis::Token => "token",
        }
    }

    pub fn from_name(name: &str) -> Option<Axis> {
        Axis::ALL.into_iter().find(|axis| axis.name() == name)
    }
}

/// A small deterministic generator (SplitMix64), so a seed names the same files on every machine.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }

    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    pub fn below(&mut self, bound: u64) -> u64 {
        self.next() % bound
    }
}

/// One statement shape: `(index, value)` to its text, newline included. The index keeps names
/// unique; the value is the number a pair's change edits.
type Unit = fn(usize, u64) -> String;

/// How one language's files are built. The generated part goes between `prologue` and `epilogue`.
pub struct Template {
    pub language: Language,
    /// The extension the file is written with, which is how `omnidiff` detects the language.
    pub extension: &'static str,
    pub prologue: &'static str,
    pub epilogue: &'static str,
    /// The shapes [`Axis::Lines`] picks from.
    pub units: &'static [Unit],
    /// One level of nesting `(level)` opened and closed, and the statement at the bottom
    /// `(depth, value)`. Levels are not indented where the language does not need it, so a file's
    /// size stays linear in its depth.
    pub open: fn(usize) -> String,
    pub close: fn(usize) -> String,
    pub leaf: fn(usize, u64) -> String,
    /// Where the nesting sits: a statement block needs an enclosing function in most languages.
    pub nest_prologue: &'static str,
    pub nest_epilogue: &'static str,
    /// A statement whose single line holds these elements, comma-separated or as the language
    /// lists them.
    pub list: fn(&[String]) -> String,
    /// A statement holding this text as a string literal or text node.
    pub string: fn(&str) -> String,
}

/// One side of a pair.
pub fn generate(template: &Template, axis: Axis, n: usize, seed: u64, changed: bool) -> String {
    let mut rng = Rng::new(seed);
    let mut out = String::new();
    // The changed value is one more than the original, so both sides draw the same numbers.
    let bump = |value: u64| if changed { value + 1 } else { value };
    match axis {
        Axis::Lines => {
            out.push_str(template.prologue);
            for i in 0..n {
                let unit = template.units[rng.below(template.units.len() as u64) as usize];
                let value = rng.below(1000);
                let value = if i + 1 == n { bump(value) } else { value };
                out.push_str(&unit(i, value));
            }
            out.push_str(template.epilogue);
        }
        Axis::Depth => {
            out.push_str(template.prologue);
            out.push_str(template.nest_prologue);
            for level in 0..n {
                out.push_str(&(template.open)(level));
            }
            out.push_str(&(template.leaf)(n, bump(rng.below(1000))));
            for level in (0..n).rev() {
                out.push_str(&(template.close)(level));
            }
            out.push_str(template.nest_epilogue);
            out.push_str(template.epilogue);
        }
        Axis::Width => {
            let mut elements: Vec<String> = (0..n).map(|_| rng.below(1000).to_string()).collect();
            if let Some(last) = elements.last_mut() {
                *last = bump(last.parse().unwrap_or(0)).to_string();
            }
            out.push_str(template.prologue);
            out.push_str(&(template.list)(&elements));
            out.push_str(template.epilogue);
        }
        Axis::Token => {
            let mut text = "x".repeat(n.saturating_sub(1));
            text.push(if changed { 'b' } else { 'a' });
            out.push_str(template.prologue);
            out.push_str(&(template.string)(&text));
            out.push_str(template.epilogue);
        }
    }
    out
}

fn joined(elements: &[String], separator: &str) -> String {
    let mut out = String::new();
    for (i, element) in elements.iter().enumerate() {
        if i > 0 {
            out.push_str(separator);
        }
        out.push_str(element);
    }
    out
}

fn repeat_into(prefix: &str, pieces: &[String], suffix: &str) -> String {
    let mut out = String::from(prefix);
    for piece in pieces {
        out.push_str(piece);
    }
    out.push_str(suffix);
    out
}

fn tag_list(elements: &[String]) -> String {
    let items: Vec<String> = elements.iter().map(|e| format!("<b>{e}</b>")).collect();
    repeat_into("<p>", &items, "</p>\n")
}

fn s(text: &str) -> String {
    text.to_string()
}

/// Every language with a grammar, in `Language` order.
pub fn templates() -> Vec<Template> {
    vec![
        Template {
            language: Language::C,
            extension: "c",
            prologue: "#include <stdio.h>\n\n",
            epilogue: "",
            units: &[
                |i, v| format!("int f{i}(void)\n{{\n    return {v};\n}}\n"),
                |i, v| format!("static int g{i} = {v};\n"),
                |i, v| format!("struct s{i} {{\n    int a;\n    int b;\n}} t{i} = {{ {v}, 0 }};\n"),
            ],
            open: |_| s("{\n"),
            close: |_| s("}\n"),
            leaf: |_, v| format!("int x = {v};\n"),
            nest_prologue: "int main(void)\n{\n",
            nest_epilogue: "}\n",
            list: |e| format!("int a[] = {{{}}};\n", joined(e, ", ")),
            string: |t| format!("const char *s = \"{t}\";\n"),
        },
        Template {
            language: Language::CPP,
            extension: "cpp",
            prologue: "#include <vector>\n\nnamespace n {\n",
            epilogue: "}  // namespace n\n",
            units: &[
                |i, v| format!("int f{i}() {{\n    return {v};\n}}\n"),
                |i, v| format!("constexpr int g{i} = {v};\n"),
                |i, v| {
                    format!("class C{i} {{\n public:\n    int m() const {{ return {v}; }}\n}};\n")
                },
            ],
            open: |_| s("{\n"),
            close: |_| s("}\n"),
            leaf: |_, v| format!("int x = {v};\n"),
            nest_prologue: "int main() {\n",
            nest_epilogue: "}\n",
            list: |e| format!("std::vector<int> a = {{{}}};\n", joined(e, ", ")),
            string: |t| format!("const char *s = \"{t}\";\n"),
        },
        Template {
            language: Language::CSS,
            extension: "css",
            prologue: "",
            epilogue: "",
            units: &[
                |i, v| format!(".c{i} {{\n  margin: {v}px;\n}}\n"),
                |i, v| format!("#d{i} {{ width: {v}%; color: red; }}\n"),
            ],
            open: |_| s("@media screen {\n"),
            close: |_| s("}\n"),
            leaf: |_, v| format!(".x {{ margin: {v}px; }}\n"),
            nest_prologue: "",
            nest_epilogue: "",
            list: |e| {
                let items: Vec<String> = e.iter().map(|e| format!("{e}px")).collect();
                format!(".a {{ grid-template-columns: {}; }}\n", joined(&items, " "))
            },
            string: |t| format!(".a {{ content: \"{t}\"; }}\n"),
        },
        Template {
            language: Language::CSharp,
            extension: "cs",
            prologue: "namespace N\n{\npublic class C\n{\n",
            epilogue: "}\n}\n",
            units: &[
                |i, v| format!("    public int F{i}()\n    {{\n        return {v};\n    }}\n"),
                |i, v| format!("    private int g{i} = {v};\n"),
            ],
            open: |_| s("{\n"),
            close: |_| s("}\n"),
            leaf: |_, v| format!("int x = {v};\n"),
            nest_prologue: "public void M()\n{\n",
            nest_epilogue: "}\n",
            list: |e| format!("    int[] a = {{{}}};\n", joined(e, ", ")),
            string: |t| format!("    string s = \"{t}\";\n"),
        },
        Template {
            language: Language::Go,
            extension: "go",
            prologue: "package main\n\n",
            epilogue: "",
            units: &[
                |i, v| format!("func f{i}() int {{\n\treturn {v}\n}}\n"),
                |i, v| format!("var g{i} = {v}\n"),
                |i, v| format!("type T{i} struct {{\n\tA int\n}}\n\nvar t{i} = T{i}{{A: {v}}}\n"),
            ],
            open: |_| s("{\n"),
            close: |_| s("}\n"),
            leaf: |_, v| format!("x := {v}\n_ = x\n"),
            nest_prologue: "func main() {\n",
            nest_epilogue: "}\n",
            list: |e| format!("var a = []int{{{}}}\n", joined(e, ", ")),
            string: |t| format!("var s = \"{t}\"\n"),
        },
        Template {
            language: Language::HTML,
            extension: "html",
            prologue: "<!DOCTYPE html>\n<html>\n<body>\n",
            epilogue: "</body>\n</html>\n",
            units: &[
                |i, v| format!("<p id=\"p{i}\">{v}</p>\n"),
                |i, v| format!("<div class=\"d{i}\">\n  <span>{v}</span>\n</div>\n"),
                |i, v| format!("<ul>\n  <li>{i}</li>\n  <li>{v}</li>\n</ul>\n"),
            ],
            open: |_| s("<div>\n"),
            close: |_| s("</div>\n"),
            leaf: |_, v| format!("<p>{v}</p>\n"),
            nest_prologue: "",
            nest_epilogue: "",
            list: tag_list,
            string: |t| format!("<p>{t}</p>\n"),
        },
        Template {
            language: Language::JSON,
            extension: "json",
            prologue: "{\n",
            epilogue: "  \"end\": 0\n}\n",
            units: &[
                |i, v| format!("  \"k{i}\": {v},\n"),
                |i, v| format!("  \"o{i}\": {{\n    \"a\": {v},\n    \"b\": [1, 2]\n  }},\n"),
            ],
            open: |_| s("{\"a\":\n"),
            close: |_| s("}\n"),
            leaf: |_, v| format!("{v}\n"),
            nest_prologue: "  \"n\": ",
            nest_epilogue: ",\n",
            list: |e| format!("  \"a\": [{}],\n", joined(e, ", ")),
            string: |t| format!("  \"s\": \"{t}\",\n"),
        },
        Template {
            language: Language::Java,
            extension: "java",
            prologue: "package p;\n\npublic class C {\n",
            epilogue: "}\n",
            units: &[
                |i, v| format!("    public int f{i}() {{\n        return {v};\n    }}\n"),
                |i, v| format!("    private int g{i} = {v};\n"),
            ],
            open: |_| s("{\n"),
            close: |_| s("}\n"),
            leaf: |_, v| format!("int x = {v};\n"),
            nest_prologue: "public void m() {\n",
            nest_epilogue: "}\n",
            list: |e| format!("    int[] a = {{{}}};\n", joined(e, ", ")),
            string: |t| format!("    String s = \"{t}\";\n"),
        },
        Template {
            language: Language::JavaScript,
            extension: "js",
            prologue: "'use strict';\n\n",
            epilogue: "",
            units: &[
                |i, v| format!("function f{i}() {{\n  return {v};\n}}\n"),
                |i, v| format!("const g{i} = {v};\n"),
                |i, v| format!("const o{i} = {{\n  a: {v},\n  b: [1, 2],\n}};\n"),
            ],
            open: |_| s("{\n"),
            close: |_| s("}\n"),
            leaf: |_, v| format!("let x = {v};\n"),
            nest_prologue: "function main() {\n",
            nest_epilogue: "}\n",
            list: |e| format!("const a = [{}];\n", joined(e, ", ")),
            string: |t| format!("const s = \"{t}\";\n"),
        },
        Template {
            language: Language::Kotlin,
            extension: "kt",
            prologue: "package p\n\n",
            epilogue: "",
            units: &[
                |i, v| format!("fun f{i}(): Int {{\n    return {v}\n}}\n"),
                |i, v| format!("val g{i} = {v}\n"),
            ],
            open: |_| s("if (true) {\n"),
            close: |_| s("}\n"),
            leaf: |_, v| format!("val x = {v}\n"),
            nest_prologue: "fun main() {\n",
            nest_epilogue: "}\n",
            list: |e| format!("val a = listOf({})\n", joined(e, ", ")),
            string: |t| format!("val s = \"{t}\"\n"),
        },
        Template {
            language: Language::LUA,
            extension: "lua",
            prologue: "",
            epilogue: "",
            units: &[
                |i, v| format!("function f{i}()\n  return {v}\nend\n"),
                |i, v| format!("local g{i} = {v}\n"),
                |i, v| format!("local t{i} = {{\n  a = {v},\n  b = {{1, 2}},\n}}\n"),
            ],
            open: |_| s("do\n"),
            close: |_| s("end\n"),
            leaf: |_, v| format!("local x = {v}\n"),
            nest_prologue: "",
            nest_epilogue: "",
            list: |e| format!("local a = {{{}}}\n", joined(e, ", ")),
            string: |t| format!("local s = \"{t}\"\n"),
        },
        Template {
            language: Language::PHP,
            extension: "php",
            prologue: "<?php\n\n",
            epilogue: "",
            units: &[
                |i, v| format!("function f{i}() {{\n    return {v};\n}}\n"),
                |i, v| format!("$g{i} = {v};\n"),
            ],
            open: |_| s("{\n"),
            close: |_| s("}\n"),
            leaf: |_, v| format!("$x = {v};\n"),
            nest_prologue: "function main() {\n",
            nest_epilogue: "}\n",
            list: |e| format!("$a = [{}];\n", joined(e, ", ")),
            string: |t| format!("$s = \"{t}\";\n"),
        },
        Template {
            language: Language::Perl,
            extension: "pl",
            prologue: "use strict;\nuse warnings;\n\n",
            epilogue: "",
            units: &[
                |i, v| format!("sub f{i} {{\n    return {v};\n}}\n"),
                |i, v| format!("my $g{i} = {v};\n"),
            ],
            open: |_| s("{\n"),
            close: |_| s("}\n"),
            leaf: |_, v| format!("my $x = {v};\n"),
            nest_prologue: "",
            nest_epilogue: "",
            list: |e| format!("my @a = ({});\n", joined(e, ", ")),
            string: |t| format!("my $s = \"{t}\";\n"),
        },
        Template {
            language: Language::Python,
            extension: "py",
            prologue: "import os\n\n",
            epilogue: "",
            units: &[
                |i, v| format!("def f{i}():\n    return {v}\n"),
                |i, v| format!("g{i} = {v}\n"),
                |i, v| format!("class C{i}:\n    a = {v}\n    b = [1, 2]\n"),
            ],
            // One space a level: Python takes any consistent indentation.
            open: |level| format!("{}if True:\n", " ".repeat(level)),
            close: |_| String::new(),
            leaf: |depth, v| format!("{}x = {v}\n", " ".repeat(depth)),
            nest_prologue: "",
            nest_epilogue: "",
            list: |e| format!("a = [{}]\n", joined(e, ", ")),
            string: |t| format!("s = \"{t}\"\n"),
        },
        Template {
            language: Language::R,
            extension: "r",
            prologue: "",
            epilogue: "",
            units: &[
                |i, v| format!("f{i} <- function() {{\n  {v}\n}}\n"),
                |i, v| format!("g{i} <- {v}\n"),
            ],
            open: |_| s("{\n"),
            close: |_| s("}\n"),
            leaf: |_, v| format!("x <- {v}\n"),
            nest_prologue: "",
            nest_epilogue: "",
            list: |e| format!("a <- c({})\n", joined(e, ", ")),
            string: |t| format!("s <- \"{t}\"\n"),
        },
        Template {
            language: Language::Ruby,
            extension: "rb",
            prologue: "# frozen_string_literal: true\n\n",
            epilogue: "",
            units: &[
                |i, v| format!("def f{i}\n  {v}\nend\n"),
                |i, v| format!("G{i} = {v}\n"),
                |i, v| format!("class C{i}\n  A = {v}\nend\n"),
            ],
            open: |_| s("begin\n"),
            close: |_| s("end\n"),
            leaf: |_, v| format!("x = {v}\n"),
            nest_prologue: "",
            nest_epilogue: "",
            list: |e| format!("a = [{}]\n", joined(e, ", ")),
            string: |t| format!("s = \"{t}\"\n"),
        },
        Template {
            language: Language::Rust,
            extension: "rs",
            prologue: "use std::fmt;\n\n",
            epilogue: "",
            units: &[
                |i, v| format!("fn f{i}() -> i32 {{\n    {v}\n}}\n"),
                |i, v| format!("const G{i}: i32 = {v};\n"),
                |i, v| {
                    format!(
                        "struct S{i} {{\n    a: i32,\n}}\n\nstatic T{i}: S{i} = S{i} {{ a: {v} }};\n"
                    )
                },
            ],
            open: |_| s("{\n"),
            close: |_| s("}\n"),
            leaf: |_, v| format!("let x = {v};\n"),
            nest_prologue: "fn main() {\n",
            nest_epilogue: "}\n",
            list: |e| format!("const A: &[i32] = &[{}];\n", joined(e, ", ")),
            string: |t| format!("const S: &str = \"{t}\";\n"),
        },
        Template {
            language: Language::Scala,
            extension: "scala",
            prologue: "package p\n\nobject O {\n",
            epilogue: "}\n",
            units: &[
                |i, v| format!("  def f{i}(): Int = {{\n    {v}\n  }}\n"),
                |i, v| format!("  val g{i} = {v}\n"),
            ],
            open: |_| s("{\n"),
            close: |_| s("}\n"),
            leaf: |_, v| format!("val x = {v}\nx\n"),
            nest_prologue: "def m(): Int = ",
            nest_epilogue: "\n",
            list: |e| format!("  val a = List({})\n", joined(e, ", ")),
            string: |t| format!("  val s = \"{t}\"\n"),
        },
        Template {
            language: Language::SQL,
            extension: "sql",
            prologue: "",
            epilogue: "",
            units: &[
                |i, v| format!("CREATE TABLE t{i} (a INT DEFAULT {v});\n"),
                |i, v| format!("INSERT INTO t (id, a) VALUES ({i}, {v});\n"),
                |i, v| format!("SELECT a, b\nFROM t\nWHERE id = {i} AND a > {v};\n"),
            ],
            open: |_| s("(SELECT a FROM\n"),
            close: |level| format!(") AS q{level}\n"),
            leaf: |_, v| format!("t WHERE a = {v}\n"),
            nest_prologue: "SELECT a FROM\n",
            nest_epilogue: ";\n",
            list: |e| format!("SELECT {};\n", joined(e, ", ")),
            string: |t| format!("SELECT '{t}';\n"),
        },
        Template {
            language: Language::ShellScript,
            extension: "sh",
            prologue: "#!/bin/bash\n\n",
            epilogue: "",
            units: &[
                |i, v| format!("f{i}() {{\n  echo {v}\n}}\n"),
                |i, v| format!("g{i}={v}\n"),
                |i, v| format!("if [ \"$x\" = {i} ]; then\n  echo {v}\nfi\n"),
            ],
            open: |_| s("{\n"),
            close: |_| s("}\n"),
            leaf: |_, v| format!("x={v}\n"),
            nest_prologue: "",
            nest_epilogue: "",
            list: |e| format!("a=({})\n", joined(e, " ")),
            string: |t| format!("s=\"{t}\"\n"),
        },
        Template {
            language: Language::Swift,
            extension: "swift",
            prologue: "import Foundation\n\n",
            epilogue: "",
            units: &[
                |i, v| format!("func f{i}() -> Int {{\n    return {v}\n}}\n"),
                |i, v| format!("let g{i} = {v}\n"),
            ],
            open: |_| s("do {\n"),
            close: |_| s("}\n"),
            leaf: |_, v| format!("let x = {v}\n"),
            nest_prologue: "func main() {\n",
            nest_epilogue: "}\n",
            list: |e| format!("let a = [{}]\n", joined(e, ", ")),
            string: |t| format!("let s = \"{t}\"\n"),
        },
        Template {
            language: Language::TSX,
            extension: "tsx",
            prologue: "import React from 'react';\n\n",
            epilogue: "",
            units: &[
                |i, v| format!("function f{i}(): number {{\n  return {v};\n}}\n"),
                |i, v| format!("const g{i}: number = {v};\n"),
                |i, v| format!("const e{i} = <div className=\"c{i}\">{v}</div>;\n"),
            ],
            open: |_| s("{\n"),
            close: |_| s("}\n"),
            leaf: |_, v| format!("let x = {v};\n"),
            nest_prologue: "function main() {\n",
            nest_epilogue: "}\n",
            list: |e| format!("const a = [{}];\n", joined(e, ", ")),
            string: |t| format!("const s = \"{t}\";\n"),
        },
        Template {
            language: Language::TypeScript,
            extension: "ts",
            prologue: "import fs from 'fs';\n\n",
            epilogue: "",
            units: &[
                |i, v| format!("function f{i}(): number {{\n  return {v};\n}}\n"),
                |i, v| format!("const g{i}: number = {v};\n"),
                |i, v| {
                    format!(
                        "interface I{i} {{\n  a: number;\n}}\nconst o{i}: I{i} = {{ a: {v} }};\n"
                    )
                },
            ],
            open: |_| s("{\n"),
            close: |_| s("}\n"),
            leaf: |_, v| format!("let x = {v};\n"),
            nest_prologue: "function main() {\n",
            nest_epilogue: "}\n",
            list: |e| format!("const a = [{}];\n", joined(e, ", ")),
            string: |t| format!("const s = \"{t}\";\n"),
        },
        Template {
            language: Language::Vimscript,
            extension: "vim",
            prologue: "",
            epilogue: "",
            units: &[
                |i, v| format!("function! F{i}()\n  return {v}\nendfunction\n"),
                |i, v| format!("let g:g{i} = {v}\n"),
            ],
            open: |_| s("if 1\n"),
            close: |_| s("endif\n"),
            leaf: |_, v| format!("let x = {v}\n"),
            nest_prologue: "",
            nest_epilogue: "",
            list: |e| format!("let a = [{}]\n", joined(e, ", ")),
            string: |t| format!("let s = \"{t}\"\n"),
        },
        Template {
            language: Language::Vue,
            extension: "vue",
            prologue: "<template>\n  <div>\n",
            epilogue: "  </div>\n</template>\n\n<script>\nexport default {}\n</script>\n",
            units: &[
                |i, v| format!("    <p id=\"p{i}\">{v}</p>\n"),
                |i, v| format!("    <div class=\"d{i}\">\n      <span>{v}</span>\n    </div>\n"),
            ],
            open: |_| s("<div>\n"),
            close: |_| s("</div>\n"),
            leaf: |_, v| format!("<p>{v}</p>\n"),
            nest_prologue: "",
            nest_epilogue: "",
            list: tag_list,
            string: |t| format!("<p>{t}</p>\n"),
        },
        Template {
            language: Language::YAML,
            extension: "yaml",
            prologue: "",
            epilogue: "",
            units: &[
                |i, v| format!("k{i}: {v}\n"),
                |i, v| format!("m{i}:\n  a: {v}\n  b: [1, 2]\n"),
                |i, v| format!("l{i}:\n  - name: n{i}\n    value: {v}\n"),
            ],
            // One space a level, the least YAML takes.
            open: |level| format!("{}k{level}:\n", " ".repeat(level)),
            close: |_| String::new(),
            leaf: |depth, v| format!("{}v: {v}\n", " ".repeat(depth)),
            nest_prologue: "",
            nest_epilogue: "",
            list: |e| format!("a: [{}]\n", joined(e, ", ")),
            string: |t| format!("s: \"{t}\"\n"),
        },
        Template {
            language: Language::XML,
            extension: "xml",
            prologue: "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<root>\n",
            epilogue: "</root>\n",
            units: &[
                |i, v| format!("  <item id=\"{i}\">{v}</item>\n"),
                |i, v| format!("  <entry key=\"k{i}\" value=\"{v}\"/>\n"),
                |i, v| format!("  <group n=\"{i}\">\n    <a>{v}</a>\n  </group>\n"),
            ],
            open: |_| s("<e>\n"),
            close: |_| s("</e>\n"),
            leaf: |_, v| format!("<v>{v}</v>\n"),
            nest_prologue: "",
            nest_epilogue: "",
            list: tag_list,
            string: |t| format!("<s>{t}</s>\n"),
        },
    ]
}

/// The first and one-past-last byte where `before` and `after` differ, on each side.
pub fn changed_span(before: &str, after: &str) -> ((usize, usize), (usize, usize)) {
    let (b, a) = (before.as_bytes(), after.as_bytes());
    let prefix = b.iter().zip(a).take_while(|(x, y)| x == y).count();
    let max_suffix = b.len().min(a.len()) - prefix;
    let suffix = b
        .iter()
        .rev()
        .zip(a.iter().rev())
        .take(max_suffix)
        .take_while(|(x, y)| x == y)
        .count();
    ((prefix, b.len() - suffix), (prefix, a.len() - suffix))
}

/// Byte offset to `(row, column)`, both zero-based, the column in bytes.
pub fn position(text: &str, offset: usize) -> (usize, usize) {
    let before = &text.as_bytes()[..offset.min(text.len())];
    let row = before.iter().filter(|&&b| b == b'\n').count();
    let line_start = before
        .iter()
        .rposition(|&b| b == b'\n')
        .map_or(0, |p| p + 1);
    (row, before.len() - line_start)
}

/// A short description of a generated file: its line count, size, and deepest line.
pub fn shape(text: &str) -> String {
    let mut out = String::new();
    let lines = text.lines().count();
    let longest = text.lines().map(str::len).max().unwrap_or(0);
    let _ = write!(
        out,
        "{lines} lines, {} bytes, longest line {longest}",
        text.len()
    );
    out
}
