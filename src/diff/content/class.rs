/*  This file is part of the OmniDiff code diffing tool.
 *
 *  Copyright (C) 2026 Marko Ivankovic
 *
 *  This program is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU Affero General Public License as published
 *  by the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 *  GNU Affero General Public License for more details.
 *
 *  You should have received a copy of the GNU Affero General Public License
 *  along with this program. If not, see <https://www.gnu.org/licenses/>.
 */

//! Java class files as text a reader can diff: what `javap` shows, and what each method reaches.
//!
//! Jars are most of the archives commits change, and a jar is mostly `.class` files, which as
//! bytes say nothing. [`listing`] turns one into lines: the class, its superclass and interfaces,
//! each field, and each method with its signature and code size, followed by what its bytecode
//! refers to - the methods it calls, the fields it reads and writes, the classes it creates or
//! checks, the constants it loads - each once, in the order the code first reaches it. Two
//! listings diff as text, so a changed method shows as the lines that changed under it.
//!
//! The bytecode itself is not listed: an instruction's operands index the constant pool, whose
//! numbering shifts whenever any constant is added, so the same code compiles to different bytes.
//! What it refers to does not shift. A change only in arithmetic or control flow, with no new
//! reference, shows as a changed code size, or not at all when the size is the same.

use std::collections::HashSet;
use std::fmt::Write;

/// True if `bytes` are a Java class file: `CAFEBABE`, then a class file's major version.
/// Mach-O universal binaries start `CAFEBABE` too, with a small architecture count where a class
/// file has its version.
pub fn is_class(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0xCA, 0xFE, 0xBA, 0xBE])
        && bytes
            .get(6..8)
            .map(|major| u16::from_be_bytes([major[0], major[1]]))
            .is_some_and(|major| (45..=100).contains(&major))
}

/// One constant pool entry, as far as a listing needs it.
#[derive(Debug, Clone)]
enum Constant {
    Utf8(String),
    Integer(i32),
    Float(f32),
    Long(i64),
    Double(f64),
    Class(u16),
    String(u16),
    /// A field, method or interface method reference: its class and its name-and-type.
    Member(u16, u16),
    NameAndType(u16, u16),
    MethodType(u16),
    /// An `invokedynamic` or dynamic constant: its name-and-type (the bootstrap is not listed).
    Dynamic(u16),
    /// What a listing never names: a method handle, a module, a package, the unused slot after
    /// a long or a double.
    Other,
}

/// A cursor over a class file's big-endian bytes.
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let slice = self.bytes.get(self.at..self.at + n)?;
        self.at += n;
        Some(slice)
    }
    fn u1(&mut self) -> Option<u8> {
        Some(self.take(1)?[0])
    }
    fn u2(&mut self) -> Option<u16> {
        let b = self.take(2)?;
        Some(u16::from_be_bytes([b[0], b[1]]))
    }
    fn u4(&mut self) -> Option<u32> {
        let b = self.take(4)?;
        Some(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }
}

struct Pool(Vec<Constant>);

impl Pool {
    fn utf8(&self, index: u16) -> String {
        match self.0.get(usize::from(index)) {
            Some(Constant::Utf8(text)) => text.clone(),
            _ => format!("#{index}"),
        }
    }

    /// A class constant's name, dotted (`java.lang.String`).
    fn class(&self, index: u16) -> String {
        match self.0.get(usize::from(index)) {
            Some(Constant::Class(name)) => self.utf8(*name).replace('/', "."),
            _ => format!("#{index}"),
        }
    }

    fn name_and_type(&self, index: u16) -> (String, String) {
        match self.0.get(usize::from(index)) {
            Some(Constant::NameAndType(name, descriptor)) => {
                (self.utf8(*name), self.utf8(*descriptor))
            }
            _ => (format!("#{index}"), String::new()),
        }
    }

