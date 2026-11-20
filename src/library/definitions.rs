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
//! Definitions - functions, methods, types, modules, impl blocks - found in a syntax tree, named
//! the way the language's own imports name them.
//!
//! **The table is our own, not tree-sitter's `tags.scm`.** 15 of the 25 grammars ship a tags
//! query, but Rust's labels structs, enums, unions and type aliases all `definition.class` and
//! lists `impl` blocks only as references, and Kotlin, Bash and Vim script have none. The kind
//! names follow the tags vocabulary where it has one (function, method, class, interface, module,
//! macro). The table is also kept apart from `diff::nodes::is_semantically_structural`, which
//! names declarations for the matcher: a change here must never move a quality gate.
//!
//! **Only definitions with a body, or that are whole on their own** (a Rust `const`, a type
//! alias, a `#define`): C and C++ prototypes and forward declarations are references to a
//! definition elsewhere, and a field or a local variable is not a definition a reader names.
//!
//! **Qualified names** are built outward-in: a definition's name is its parent's *child prefix*,
//! a separator the language picks for that parent and child, and its own name segment
//! ([`qualified_name`](super::qualified_name) holds the separators and the file-level prefix).
//! A parent's child prefix is usually its own qualified name; the exceptions are a Rust `impl`
//! (its methods are `Type::method` or `<Type as Trait>::method`, the impl itself has no path),
//! a Swift `extension` (the extended type), and a Python function (`outer.<locals>.inner`, as
//! `__qualname__` writes it, PEP 3155). Overloaded methods - two of one name in one scope, in a
//! language that allows it - get their parameter types, as Javadoc writes `parse(int)`; a name
//! without an overload stays bare, so a reader's `com.acme.Parser.parse` is what they wrote in
//! an import.
//!
//! **A definition's span covers what belongs to it**: a Python definition's decorators, and a
//! Rust item's attributes and `///` doc comments right before it, so `#[derive(Debug)]` added to
//! a struct modifies that struct.

use std::ops::Range;

use tree_sitter::Node;

use super::qualified_name::{self, FilePrefix};
use super::{Diff, Side, Span};
use crate::code::Language;

/// What kind of thing a [`Definition`] is.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DefinitionKind {
    /// A module, namespace or package declared in the file (Rust `mod`, C++ and C# `namespace`,
    /// TypeScript `namespace`, Ruby `module`).
    Module,
    /// A function not inside a type.
    Function,
    /// A function inside a class, struct, trait, interface or impl block, constructors included.
    Method,
    Struct,
    Enum,
    Union,
    /// A class, and the singletons some languages declare like one (Kotlin and Scala `object`).
    Class,
    /// An interface, a Swift protocol, a Java annotation type.
    Interface,
    /// A Rust or Scala trait, a PHP trait.
    Trait,
    /// A Rust `impl` block or a Swift `extension`: members added to a type declared elsewhere.
    Impl,
    /// A type alias or typedef.
    TypeAlias,
    /// A named constant.
    Constant,
    /// A global or static variable.
    Variable,
    /// A macro: Rust `macro_rules!`, a C `#define`.
    Macro,
    /// A definition of a kind no other variant names, such as an SQL table or view;
    /// [`Definition::grammar_kind`] says which.
    Other,
}

impl DefinitionKind {
    /// Functions and methods.
    pub fn is_function(self) -> bool {
        matches!(self, DefinitionKind::Function | DefinitionKind::Method)
    }

    /// Structs, enums, unions, classes, interfaces, traits and type aliases.
    pub fn is_type(self) -> bool {
        matches!(
            self,
            DefinitionKind::Struct
                | DefinitionKind::Enum
                | DefinitionKind::Union
                | DefinitionKind::Class
                | DefinitionKind::Interface
                | DefinitionKind::Trait
                | DefinitionKind::TypeAlias
        )
    }

    /// Whether functions directly inside it are methods.
    fn holds_methods(self) -> bool {
        matches!(
            self,
            DefinitionKind::Struct
                | DefinitionKind::Enum
                | DefinitionKind::Union
                | DefinitionKind::Class
                | DefinitionKind::Interface
                | DefinitionKind::Trait
                | DefinitionKind::Impl
        )
    }

    /// Kinds that pair with each other across the sides: a struct that became a class is the
    /// same type, a function that moved into a class became a method.
    pub(crate) fn family(self) -> u8 {
        if self.is_function() {
            0
        } else if self.is_type() {
            1
        } else {
            match self {
                DefinitionKind::Module => 2,
                DefinitionKind::Impl => 3,
                DefinitionKind::Constant | DefinitionKind::Variable => 4,
                DefinitionKind::Macro => 5,
                _ => 6,
            }
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            DefinitionKind::Module => "module",
            DefinitionKind::Function => "function",
            DefinitionKind::Method => "method",
            DefinitionKind::Struct => "struct",
            DefinitionKind::Enum => "enum",
            DefinitionKind::Union => "union",
            DefinitionKind::Class => "class",
            DefinitionKind::Interface => "interface",
            DefinitionKind::Trait => "trait",
            DefinitionKind::Impl => "impl",
            DefinitionKind::TypeAlias => "type alias",
            DefinitionKind::Constant => "constant",
            DefinitionKind::Variable => "variable",
            DefinitionKind::Macro => "macro",
            DefinitionKind::Other => "other",
        }
    }
}

