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
//! One test per language: every definition of a sample, kind and qualified name, in document
//! order. The samples cover each kind of definition the language's table names.

use crate::code::Language;
use crate::library::{Diff, Side};

#[track_caller]
fn assert_definitions(language: Language, source: &str, expected: &[(&str, &str)]) {
    let diff = Diff::new(source, source, language);
    assert!(diff.is_structural(), "{language} sample did not parse");
    let found: Vec<(&str, &str)> = diff
        .definitions_of(Side::After)
        .iter()
        .map(|d| (d.kind().as_str(), d.qualified_name()))
        .collect();
    assert_eq!(found, expected);
}

/// Every language's sample, extracted with and without the [`super::DEFINITION_KINDS`] filter:
/// a `classify` arm whose kind is missing from the list would drop its definitions.
#[test]
fn the_kind_filter_drops_no_definition() {
    // A Windows checkout has CRLF line endings, and `include_str!` keeps them.
    let samples = include_str!("tests.rs").replace("\r\n", "\n");
    let mut checked = 0;
    for language in <Language as strum::IntoEnumIterator>::iter() {
        let marker = format!("        Language::{language:?},\n        r##\"");
        let Some(start) = samples.find(&marker).map(|i| i + marker.len()) else {
            continue;
        };
        let end = start + samples[start..].find("\"##").expect("a closed sample");
        let code = crate::code::Code::from_string(&samples[start..end], &language);
        let prefix = super::FilePrefix::default();
        let names = |filtered| -> Vec<String> {
            super::extract_filtered(&code, &prefix, filtered)
                .iter()
                .map(|d| d.qualified_name().to_string())
                .collect()
        };
        assert_eq!(names(true), names(false), "{language:?}");
        checked += 1;
    }
    assert_eq!(checked, 20);
}

#[test]
fn a_file_location_completes_the_name_where_the_language_derives_it_from_one() {
    let source = "pub struct Parser;\nimpl Parser {\n    fn parse(&self) {}\n}\n";
    let diff = Diff::new(source, source, Language::Rust)
        .with_paths("x/src/parser/lexer.rs", "x/src/parser/lexer.rs");
    let names: Vec<&str> = diff
        .definitions_of(Side::After)
        .iter()
        .map(|d| d.qualified_name())
        .collect();
    assert_eq!(
        names,
        [
            "crate::parser::lexer::Parser",
            "impl crate::parser::lexer::Parser",
            "crate::parser::lexer::Parser::parse"
        ]
    );
}

#[test]
fn a_definitions_span_takes_in_its_attributes_doc_comments_and_decorators() {
    let rust = "// not mine\n\n/// Docs.\n#[derive(Debug)]\nstruct S;\n";
    let diff = Diff::new(rust, rust, Language::Rust);
    let s = &diff.definitions_of(Side::After)[0];
    assert_eq!(
        &rust[s.span().bytes.clone()],
        "/// Docs.\n#[derive(Debug)]\nstruct S;"
    );
    assert_eq!(&rust[s.name_span().bytes.clone()], "S");
    assert_eq!((s.span().start.row, s.span().start.column), (2, 0));

    let python = "@cache\ndef f():\n    pass\n";
    let diff = Diff::new(python, python, Language::Python);
    let f = &diff.definitions_of(Side::After)[0];
    assert_eq!(f.span().bytes.start, 0);
}

#[test]
fn a_language_without_a_grammar_has_no_definitions() {
    let diff = Diff::new("fn x() {}", "fn y() {}", Language::Unknown);
    assert!(!diff.is_structural());
    assert!(diff.definitions_of(Side::Before).is_empty());
    assert_eq!(diff.changed_definitions().count(), 0);
}