    /// How a listing names constant `index`: a member as `Class.name(types)` or `Class.name: type`,
    /// a string quoted, a number as written.
    fn describe(&self, index: u16) -> String {
        match self.0.get(usize::from(index)) {
            Some(Constant::Member(class, name_and_type)) => {
                let (name, descriptor) = self.name_and_type(*name_and_type);
                let class = self.class(*class);
                if descriptor.starts_with('(') {
                    format!("{class}.{name}{}", method_types(&descriptor).0)
                } else {
                    format!("{class}.{name}: {}", field_type(&descriptor))
                }
            }
            Some(Constant::Class(_)) => self.class(index),
            Some(Constant::String(text)) => format!("{:?}", self.utf8(*text)),
            Some(Constant::Integer(value)) => value.to_string(),
            Some(Constant::Float(value)) => format!("{value}f"),
            Some(Constant::Long(value)) => format!("{value}L"),
            Some(Constant::Double(value)) => format!("{value}d"),
            Some(Constant::MethodType(descriptor)) => method_types(&self.utf8(*descriptor)).0,
            Some(Constant::Dynamic(name_and_type)) => {
                let (name, descriptor) = self.name_and_type(*name_and_type);
                format!("dynamic {name}{descriptor}")
            }
            _ => format!("#{index}"),
        }
    }
}

fn read_pool(reader: &mut Reader) -> Option<Pool> {
    let count = reader.u2()?;
    let mut pool = vec![Constant::Other];
    while pool.len() < usize::from(count) {
        let constant = match reader.u1()? {
            1 => {
                let length = reader.u2()?;
                // Modified UTF-8; read as UTF-8, the two differ only in NUL and astral characters.
                Constant::Utf8(String::from_utf8_lossy(reader.take(usize::from(length))?).into())
            }
            3 => Constant::Integer(reader.u4()? as i32),
            4 => Constant::Float(f32::from_bits(reader.u4()?)),
            5 | 6 => {
                let tag = reader.bytes[reader.at - 1];
                let bits = u64::from(reader.u4()?) << 32 | u64::from(reader.u4()?);
                pool.push(if tag == 5 {
                    Constant::Long(bits as i64)
                } else {
                    Constant::Double(f64::from_bits(bits))
                });
                // A long or a double takes two slots.
                Constant::Other
            }
            7 => Constant::Class(reader.u2()?),
            8 => Constant::String(reader.u2()?),
            9..=11 => Constant::Member(reader.u2()?, reader.u2()?),
            12 => Constant::NameAndType(reader.u2()?, reader.u2()?),
            15 => {
                reader.take(3)?;
                Constant::Other
            }
            16 => Constant::MethodType(reader.u2()?),
            17 | 18 => {
                reader.u2()?;
                Constant::Dynamic(reader.u2()?)
            }
            19 | 20 => {
                reader.u2()?;
                Constant::Other
            }
            _ => return None,
        };
        pool.push(constant);
    }
    Some(Pool(pool))
}

/// One field type of a descriptor, from `chars`: `I` as `int`, `[Ljava/lang/String;` as
/// `java.lang.String[]`.
fn next_type(chars: &mut std::iter::Peekable<std::str::Chars>) -> Option<String> {
    let mut dimensions = 0;
    while chars.peek() == Some(&'[') {
        chars.next();
        dimensions += 1;
    }
    let base = match chars.next()? {
        'B' => "byte".to_string(),
        'C' => "char".to_string(),
        'D' => "double".to_string(),
        'F' => "float".to_string(),
        'I' => "int".to_string(),
        'J' => "long".to_string(),
        'S' => "short".to_string(),
        'Z' => "boolean".to_string(),
        'V' => "void".to_string(),
        'L' => chars
            .by_ref()
            .take_while(|&c| c != ';')
            .collect::<String>()
            .replace('/', "."),
        _ => return None,
    };
    Some(base + &"[]".repeat(dimensions))
}

fn field_type(descriptor: &str) -> String {
    next_type(&mut descriptor.chars().peekable()).unwrap_or_else(|| descriptor.to_string())
}

/// A method descriptor as `(int, java.lang.String)` and its return type.
fn method_types(descriptor: &str) -> (String, String) {
    let mut chars = descriptor.chars().peekable();
    if chars.next() != Some('(') {
        return (descriptor.to_string(), String::new());
    }
    let mut parameters = Vec::new();
    while chars.peek().is_some_and(|&c| c != ')') {
        match next_type(&mut chars) {
            Some(parameter) => parameters.push(parameter),
            None => return (descriptor.to_string(), String::new()),
        }
    }
    chars.next();
    let returns = next_type(&mut chars).unwrap_or_default();
    (format!("({})", parameters.join(", ")), returns)
}