impl std::fmt::Display for DefinitionKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A function, type, module or other named definition in one side of a diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Definition {
    kind: DefinitionKind,
    grammar_kind: &'static str,
    name: String,
    qualified_name: String,
    /// What a definition directly inside this one starts its qualified name with.
    child_prefix: String,
    span: Span,
    name_span: Span,
    depth: usize,
    /// Index of the enclosing definition in the same side's list.
    pub(crate) parent: Option<usize>,
    /// The tree-sitter id of the definition's node, which the engine's mapping is keyed by.
    pub(crate) node_id: usize,
    /// The node's own byte range, without the attributes or decorators [`Self::span`] adds.
    pub(crate) node_bytes: Range<usize>,
    /// The parameter types, for telling overloads apart.
    overload_key: Option<String>,
}

impl Definition {
    pub fn kind(&self) -> DefinitionKind {
        self.kind
    }

    /// The grammar's own name for the node: `function_item`, `class_declaration`.
    pub fn grammar_kind(&self) -> &'static str {
        self.grammar_kind
    }

    /// The definition's own name: `parse` in `crate::parser::Parser::parse`. A Rust impl block
    /// is named as rustdoc shows it, `impl Display for Parser`.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The name the language's imports and documentation use for it:
    /// `crate::parser::Parser::parse` (Rust), `pkg.parser.Parser.parse` (Python),
    /// `com.acme.Parser.parse` (Java, Kotlin, Scala), `acme::Parser::parse` (C++),
    /// `Acme\Parser::parse` (PHP), `example.com/acme/parser.Parser.Parse` (Go),
    /// `Acme::Parser#parse` (Ruby). The part from the file's location (a Rust crate path, a
    /// Python package, a Go import path) is only there when the diff knows where the file sits
    /// ([`Diff::with_paths`](super::Diff::with_paths)); the rest always is.
    pub fn qualified_name(&self) -> &str {
        &self.qualified_name
    }

    /// Where the definition is, attributes, decorators and doc comments included.
    pub fn span(&self) -> &Span {
        &self.span
    }

    /// Where its name is.
    pub fn name_span(&self) -> &Span {
        &self.name_span
    }

    /// How many definitions it is inside: 0 for one directly in the file.
    pub fn depth(&self) -> usize {
        self.depth
    }

    /// Tells an overload from its namesakes: `parse` becomes `parse(int)`.
    pub(crate) fn append_overload_key(&mut self) {
        if let Some(key) = &self.overload_key {
            self.qualified_name.push_str(key);
            self.child_prefix = self.qualified_name.clone();
        }
    }
}

/// What happened to a definition between the two sides.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChangeKind {
    Added,
    Removed,
    /// Its name changed (and perhaps more: see [`DefinitionChange::is_modified`]).
    Renamed,
    /// It now sits in another definition, or somewhere else in the file.
    Moved,
    /// Its text changed.
    Modified,
    Unchanged,
}

impl ChangeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ChangeKind::Added => "added",
            ChangeKind::Removed => "removed",
            ChangeKind::Renamed => "renamed",
            ChangeKind::Moved => "moved",
            ChangeKind::Modified => "modified",
            ChangeKind::Unchanged => "unchanged",
        }
    }
}

impl std::fmt::Display for ChangeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One definition across the two sides: present in both (paired by the node mapping), or only in
/// one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionChange {
    pub(crate) before: Option<Definition>,
    pub(crate) after: Option<Definition>,
    pub(crate) renamed: bool,
    pub(crate) moved: bool,
    pub(crate) modified: bool,
    pub(crate) cosmetic_only: bool,
}

impl DefinitionChange {
    /// What happened to it. A definition both renamed and changed is `Renamed`, one both moved
    /// and changed `Moved`; [`Self::is_modified`] and the other flags tell every combination.
    pub fn kind(&self) -> ChangeKind {
        match (&self.before, &self.after) {
            (None, Some(_)) => ChangeKind::Added,
            (Some(_), None) => ChangeKind::Removed,
            _ if self.renamed => ChangeKind::Renamed,
            _ if self.moved => ChangeKind::Moved,
            _ if self.modified => ChangeKind::Modified,
            _ => ChangeKind::Unchanged,
        }
    }

    /// The definition in the before side; `None` if it was added.
    pub fn before(&self) -> Option<&Definition> {
        self.before.as_ref()
    }

    /// The definition in the after side; `None` if it was removed.
    pub fn after(&self) -> Option<&Definition> {
        self.after.as_ref()
    }

    /// The after side's definition, or the before side's for a removed one.
    pub fn definition(&self) -> &Definition {
        self.after
            .as_ref()
            .or(self.before.as_ref())
            .expect("a change has at least one side")
    }