#[test]
fn rust() {
    assert_definitions(
        Language::Rust,
        r##"mod parser { pub struct Parser<T> { x: T } enum E { A } union U { a: u8 } type Alias = u8; const C: u8 = 1; static S: u8 = 2;
impl<T> Parser<T> { fn new() -> Self { todo!() } const K: u8 = 1; }
impl<T> std::fmt::Display for Parser<T> { fn fmt(&self) {} }
trait Visit { fn visit(&self); fn d(&self) {} }
macro_rules! m { () => {} }
fn outer() { fn inner() {} } }"##,
        &[
            ("module", "parser"),
            ("struct", "parser::Parser"),
            ("enum", "parser::E"),
            ("union", "parser::U"),
            ("type alias", "parser::Alias"),
            ("constant", "parser::C"),
            ("variable", "parser::S"),
            ("impl", "impl parser::Parser"),
            ("method", "parser::Parser::new"),
            ("constant", "parser::Parser::K"),
            ("impl", "impl std::fmt::Display for parser::Parser"),
            ("method", "<parser::Parser as std::fmt::Display>::fmt"),
            ("trait", "parser::Visit"),
            ("method", "parser::Visit::visit"),
            ("method", "parser::Visit::d"),
            ("macro", "parser::m"),
            ("function", "parser::outer"),
            ("function", "parser::outer::inner"),
        ],
    );
}

#[test]
fn python() {
    assert_definitions(
        Language::Python,
        r##"class Parser:
    @staticmethod
    def parse(self): pass
    class Inner:
        def go(self): pass
def outer():
    def inner(): pass
async def run(): pass"##,
        &[
            ("class", "Parser"),
            ("method", "Parser.parse"),
            ("class", "Parser.Inner"),
            ("method", "Parser.Inner.go"),
            ("function", "outer"),
            ("function", "outer.<locals>.inner"),
            ("function", "run"),
        ],
    );
}

#[test]
fn go() {
    assert_definitions(
        Language::Go,
        r##"package parser
type Parser struct{ x int }
type Visitor interface{ Visit() }
type ID int
type Alias = int
const C = 1
var V = 2
func New() *Parser { return nil }
func (p *Parser) Parse() {}
func (p Parser) Value() {}
func (p *Box[T]) Get() {}"##,
        &[
            ("struct", "parser.Parser"),
            ("interface", "parser.Visitor"),
            ("type alias", "parser.ID"),
            ("type alias", "parser.Alias"),
            ("constant", "parser.C"),
            ("variable", "parser.V"),
            ("function", "parser.New"),
            ("method", "parser.Parser.Parse"),
            ("method", "parser.Parser.Value"),
            ("method", "parser.Box.Get"),
        ],
    );
}

#[test]
fn java() {
    assert_definitions(
        Language::Java,
        r##"package com.acme;
public class Parser { Parser() {} void parse(int a) {} void parse(String s, int[] b) {} static class Inner { void go() {} }
interface V { void visit(); } enum E { A; void m() {} } record R(int x) {} @interface Ann {} }"##,
        &[
            ("class", "com.acme.Parser"),
            ("method", "com.acme.Parser.Parser"),
            ("method", "com.acme.Parser.parse(int)"),
            ("method", "com.acme.Parser.parse(String, int[])"),
            ("class", "com.acme.Parser.Inner"),
            ("method", "com.acme.Parser.Inner.go"),
            ("interface", "com.acme.Parser.V"),
            ("method", "com.acme.Parser.V.visit"),
            ("enum", "com.acme.Parser.E"),
            ("method", "com.acme.Parser.E.m"),
            ("class", "com.acme.Parser.R"),
            ("interface", "com.acme.Parser.Ann"),
        ],
    );
}

#[test]
fn kotlin() {
    assert_definitions(
        Language::Kotlin,
        r##"package com.acme

class Parser {
    fun parse(a: Int) {}

    companion object {
        fun create() {}
    }
}

interface V {
    fun visit()
}

enum class E { A }

object Single {
    fun go() {}
}

fun top() {}

typealias Alias = Int"##,
        &[
            ("class", "com.acme.Parser"),
            ("method", "com.acme.Parser.parse"),
            ("class", "com.acme.Parser.Companion"),
            ("method", "com.acme.Parser.Companion.create"),
            ("interface", "com.acme.V"),
            ("method", "com.acme.V.visit"),
            ("enum", "com.acme.E"),
            ("class", "com.acme.Single"),
            ("method", "com.acme.Single.go"),
            ("function", "com.acme.top"),
            ("type alias", "com.acme.Alias"),
        ],
    );
}