/// The modifiers `flags` set, for a class (`class`), a field or a method.
fn modifiers(flags: u16, kind: &str) -> String {
    let mut words = Vec::new();
    let named: &[(u16, &str)] = match kind {
        "class" => &[
            (0x0001, "public"),
            (0x0010, "final"),
            (0x0200, "interface"),
            (0x0400, "abstract"),
            (0x1000, "synthetic"),
            (0x2000, "annotation"),
            (0x4000, "enum"),
            (0x8000, "module"),
        ],
        "field" => &[
            (0x0001, "public"),
            (0x0002, "private"),
            (0x0004, "protected"),
            (0x0008, "static"),
            (0x0010, "final"),
            (0x0040, "volatile"),
            (0x0080, "transient"),
            (0x1000, "synthetic"),
            (0x4000, "enum"),
        ],
        _ => &[
            (0x0001, "public"),
            (0x0002, "private"),
            (0x0004, "protected"),
            (0x0008, "static"),
            (0x0010, "final"),
            (0x0020, "synchronized"),
            (0x0040, "bridge"),
            (0x0080, "varargs"),
            (0x0100, "native"),
            (0x0400, "abstract"),
            (0x0800, "strict"),
            (0x1000, "synthetic"),
        ],
    };
    for (bit, word) in named {
        if flags & bit != 0 {
            words.push(*word);
        }
    }
    words.join(" ")
}

/// The length of the instruction at `pc` in `code`, operands included; `None` for an opcode a
/// class file cannot hold.
fn instruction_length(code: &[u8], pc: usize) -> Option<usize> {
    let opcode = *code.get(pc)?;
    let word = |at: usize| -> Option<i32> {
        Some(i32::from_be_bytes(code.get(at..at + 4)?.try_into().ok()?))
    };
    Some(match opcode {
        0x10 | 0x12 | 0x15..=0x19 | 0x36..=0x3a | 0xa9 | 0xbc => 2,
        0x11
        | 0x13
        | 0x14
        | 0x84
        | 0x99..=0xa8
        | 0xb2..=0xb8
        | 0xbb
        | 0xbd
        | 0xc0
        | 0xc1
        | 0xc6
        | 0xc7 => 3,
        0xc5 => 4,
        0xb9 | 0xba | 0xc8 | 0xc9 => 5,
        0xc4 => {
            if *code.get(pc + 1)? == 0x84 {
                6
            } else {
                4
            }
        }
        0xaa => {
            let start = pc + 1 + (4 - (pc + 1) % 4) % 4;
            let (low, high) = (word(start + 4)?, word(start + 8)?);
            let entries = usize::try_from(i64::from(high) - i64::from(low) + 1).ok()?;
            start - pc + 12 + entries * 4
        }
        0xab => {
            let start = pc + 1 + (4 - (pc + 1) % 4) % 4;
            let pairs = usize::try_from(word(start + 4)?).ok()?;
            start - pc + 8 + pairs * 8
        }
        0x00..=0x0f
        | 0x1a..=0x35
        | 0x3b..=0x83
        | 0x85..=0x98
        | 0xac..=0xb1
        | 0xbe
        | 0xbf
        | 0xc2
        | 0xc3
        | 0xca
        | 0xfe
        | 0xff => 1,
        _ => return None,
    })
}