    /// [`Definition::qualified_name`] of [`Self::definition`]: the new name of a renamed one.
    pub fn qualified_name(&self) -> &str {
        self.definition().qualified_name()
    }

    pub fn definition_kind(&self) -> DefinitionKind {
        self.definition().kind()
    }

    /// Directly in the file, not inside another definition.
    pub fn is_top_level(&self) -> bool {
        self.definition().depth() == 0
    }

    pub fn is_renamed(&self) -> bool {
        self.renamed
    }

    pub fn is_moved(&self) -> bool {
        self.moved
    }

    /// Whether its text changed, beyond the move or rename itself.
    pub fn is_modified(&self) -> bool {
        self.modified
    }

    /// Whether everything that changed inside it is comments or whitespace.
    pub fn is_cosmetic_only(&self) -> bool {
        self.cosmetic_only
    }
}

/// Both sides' definitions and the changes between them, built once per [`Diff`].
#[derive(Debug, Default)]
pub(crate) struct Definitions {
    pub(crate) sides: [Vec<Definition>; 2],
    pub(crate) changes: Vec<DefinitionChange>,
}

impl Definitions {
    pub(crate) fn build(diff: &Diff) -> Self {
        if !diff.is_structural() {
            return Self::default();
        }
        crate::diff::on_pipeline_stack(|| {
            let sides = [Side::Before, Side::After].map(|side| {
                let code = diff.code(side);
                let prefix = qualified_name::file_prefix(
                    diff.path(side),
                    code.metadata.language.unwrap_or(Language::Unknown),
                    &code.contents,
                    diff.project_files(side),
                );
                extract(code, &prefix)
            });
            let changes = super::changes::pair(diff, &sides);
            Self { sides, changes }
        })
    }
}

/// Every node kind [`classify`] or [`prefix_statement`] has an arm for, in any language. A node of
/// any other kind is skipped by its kind id, without the string comparisons of their `match`es,
/// which were most of the extraction's time. The test `the_kind_filter_drops_no_definition`
/// checks that this list misses no arm.
const DEFINITION_KINDS: &[&str] = &[
    "abstract_class_declaration",
    "abstract_method_signature",
    "alias_declaration",
    "annotation_type_declaration",
    "binary_operator",
    "class",
    "class_declaration",
    "class_definition",
    "class_specifier",
    "companion_object",
    "const_item",
    "const_spec",
    "constructor_declaration",
    "create_function",
    "create_index",
    "create_materialized_view",
    "create_sequence",
    "create_table",
    "create_trigger",
    "create_type",
    "create_view",
    "delegate_declaration",
    "destructor_declaration",
    "enum_declaration",
    "enum_definition",
    "enum_item",
    "enum_specifier",
    "file_scoped_namespace_declaration",
    "function_declaration",
    "function_definition",
    "function_item",
    "function_signature",
    "function_signature_item",
    "generator_function_declaration",
    "impl_item",
    "init_declaration",
    "interface_declaration",
    "internal_module",
    "macro_definition",
    "method",
    "method_declaration",
    "method_declaration_statement",
    "method_definition",
    "method_signature",
    "mod_item",
    "module",
    "namespace_declaration",
    "namespace_definition",
    "object_declaration",
    "object_definition",
    "package_clause",
    "package_declaration",
    "package_header",
    "package_statement",
    "preproc_def",
    "preproc_function_def",
    "protocol_declaration",
    "protocol_function_declaration",
    "record_declaration",
    "singleton_method",
    "static_item",
    "struct_declaration",
    "struct_item",
    "struct_specifier",
    "subroutine_declaration_statement",
    "trait_declaration",
    "trait_definition",
    "trait_item",
    "type_alias",
    "type_alias_declaration",
    "type_definition",
    "type_item",
    "type_spec",
    "typealias_declaration",
    "union_item",
    "union_specifier",
    "var_spec",
    "variable_declarator",
];

/// Every definition in `code`, outermost first in document order.
pub(crate) fn extract(code: &crate::code::Code, prefix: &FilePrefix) -> Vec<Definition> {
    extract_filtered(code, prefix, true)
}