#[test]
fn scala() {
    assert_definitions(
        Language::Scala,
        r##"package com.acme
class Parser { def parse(a: Int): Unit = {} }
object Parser { def apply(): Parser = ??? }
trait V { def visit(): Unit }
enum E { case A }
type Alias = Int
def top(): Unit = {}"##,
        &[
            ("class", "com.acme.Parser"),
            ("method", "com.acme.Parser.parse"),
            ("class", "com.acme.Parser"),
            ("method", "com.acme.Parser.apply"),
            ("trait", "com.acme.V"),
            ("method", "com.acme.V.visit"),
            ("enum", "com.acme.E"),
            ("type alias", "com.acme.Alias"),
            ("function", "com.acme.top"),
        ],
    );
}

#[test]
fn csharp() {
    assert_definitions(
        Language::CSharp,
        r##"namespace Acme.Parsing { public class Parser { public Parser() {} public void Parse(int a) {} public void Parse(string s) {} public int P { get; set; } }
interface IV { void Visit(); } struct S {} enum E { A } record R(int X); delegate void D(); }
namespace Other;
class Late { void M() {} }"##,
        &[
            ("module", "Acme.Parsing"),
            ("class", "Acme.Parsing.Parser"),
            ("method", "Acme.Parsing.Parser.Parser"),
            ("method", "Acme.Parsing.Parser.Parse(int)"),
            ("method", "Acme.Parsing.Parser.Parse(string)"),
            ("interface", "Acme.Parsing.IV"),
            ("method", "Acme.Parsing.IV.Visit"),
            ("struct", "Acme.Parsing.S"),
            ("enum", "Acme.Parsing.E"),
            ("class", "Acme.Parsing.R"),
            ("type alias", "Acme.Parsing.D"),
            ("class", "Other.Late"),
            ("method", "Other.Late.M"),
        ],
    );
}

#[test]
fn cpp() {
    assert_definitions(
        Language::CPP,
        r##"namespace acme { namespace inner { class Parser { public: void parse(int a); int x; }; }
void inner::Parser::parse(int a) {}
template <typename T> struct Box { T get() { return T(); } };
enum class E { A }; union U { int a; }; using Alias = int; typedef int Old;
int free_fn(int a, char* b) { return 0; } }
#define MAX(a,b) ((a)>(b)?(a):(b))
#define LIMIT 3"##,
        &[
            ("module", "acme"),
            ("module", "acme::inner"),
            ("class", "acme::inner::Parser"),
            ("method", "acme::inner::Parser::parse"),
            ("struct", "acme::Box"),
            ("method", "acme::Box::get"),
            ("enum", "acme::E"),
            ("union", "acme::U"),
            ("type alias", "acme::Alias"),
            ("type alias", "acme::Old"),
            ("function", "acme::free_fn"),
            ("macro", "MAX"),
            ("macro", "LIMIT"),
        ],
    );
}

#[test]
fn c() {
    assert_definitions(
        Language::C,
        r##"struct point { int x; }; enum color { RED }; union u { int a; }; typedef struct point point_t;
static int helper(int a) { return a; }
int *make(void) { return 0; }
#define SQR(x) ((x)*(x))
struct decl_only;"##,
        &[
            ("struct", "point"),
            ("enum", "color"),
            ("union", "u"),
            ("type alias", "point_t"),
            ("function", "helper"),
            ("function", "make"),
            ("macro", "SQR"),
        ],
    );
}

#[test]
fn javascript() {
    assert_definitions(
        Language::JavaScript,
        r##"export class Parser { constructor() {} parse() {} static make() {} get size() { return 1; } }
function top() {}
function* gen() {}
const arrow = () => {};
let fexpr = function () {};
export default function () {}
const obj = { method() {} };"##,
        &[
            ("class", "Parser"),
            ("method", "Parser.constructor"),
            ("method", "Parser.parse"),
            ("method", "Parser.make"),
            ("method", "Parser.size"),
            ("function", "top"),
            ("function", "gen"),
            ("function", "arrow"),
            ("function", "fexpr"),
        ],
    );
}

#[test]
fn typescript() {
    assert_definitions(
        Language::TypeScript,
        r##"namespace Acme { export class Parser { parse(a: number): void {} } }
interface V { visit(): void }
type Alias = number;
enum E { A }
abstract class Base { abstract m(): void; }
function top(a: string): void {}
declare module "x" { function y(): void; }"##,
        &[
            ("module", "Acme"),
            ("class", "Acme.Parser"),
            ("method", "Acme.Parser.parse"),
            ("interface", "V"),
            ("method", "V.visit"),
            ("type alias", "Alias"),
            ("enum", "E"),
            ("class", "Base"),
            ("method", "Base.m"),
            ("function", "top"),
            ("module", "x"),
            ("function", "x.y"),
        ],
    );
}