/// What `code` refers to, each once, in the order the code first reaches it: `calls`, `reads`,
/// `writes`, `creates`, `checks`, `loads`.
fn references(code: &[u8], pool: &Pool) -> Vec<String> {
    let mut found = Vec::new();
    let mut seen = HashSet::new();
    let mut pc = 0;
    while pc < code.len() {
        let Some(length) = instruction_length(code, pc) else {
            found.push("(bytecode this listing cannot read)".to_string());
            break;
        };
        let u2 = || {
            code.get(pc + 1..pc + 3)
                .map(|b| u16::from_be_bytes([b[0], b[1]]))
        };
        let reference = match code[pc] {
            0x12 => code.get(pc + 1).map(|&index| ("loads", u16::from(index))),
            0x13 | 0x14 => u2().map(|index| ("loads", index)),
            0xb2 | 0xb4 => u2().map(|index| ("reads", index)),
            0xb3 | 0xb5 => u2().map(|index| ("writes", index)),
            0xb6..=0xb9 => u2().map(|index| ("calls", index)),
            0xba => u2().map(|index| ("calls", index)),
            0xbb | 0xbd | 0xc5 => u2().map(|index| ("creates", index)),
            0xc0 | 0xc1 => u2().map(|index| ("checks", index)),
            _ => None,
        };
        if let Some((verb, index)) = reference {
            let line = format!("{verb} {}", pool.describe(index));
            if seen.insert(line.clone()) {
                found.push(line);
            }
        }
        pc += length;
    }
    found
}

/// An attribute: its name and its bytes.
fn read_attributes<'a>(reader: &mut Reader<'a>, pool: &Pool) -> Option<Vec<(String, &'a [u8])>> {
    let count = reader.u2()?;
    let mut attributes = Vec::with_capacity(usize::from(count));
    for _ in 0..count {
        let name = pool.utf8(reader.u2()?);
        let length = reader.u4()? as usize;
        attributes.push((name, reader.take(length)?));
    }
    Some(attributes)
}