/// [`extract`], with the [`DEFINITION_KINDS`] filter switchable for the test that checks it.
fn extract_filtered(
    code: &crate::code::Code,
    prefix: &FilePrefix,
    filtered: bool,
) -> Vec<Definition> {
    let (Some(tree), Some(language)) = (code.ast.as_ref(), code.metadata.language) else {
        return Vec::new();
    };
    let source = code.contents.as_bytes();
    let lines = LineIndex::new(&code.contents);
    let grammar = tree.language();
    let wanted: std::collections::HashSet<&str> = DEFINITION_KINDS.iter().copied().collect();
    // Indexed by kind id. Several ids can share a name (aliases), so every id is looked at.
    let candidate: Vec<bool> = (0..grammar.node_kind_count() as u16)
        .map(|id| {
            !filtered
                || grammar
                    .node_kind_for_id(id)
                    .is_some_and(|k| wanted.contains(k))
        })
        .collect();
    let mut out: Vec<Definition> = Vec::new();
    // Definitions still open at the cursor, innermost last, as indices into `out`.
    let mut open: Vec<usize> = Vec::new();
    // A file-level prefix a statement sets for what follows it (a C# file-scoped namespace, a
    // Perl or PHP `package`/`namespace` statement), on top of `prefix`.
    let mut statement_prefix: Option<String> = None;

    let mut cursor = tree.walk();
    'walk: loop {
        let node = cursor.node();
        while open
            .last()
            .is_some_and(|&i| node.start_byte() >= out[i].node_bytes.end)
        {
            open.pop();
        }

        if !candidate
            .get(usize::from(node.kind_id()))
            .copied()
            .unwrap_or(true)
        {
            // Not a kind any definition has.
        } else if let Some(set) = prefix_statement(node, language, source) {
            statement_prefix = Some(set);
        } else if let Some(found) = classify(node, language, source) {
            let parent = open.last().copied();
            let parent_def = parent.map(|i| &out[i]);
            let kind = match (found.kind, parent_def) {
                (DefinitionKind::Function, Some(p))
                    if p.kind.holds_methods()
                        // A Ruby module's methods are mixed into classes.
                        || (language == Language::Ruby && p.kind == DefinitionKind::Module) =>
                {
                    DefinitionKind::Method
                }
                (kind, _) => kind,
            };
            let base = match parent_def {
                Some(p) => p.child_prefix.clone(),
                None => {
                    qualified_name::join_file_prefix(prefix, statement_prefix.as_deref(), language)
                }
            };
            let separator =
                qualified_name::separator(language, parent_def.map(|p| p.kind), kind, &found);
            let mut qualified = qualified_name::join(&base, separator, &found.segment);
            let child_prefix = match &found.child_prefix {
                ChildPrefix::Own => qualified.clone(),
                ChildPrefix::Locals => format!("{qualified}.<locals>"),
                // An impl or extension block has no path of its own: its members are the type's,
                // and the block is named as rustdoc and Swift's documentation show it, with the
                // type's full path (`qualified` is that path here).
                ChildPrefix::Type { template, display } => {
                    let prefix = template.replace(TYPE, &qualified);
                    qualified = display.replace(TYPE, &qualified);
                    prefix
                }
            };
            let span_bytes = extent(node, language, source);
            let depth = open.len();
            out.push(Definition {
                kind,
                grammar_kind: node.kind(),
                name: found.name,
                qualified_name: qualified,
                child_prefix,
                span: lines.span(span_bytes),
                name_span: lines.span(found.name_bytes),
                depth,
                parent,
                node_id: node.id(),
                node_bytes: node.byte_range(),
                overload_key: found.overload_key,
            });
            open.push(out.len() - 1);
        }

        if cursor.goto_first_child() {
            continue;
        }
        loop {
            if cursor.goto_next_sibling() {
                continue 'walk;
            }
            if !cursor.goto_parent() {
                break 'walk;
            }
        }
    }
    if language == Language::CPP {
        mark_out_of_line_methods(&mut out);
    }
    qualified_name::name_overloads(&mut out, language);
    out
}

/// A C++ function defined outside its class (`void Parser::parse() {}`) is a method when the
/// file defines a class of that qualified name.
fn mark_out_of_line_methods(definitions: &mut [Definition]) {
    let types: std::collections::HashSet<String> = definitions
        .iter()
        .filter(|d| d.kind.holds_methods())
        .map(|d| d.qualified_name.clone())
        .collect();
    for d in definitions
        .iter_mut()
        .filter(|d| d.kind == DefinitionKind::Function)
    {
        if let Some((scope, _)) = d.qualified_name.rsplit_once("::")
            && types.contains(scope)
        {
            d.kind = DefinitionKind::Method;
        }
    }
}

/// What [`classify`] knows about a definition node.
pub(crate) struct Found {
    kind: DefinitionKind,
    /// [`Definition::name`].
    name: String,
    name_bytes: Range<usize>,
    /// What it adds to its parent's prefix: usually its name; for a C++ out-of-line definition
    /// the whole qualified declarator (`inner::Parser::parse`), for Lua `M.parse` as written.
    pub(crate) segment: String,
    child_prefix: ChildPrefix,
    overload_key: Option<String>,
    /// A Ruby singleton method (`def self.make`), joined with `.` instead of `#`.
    pub(crate) singleton: bool,
}

enum ChildPrefix {
    /// Children continue the definition's own qualified name.
    Own,
    /// A function's: `outer.<locals>.inner`.
    Locals,
    /// An impl's or extension's, whose segment is the type: its children's prefix and its own
    /// qualified name are these templates, [`TYPE`] standing for the type's full path.
    Type { template: String, display: String },
}

/// Where [`ChildPrefix::Type`]'s templates put the type's full path.
const TYPE: &str = "{TYPE}";

impl Found {
    fn new(kind: DefinitionKind, name_node: Node, source: &[u8]) -> Option<Self> {
        let name = text(name_node, source)?;
        Some(Self::named(kind, name, name_node.byte_range()))
    }