#[test]
fn php() {
    assert_definitions(
        Language::PHP,
        r##"<?php
namespace Acme\Parsing;
class Parser { public function parse() {} public static function make() {} }
interface V { public function visit(); }
trait T { function t() {} }
enum E { case A; }
function top() {}"##,
        &[
            ("class", "Acme\\Parsing\\Parser"),
            ("method", "Acme\\Parsing\\Parser::parse"),
            ("method", "Acme\\Parsing\\Parser::make"),
            ("interface", "Acme\\Parsing\\V"),
            ("method", "Acme\\Parsing\\V::visit"),
            ("trait", "Acme\\Parsing\\T"),
            ("method", "Acme\\Parsing\\T::t"),
            ("enum", "Acme\\Parsing\\E"),
            ("function", "Acme\\Parsing\\top"),
        ],
    );
}

#[test]
fn ruby() {
    assert_definitions(
        Language::Ruby,
        r##"module Acme
  class Parser < Base
    def parse; end
    def self.make; end
    class << self
      def other; end
    end
  end
end
def top; end"##,
        &[
            ("module", "Acme"),
            ("class", "Acme::Parser"),
            ("method", "Acme::Parser#parse"),
            ("method", "Acme::Parser.make"),
            ("method", "Acme::Parser.other"),
            ("function", "top"),
        ],
    );
}

#[test]
fn swift() {
    assert_definitions(
        Language::Swift,
        r##"class Parser {
    func parse(_ a: Int, with b: String) {}
    func parse(_ a: Int) {}
    init() {}
}
struct S {
    func m() {}
}
enum E {
    case a
}
protocol V {
    func visit()
}
extension Parser {
    func more() {}
}
typealias Alias = Int
func top() {}"##,
        &[
            ("class", "Parser"),
            ("method", "Parser.parse(_:with:)"),
            ("method", "Parser.parse(_:)"),
            ("method", "Parser.init"),
            ("struct", "S"),
            ("method", "S.m"),
            ("enum", "E"),
            ("interface", "V"),
            ("method", "V.visit"),
            ("impl", "extension Parser"),
            ("method", "Parser.more"),
            ("type alias", "Alias"),
            ("function", "top"),
        ],
    );
}

#[test]
fn lua() {
    assert_definitions(
        Language::LUA,
        r##"local M = {}
function M.parse() end
function M:method() end
local function helper() end
function global() end
local f = function() end"##,
        &[
            ("function", "M.parse"),
            ("function", "M:method"),
            ("function", "helper"),
            ("function", "global"),
        ],
    );
}

#[test]
fn shell_script() {
    assert_definitions(
        Language::ShellScript,
        r##"build() { echo; }
function deploy { echo; }"##,
        &[("function", "build"), ("function", "deploy")],
    );
}

#[test]
fn perl() {
    assert_definitions(
        Language::Perl,
        r##"package Acme::Parser;
sub parse { 1 }
package Other;
sub go { 2 }"##,
        &[
            ("function", "Acme::Parser::parse"),
            ("function", "Other::go"),
        ],
    );
}

#[test]
fn r() {
    assert_definitions(
        Language::R,
        r##"parse <- function(x) x
helper = function(y) y"##,
        &[("function", "parse"), ("function", "helper")],
    );
}

#[test]
fn vimscript() {
    assert_definitions(
        Language::Vimscript,
        r##"function! s:Helper() abort
endfunction
function Global()
endfunction"##,
        &[("function", "s:Helper"), ("function", "Global")],
    );
}

#[test]
fn sql() {
    assert_definitions(
        Language::SQL,
        r##"CREATE TABLE users (id int);
CREATE VIEW v AS SELECT 1;
CREATE FUNCTION f() RETURNS int AS $$ SELECT 1 $$ LANGUAGE sql;"##,
        &[("other", "users"), ("other", "v"), ("other", "f")],
    );
}