/// A class file as text (see the module doc); `None` if it does not read as one.
pub fn listing(bytes: &[u8]) -> Option<String> {
    if !is_class(bytes) {
        return None;
    }
    let mut reader = Reader { bytes, at: 4 };
    let (minor, major) = (reader.u2()?, reader.u2()?);
    let pool = read_pool(&mut reader)?;
    let access = reader.u2()?;
    let (this, superclass) = (reader.u2()?, reader.u2()?);
    let interfaces: Vec<String> = (0..reader.u2()?)
        .map(|_| reader.u2().map(|index| pool.class(index)))
        .collect::<Option<_>>()?;

    let mut out = String::new();
    let kind = if access & 0x0200 != 0 { "" } else { "class " };
    let _ = write!(
        out,
        "{} {kind}{}",
        modifiers(access, "class"),
        pool.class(this)
    );
    if superclass != 0 {
        let _ = write!(out, " extends {}", pool.class(superclass));
    }
    if !interfaces.is_empty() {
        let _ = write!(out, " implements {}", interfaces.join(", "));
    }
    let _ = writeln!(out, "\nclass file version {major}.{minor}");

    for _ in 0..reader.u2()? {
        let access = reader.u2()?;
        let (name, descriptor) = (pool.utf8(reader.u2()?), pool.utf8(reader.u2()?));
        read_attributes(&mut reader, &pool)?;
        let _ = writeln!(
            out,
            "\nfield {} {} {name}",
            modifiers(access, "field"),
            field_type(&descriptor)
        );
    }
    for _ in 0..reader.u2()? {
        let access = reader.u2()?;
        let (name, descriptor) = (pool.utf8(reader.u2()?), pool.utf8(reader.u2()?));
        let attributes = read_attributes(&mut reader, &pool)?;
        let (parameters, returns) = method_types(&descriptor);
        let _ = write!(
            out,
            "\nmethod {} {returns} {name}{parameters}",
            modifiers(access, "method")
        );
        match attributes.iter().find(|(name, _)| name == "Code") {
            Some((_, code)) => {
                let mut code_reader = Reader { bytes: code, at: 4 };
                let length = code_reader.u4()? as usize;
                let body = code_reader.take(length)?;
                let _ = writeln!(out, "  [{length} bytes of code]");
                for reference in references(body, &pool) {
                    let _ = writeln!(out, "    {reference}");
                }
            }
            None => out.push('\n'),
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A class file `Hello` with one field and one method calling `PrintStream.println("hi")`,
    /// assembled by hand: the constant pool, then the class.
    fn hello(greeting: &str) -> Vec<u8> {
        let mut pool: Vec<Vec<u8>> = Vec::new();
        let utf8 = |text: &str| {
            let mut entry = vec![1];
            entry.extend_from_slice(&(text.len() as u16).to_be_bytes());
            entry.extend_from_slice(text.as_bytes());
            entry
        };
        let pair = |tag: u8, a: u16, b: u16| {
            let mut entry = vec![tag];
            entry.extend_from_slice(&a.to_be_bytes());
            entry.extend_from_slice(&b.to_be_bytes());
            entry
        };
        let one = |tag: u8, a: u16| {
            let mut entry = vec![tag];
            entry.extend_from_slice(&a.to_be_bytes());
            entry
        };
        pool.push(utf8("Hello")); // 1
        pool.push(one(7, 1)); // 2 class Hello
        pool.push(utf8("java/lang/Object")); // 3
        pool.push(one(7, 3)); // 4 class Object
        pool.push(utf8("count")); // 5
        pool.push(utf8("I")); // 6
        pool.push(utf8("main")); // 7
        pool.push(utf8("([Ljava/lang/String;)V")); // 8
        pool.push(utf8("Code")); // 9
        pool.push(utf8("java/io/PrintStream")); // 10
        pool.push(one(7, 10)); // 11
        pool.push(utf8("println")); // 12
        pool.push(utf8("(Ljava/lang/String;)V")); // 13
        pool.push(pair(12, 12, 13)); // 14
        pool.push(pair(10, 11, 14)); // 15 Methodref PrintStream.println
        pool.push(utf8(greeting)); // 16
        pool.push(one(8, 16)); // 17 String
        let mut bytes = vec![0xCA, 0xFE, 0xBA, 0xBE, 0, 0, 0, 61];
        bytes.extend_from_slice(&(pool.len() as u16 + 1).to_be_bytes());
        for entry in pool {
            bytes.extend(entry);
        }
        bytes.extend_from_slice(&[0x00, 0x21, 0, 2, 0, 4, 0, 0]); // public super, this, super, 0 interfaces
        bytes.extend_from_slice(&[0, 1, 0x00, 0x02, 0, 5, 0, 6, 0, 0]); // private int count
        // public static void main(String[]): aconst_null, ldc #17, invokevirtual #15, return
        let code = [0x01, 0x12, 17, 0xb6, 0, 15, 0xb1];
        let mut attribute = vec![0, 2, 0, 1];
        attribute.extend_from_slice(&(code.len() as u32).to_be_bytes());
        attribute.extend_from_slice(&code);
        attribute.extend_from_slice(&[0, 0, 0, 0]);
        bytes.extend_from_slice(&[0, 1, 0x00, 0x09, 0, 7, 0, 8, 0, 1, 0, 9]);
        bytes.extend_from_slice(&(attribute.len() as u32).to_be_bytes());
        bytes.extend(attribute);
        bytes.extend_from_slice(&[0, 0]);
        bytes
    }

    #[test]
    fn a_class_lists_its_members_and_what_its_code_reaches() {
        let text = listing(&hello("hi")).expect("a class file");
        assert_eq!(
            text,
            "public class Hello extends java.lang.Object\nclass file version 61.0\n\
             \nfield private int count\n\
             \nmethod public static void main(java.lang.String[])  [7 bytes of code]\n\
             \x20   loads \"hi\"\n\
             \x20   calls java.io.PrintStream.println(java.lang.String)\n"
        );
    }

    #[test]
    fn a_mach_o_universal_binary_is_not_a_class() {
        assert!(is_class(&hello("hi")));
        assert!(!is_class(&[0xCA, 0xFE, 0xBA, 0xBE, 0, 0, 0, 2, 0, 0, 0, 7]));
    }

    #[test]
    fn switches_are_stepped_over() {
        // iconst_0, tableswitch: two bytes of padding to a multiple of 4, default, low 0,
        // high 1, two offsets; then return.
        let mut code = vec![0x03, 0xaa, 0, 0];
        for word in [0u32, 0, 1, 0, 0] {
            code.extend_from_slice(&word.to_be_bytes());
        }
        code.push(0xb1);
        assert_eq!(instruction_length(&code, 1), Some(23));
        assert_eq!(instruction_length(&code, 24), Some(1));
        assert!(references(&code, &Pool(Vec::new())).is_empty());
    }
}