    fn named(kind: DefinitionKind, name: String, name_bytes: Range<usize>) -> Self {
        Self {
            kind,
            segment: name.clone(),
            name,
            name_bytes,
            child_prefix: ChildPrefix::Own,
            overload_key: None,
            singleton: false,
        }
    }

    fn with_parameters(mut self, parameters: Option<Node>, source: &[u8]) -> Self {
        self.overload_key = parameters.map(|p| parameter_types(p, source));
        self
    }
}

fn text(node: Node, source: &[u8]) -> Option<String> {
    let text = node.utf8_text(source).ok()?.trim();
    (!text.is_empty()).then(|| collapse_whitespace(text))
}

fn collapse_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn field<'a>(node: Node<'a>, name: &str) -> Option<Node<'a>> {
    node.child_by_field_name(name)
}

fn named<'a>(
    kind: DefinitionKind,
    node: Node<'a>,
    name_field: &str,
    source: &[u8],
) -> Option<Found> {
    Found::new(kind, field(node, name_field)?, source)
}

/// Whether `node` has an anonymous child token spelled `keyword`.
fn has_keyword(node: Node, keyword: &str) -> bool {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .any(|c| !c.is_named() && c.kind() == keyword)
}

fn first_named_child_of_kind<'a>(node: Node<'a>, kinds: &[&str]) -> Option<Node<'a>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .find(|c| kinds.contains(&c.kind()))
}

/// The parameter types of a parameter list, `(int, String)`, for overloads. Each parameter is its
/// `type` field where it has one, else its whole text without its name.
fn parameter_types(list: Node, source: &[u8]) -> String {
    let mut cursor = list.walk();
    let types: Vec<String> = list
        .named_children(&mut cursor)
        .filter(|p| !crate::diff::nodes::is_comment(p.kind()))
        .filter_map(|p| {
            let ty = field(p, "type").and_then(|t| text(t, source));
            let ty = ty.or_else(|| text(p, source))?;
            // A C/C++ declarator's pointer or reference belongs to the type.
            let marker = field(p, "declarator").map_or("", |d| match d.kind() {
                "pointer_declarator" | "abstract_pointer_declarator" => "*",
                "reference_declarator" | "abstract_reference_declarator" => "&",
                _ => "",
            });
            Some(format!("{ty}{marker}"))
        })
        .collect();
    format!("({})", types.join(", "))
}

/// The name in a C or C++ declarator chain: `*make(void)` -> `make`, `inner::Parser::parse` ->
/// the qualified identifier, as the node that holds it.
fn declarator_name(mut node: Node) -> Option<Node> {
    loop {
        match node.kind() {
            "identifier"
            | "field_identifier"
            | "qualified_identifier"
            | "destructor_name"
            | "operator_name"
            | "type_identifier" => return Some(node),
            _ => node = field(node, "declarator")?,
        }
    }
}

/// The base name of a type as written, without generic arguments or a pointer: `Box[T]`,
/// `*Box[T]` and `Box<T>` are all `Box`; a path keeps its path, `std::fmt::Display`.
fn type_base(node: Node, source: &[u8]) -> Option<String> {
    let mut node = node;
    while matches!(
        node.kind(),
        "generic_type" | "pointer_type" | "reference_type" | "generic_name"
    ) {
        node = field(node, "type")
            .or_else(|| node.named_child(0))
            .filter(|n| n.id() != node.id())?;
    }
    let full = text(node, source)?;
    let base = full
        .split(['<', '['])
        .next()?
        .trim()
        .trim_start_matches(['*', '&']);
    Some(base.trim().to_string())
}

/// A statement that sets the file-level prefix for what follows it, and that prefix: a C#
/// file-scoped namespace, a PHP `namespace X;`, a Perl `package X;`, and the package declaration
/// of Java, Kotlin, Scala and Go.
fn prefix_statement(node: Node, language: Language, source: &[u8]) -> Option<String> {
    match (language, node.kind()) {
        (Language::CSharp, "file_scoped_namespace_declaration") => {
            text(field(node, "name")?, source)
        }
        (Language::PHP, "namespace_definition") if field(node, "body").is_none() => {
            text(field(node, "name")?, source)
        }
        (Language::Perl, "package_statement") => text(field(node, "name")?, source),
        (Language::Java, "package_declaration") => text(
            first_named_child_of_kind(node, &["scoped_identifier", "identifier"])?,
            source,
        ),
        (Language::Kotlin, "package_header") => text(node.named_child(0)?, source),
        (Language::Scala, "package_clause") => text(field(node, "name")?, source),
        (Language::Go, "package_clause") => text(node.named_child(0)?, source),
        _ => None,
    }
}

/// Whether `node` is a definition, and what it is.
fn classify(node: Node, language: Language, source: &[u8]) -> Option<Found> {
    use DefinitionKind as K;
    let kind = node.kind();
    match language {
        Language::Rust => match kind {
            "mod_item" => named(K::Module, node, "name", source),
            "function_item" | "function_signature_item" => named(K::Function, node, "name", source),
            "struct_item" => named(K::Struct, node, "name", source),
            "enum_item" => named(K::Enum, node, "name", source),
            "union_item" => named(K::Union, node, "name", source),
            "trait_item" => named(K::Trait, node, "name", source),
            "type_item" => named(K::TypeAlias, node, "name", source),
            "const_item" => named(K::Constant, node, "name", source),
            "static_item" => named(K::Variable, node, "name", source),
            "macro_definition" => named(K::Macro, node, "name", source),
            "impl_item" => {
                let ty_node = field(node, "type")?;
                let ty = type_base(ty_node, source)?;
                let trait_ = field(node, "trait").and_then(|t| type_base(t, source));
                let name = match &trait_ {
                    Some(t) => format!("impl {t} for {ty}"),
                    None => format!("impl {ty}"),
                };
                let mut found = Found::named(K::Impl, name, ty_node.byte_range());
                found.segment = ty;
                found.child_prefix = match &trait_ {
                    Some(t) => ChildPrefix::Type {
                        template: format!("<{TYPE} as {t}>"),
                        display: format!("impl {t} for {TYPE}"),
                    },
                    None => ChildPrefix::Type {
                        template: TYPE.to_string(),
                        display: format!("impl {TYPE}"),
                    },
                };
                Some(found)
            }
            _ => None,
        },
        Language::Python => match kind {
            "function_definition" => {
                let mut found = named(K::Function, node, "name", source)?;
                found.child_prefix = ChildPrefix::Locals;
                Some(found)
            }
            "class_definition" => named(K::Class, node, "name", source),
            _ => None,
        },
        Language::Go => match kind {
            "function_declaration" => named(K::Function, node, "name", source),
            "method_declaration" => {
                let receiver = field(node, "receiver")?;
                let parameter = first_named_child_of_kind(receiver, &["parameter_declaration"])?;
                let ty = type_base(field(parameter, "type")?, source)?;
                let mut found = named(K::Method, node, "name", source)?;
                found.segment = format!("{ty}.{}", found.name);
                Some(found)
            }
            "type_spec" => {
                let ty = field(node, "type")?;
                let kind = match ty.kind() {
                    "struct_type" => K::Struct,
                    "interface_type" => K::Interface,
                    _ => K::TypeAlias,
                };
                named(kind, node, "name", source)
            }
            "type_alias" => named(K::TypeAlias, node, "name", source),
            // Package-level only: a `const` or `var` inside a function is a local.
            "const_spec" | "var_spec" if is_top_level_go(node) => {
                let kind = if kind == "const_spec" {
                    K::Constant
                } else {
                    K::Variable
                };
                named(kind, node, "name", source)
            }
            _ => None,
        },
        Language::Java => match kind {
            "class_declaration" | "record_declaration" => named(K::Class, node, "name", source),
            "interface_declaration" | "annotation_type_declaration" => {
                named(K::Interface, node, "name", source)
            }
            "enum_declaration" => named(K::Enum, node, "name", source),
            "method_declaration" | "constructor_declaration" => Some(
                named(K::Method, node, "name", source)?
                    .with_parameters(field(node, "parameters"), source),
            ),
            _ => None,
        },
        Language::Kotlin => match kind {
            "class_declaration" => {
                let kind = if has_keyword(node, "interface") {
                    K::Interface
                } else if first_named_child_of_kind(node, &["enum_class_body"]).is_some() {
                    K::Enum
                } else {
                    K::Class
                };
                named(kind, node, "name", source)
            }
            "object_declaration" => named(K::Class, node, "name", source),
            "companion_object" => Some(match field(node, "name") {
                Some(name) => Found::new(K::Class, name, source)?,
                // Kotlin's own name for an unnamed companion.
                None => Found::named(
                    K::Class,
                    "Companion".to_string(),
                    node.start_byte()..node.start_byte(),
                ),
            }),
            "function_declaration" => {
                let parameters = first_named_child_of_kind(node, &["function_value_parameters"]);
                Some(named(K::Function, node, "name", source)?.with_parameters(parameters, source))
            }
            // The grammar puts a type alias's name in its `type` field.
            "type_alias" => named(K::TypeAlias, node, "type", source),
            _ => None,
        },
        Language::Scala => match kind {
            "class_definition" | "object_definition" => named(K::Class, node, "name", source),
            "trait_definition" => named(K::Trait, node, "name", source),
            "enum_definition" => named(K::Enum, node, "name", source),
            "function_definition" | "function_declaration" => {
                let parameters = field(node, "parameters");
                Some(named(K::Function, node, "name", source)?.with_parameters(parameters, source))
            }
            "type_definition" => named(K::TypeAlias, node, "name", source),
            _ => None,
        },
        Language::CSharp => match kind {
            "namespace_declaration" => named(K::Module, node, "name", source),
            "class_declaration" | "record_declaration" => named(K::Class, node, "name", source),
            "struct_declaration" => named(K::Struct, node, "name", source),
            "interface_declaration" => named(K::Interface, node, "name", source),
            "enum_declaration" => named(K::Enum, node, "name", source),
            "delegate_declaration" => named(K::TypeAlias, node, "name", source),
            "method_declaration" | "constructor_declaration" | "destructor_declaration" => Some(
                named(K::Method, node, "name", source)?
                    .with_parameters(field(node, "parameters"), source),
            ),
            _ => None,
        },
        Language::C | Language::CPP => match kind {
            "function_definition" => {
                let declarator = field(node, "declarator")?;
                let name_node = declarator_name(declarator)?;
                let full = text(name_node, source)?;
                let short = full.rsplit("::").next()?.to_string();
                let parameters = function_parameters(declarator);
                let mut found = Found::named(K::Function, short, name_node.byte_range())
                    .with_parameters(parameters, source);
                found.segment = full;
                Some(found)
            }
            "struct_specifier" | "union_specifier" | "enum_specifier" | "class_specifier"
                if field(node, "body").is_some() =>
            {
                let kind = match kind {
                    "struct_specifier" => K::Struct,
                    "union_specifier" => K::Union,
                    "enum_specifier" => K::Enum,
                    _ => K::Class,
                };
                named(kind, node, "name", source)
            }
            "type_definition" => {
                let name = declarator_name(field(node, "declarator")?)?;
                Found::new(K::TypeAlias, name, source)
            }
            "alias_declaration" => named(K::TypeAlias, node, "name", source),
            "namespace_definition" => named(K::Module, node, "name", source),
            "preproc_def" | "preproc_function_def" => named(K::Macro, node, "name", source),
            _ => None,
        },
        Language::JavaScript | Language::TypeScript | Language::TSX => match kind {
            "function_declaration" | "generator_function_declaration" | "function_signature" => {
                named(K::Function, node, "name", source)
            }
            "class_declaration" | "abstract_class_declaration" | "class" => {
                named(K::Class, node, "name", source)
            }
            "method_definition" | "method_signature" | "abstract_method_signature"
                if node.parent().is_some_and(|p| {
                    matches!(p.kind(), "class_body" | "interface_body" | "object_type")
                }) =>
            {
                named(K::Method, node, "name", source)
            }
            "variable_declarator"
                if field(node, "value").is_some_and(|v| {
                    matches!(
                        v.kind(),
                        "arrow_function" | "function_expression" | "function"
                    )
                }) =>
            {
                named(K::Function, node, "name", source)
            }
            "interface_declaration" => named(K::Interface, node, "name", source),
            "type_alias_declaration" => named(K::TypeAlias, node, "name", source),
            "enum_declaration" => named(K::Enum, node, "name", source),
            "internal_module" | "module" => {
                let name = field(node, "name")?;
                let raw = text(name, source)?;
                Some(Found::named(
                    K::Module,
                    raw.trim_matches(['"', '\'']).to_string(),
                    name.byte_range(),
                ))
            }
            _ => None,
        },
        Language::PHP => match kind {
            "namespace_definition" if field(node, "body").is_some() => {
                named(K::Module, node, "name", source)
            }
            "class_declaration" => named(K::Class, node, "name", source),
            "interface_declaration" => named(K::Interface, node, "name", source),
            "trait_declaration" => named(K::Trait, node, "name", source),
            "enum_declaration" => named(K::Enum, node, "name", source),
            "function_definition" => named(K::Function, node, "name", source),
            "method_declaration" => named(K::Method, node, "name", source),
            _ => None,
        },
        Language::Ruby => match kind {
            "module" => named(K::Module, node, "name", source),
            "class" => named(K::Class, node, "name", source),
            "method" => {
                let mut found = named(K::Function, node, "name", source)?;
                // `class << self ... def other` is a singleton method too.
                found.singleton = node
                    .parent()
                    .and_then(|p| p.parent())
                    .is_some_and(|gp| gp.kind() == "singleton_class");
                Some(found)
            }
            "singleton_method" => {
                let mut found = named(K::Function, node, "name", source)?;
                found.singleton = true;
                Some(found)
            }
            _ => None,
        },
        Language::Swift => match kind {
            "class_declaration" => {
                let declared = field(node, "declaration_kind").map(|k| k.kind());
                let name = field(node, "name")?;
                match declared {
                    Some("extension") => {
                        let ty = type_base(name, source)?;
                        let mut found =
                            Found::named(K::Impl, format!("extension {ty}"), name.byte_range());
                        found.segment = ty;
                        found.child_prefix = ChildPrefix::Type {
                            template: TYPE.to_string(),
                            display: format!("extension {TYPE}"),
                        };
                        Some(found)
                    }
                    _ => {
                        let kind = match declared {
                            Some("struct") => K::Struct,
                            Some("enum") => K::Enum,
                            _ => K::Class,
                        };
                        Found::new(kind, name, source)
                    }
                }
            }
            "protocol_declaration" => named(K::Interface, node, "name", source),
            "function_declaration" | "protocol_function_declaration" => {
                let mut found = named(K::Function, node, "name", source)?;
                found.overload_key = Some(swift_labels(node, source));
                Some(found)
            }
            "init_declaration" => {
                let mut found = Found::named(
                    K::Method,
                    "init".to_string(),
                    node.start_byte()..node.start_byte() + 4,
                );
                found.overload_key = Some(swift_labels(node, source));
                Some(found)
            }
            "typealias_declaration" => named(K::TypeAlias, node, "name", source),
            _ => None,
        },
        Language::LUA => match kind {
            // `function M.parse()` and `function M:method()` are named as written.
            "function_declaration" => {
                let name = field(node, "name")?;
                let full = text(name, source)?;
                let short = full.rsplit(['.', ':']).next()?.to_string();
                let mut found = Found::named(K::Function, short, name.byte_range());
                found.segment = full;
                Some(found)
            }
            _ => None,
        },
        Language::ShellScript => match kind {
            "function_definition" => named(K::Function, node, "name", source),
            _ => None,
        },
        Language::Perl => match kind {
            "subroutine_declaration_statement" | "method_declaration_statement" => {
                named(K::Function, node, "name", source)
            }
            _ => None,
        },
        Language::R => match kind {
            "binary_operator"
                if field(node, "rhs").is_some_and(|r| r.kind() == "function_definition")
                    && field(node, "lhs").is_some_and(|l| l.kind() == "identifier") =>
            {
                named(K::Function, node, "lhs", source)
            }
            _ => None,
        },
        Language::Vimscript => match kind {
            "function_definition" => {
                let declaration = first_named_child_of_kind(node, &["function_declaration"])?;
                named(K::Function, declaration, "name", source)
            }
            _ => None,
        },
        Language::SQL => match kind {
            "create_table"
            | "create_view"
            | "create_materialized_view"
            | "create_function"
            | "create_type"
            | "create_index"
            | "create_trigger"
            | "create_sequence" => {
                let reference = first_named_child_of_kind(node, &["object_reference"])?;
                Found::new(K::Other, reference, source)
            }
            _ => None,
        },
        _ => None,
    }
}

fn is_top_level_go(node: Node) -> bool {
    let mut current = node.parent();
    while let Some(n) = current {
        match n.kind() {
            "source_file" => return true,
            "function_declaration" | "method_declaration" | "func_literal" | "block" => {
                return false;
            }
            _ => current = n.parent(),
        }
    }
    false
}

/// The parameter list of a C/C++ function declarator.
fn function_parameters(mut declarator: Node) -> Option<Node> {
    loop {
        if declarator.kind() == "function_declarator" {
            return field(declarator, "parameters");
        }
        declarator = field(declarator, "declarator")?;
    }
}

/// A Swift function's argument labels, `(_:with:)`, Swift's own way to tell overloads apart.
fn swift_labels(node: Node, source: &[u8]) -> String {
    let mut cursor = node.walk();
    let labels: Vec<String> = node
        .named_children(&mut cursor)
        .filter(|c| c.kind() == "parameter")
        .filter_map(|p| {
            let label = field(p, "external_name").or_else(|| field(p, "name"))?;
            Some(format!("{}:", text(label, source)?))
        })
        .collect();
    format!("({})", labels.join(""))
}

/// The bytes a definition covers: its node, plus a Python definition's decorators and a Rust
/// item's attributes and doc comments directly before it.
fn extent(node: Node, language: Language, source: &[u8]) -> Range<usize> {
    let mut start = node.start_byte();
    match language {
        Language::Python => {
            if let Some(parent) = node.parent().filter(|p| p.kind() == "decorated_definition") {
                start = parent.start_byte();
            }
        }
        Language::Rust => {
            let mut previous = node.prev_sibling();
            while let Some(p) = previous {
                let is_doc = matches!(p.kind(), "line_comment" | "block_comment")
                    && p.utf8_text(source)
                        .is_ok_and(|t| t.starts_with("///") || t.starts_with("/**"));
                if p.kind() != "attribute_item" && !is_doc {
                    break;
                }
                start = p.start_byte();
                previous = p.prev_sibling();
            }
        }
        _ => {}
    }
    start..node.end_byte()
}

/// Row starts of a text, for turning byte ranges into [`Span`]s by binary search.
pub(crate) struct LineIndex {
    row_starts: Vec<usize>,
}

impl LineIndex {
    pub(crate) fn new(text: &str) -> Self {
        let mut row_starts = vec![0];
        row_starts.extend(
            text.bytes()
                .enumerate()
                .filter(|(_, b)| *b == b'\n')
                .map(|(i, _)| i + 1),
        );
        Self { row_starts }
    }

    fn position(&self, offset: usize) -> super::Position {
        let row = self.row_starts.partition_point(|&start| start <= offset) - 1;
        super::Position {
            row,
            column: offset - self.row_starts[row],
        }
    }

    pub(crate) fn span(&self, bytes: Range<usize>) -> Span {
        Span {
            start: self.position(bytes.start),
            end: self.position(bytes.end),
            bytes,
        }
    }
}

#[cfg(test)]
mod tests;
