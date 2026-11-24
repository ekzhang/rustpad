//! Language table: how to compile, run and probe each supported language.

/// Resource limits for one process.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Limits {
    pub cpu_ms: u64,
    pub clock_ms: u64,
    pub memory_mib: u64,
    pub procs: u64,
}

/// One command of a language: compile or run.
///
/// `args` may hold the whole-token placeholders `{file}` and `{class}`. `env`
/// is added to `BASE_ENV`. `cache` lists names kept with `copyOutCached`; a
/// trailing `?` marks a name as optional.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Stage {
    pub args: &'static [&'static str],
    pub env: &'static [&'static str],
    pub limits: Limits,
    pub cache: &'static [&'static str],
}

/// Where the source file name comes from.
#[derive(Clone, Copy, Debug)]
pub(crate) enum SourceName {
    /// A fixed file name.
    Fixed(&'static str),
    /// `<Class>.java`, taken from the source.
    JavaClass,
}

/// A language the runner can execute.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Language {
    /// Monaco language ID.
    pub id: &'static str,
    /// Runtime name shown to clients.
    pub name: &'static str,
    pub source: SourceName,
    pub compile: Option<Stage>,
    pub run: Stage,
    /// Command whose output holds the runtime version.
    pub probe: &'static [&'static str],
}

/// Environment of every process.
pub(crate) const BASE_ENV: &[&str] = &[
    "PATH=/usr/local/bin:/usr/bin:/bin",
    "HOME=/w",
    "LANG=C.UTF-8",
];

/// Limits of version probes.
pub(crate) const PROBE_LIMITS: Limits = Limits {
    cpu_ms: 10_000,
    clock_ms: 15_000,
    memory_mib: 1024,
    procs: 128,
};

const RUN_LIMITS: Limits = Limits {
    cpu_ms: 3000,
    clock_ms: 5000,
    memory_mib: 256,
    procs: 64,
};

const NATIVE_COMPILE_LIMITS: Limits = Limits {
    cpu_ms: 10_000,
    clock_ms: 15_000,
    memory_mib: 512,
    procs: 64,
};

const JAVAC_LIMITS: Limits = Limits {
    cpu_ms: 20_000,
    clock_ms: 15_000,
    memory_mib: 512,
    procs: 128,
};

const KOTLINC_LIMITS: Limits = Limits {
    cpu_ms: 20_000,
    clock_ms: 15_000,
    memory_mib: 1024,
    procs: 128,
};

const JVM_RUN: Stage = Stage {
    args: &[
        "/usr/bin/java",
        "-Xmx192m",
        "-XX:+UseSerialGC",
        "-XX:TieredStopAtLevel=1",
        "-jar",
        "main.jar",
    ],
    env: &[],
    limits: Limits {
        cpu_ms: 5000,
        clock_ms: 5000,
        memory_mib: 256,
        procs: 128,
    },
    cache: &[],
};

const NATIVE_RUN: Stage = Stage {
    args: &["main"],
    env: &[],
    limits: RUN_LIMITS,
    cache: &[],
};

pub(crate) const LANGUAGES: &[Language] = &[
    Language {
        id: "python",
        name: "python",
        source: SourceName::Fixed("main.py"),
        compile: None,
        run: Stage {
            args: &["/usr/bin/python3", "main.py"],
            env: &[],
            limits: RUN_LIMITS,
            cache: &[],
        },
        probe: &["/usr/bin/python3", "--version"],
    },
    Language {
        id: "c",
        name: "gcc",
        source: SourceName::Fixed("main.c"),
        compile: Some(Stage {
            args: &["/usr/bin/gcc", "-O2", "-std=c17", "main.c", "-o", "main", "-lm"],
            env: &[],
            limits: NATIVE_COMPILE_LIMITS,
            cache: &["main?"],
        }),
        run: NATIVE_RUN,
        probe: &["/usr/bin/gcc", "-dumpfullversion"],
    },
    Language {
        id: "cpp",
        name: "g++",
        source: SourceName::Fixed("main.cpp"),
        compile: Some(Stage {
            args: &["/usr/bin/g++", "-O2", "-std=c++20", "main.cpp", "-o", "main"],
            env: &[],
            limits: NATIVE_COMPILE_LIMITS,
            cache: &["main?"],
        }),
        run: NATIVE_RUN,
        probe: &["/usr/bin/g++", "-dumpfullversion"],
    },
    Language {
        id: "java",
        name: "java",
        source: SourceName::JavaClass,
        compile: Some(Stage {
            args: &[
                "/bin/sh",
                "-c",
                "javac -J-Xmx384m -J-XX:+UseSerialGC -J-XX:TieredStopAtLevel=1 -d classes \"$0\" && jar -J-Djava.io.tmpdir=. --create --file main.jar --main-class \"$1\" -C classes .",
                "{file}",
                "{class}",
            ],
            env: &[],
            limits: JAVAC_LIMITS,
            cache: &["main.jar?"],
        }),
        run: JVM_RUN,
        probe: &["/usr/bin/java", "-version"],
    },
    Language {
        id: "kotlin",
        name: "kotlin",
        source: SourceName::Fixed("main.kt"),
        compile: Some(Stage {
            args: &[
                "/usr/local/bin/kotlinc",
                "-J-Xmx384m",
                "-J-XX:+UseSerialGC",
                "main.kt",
                "-include-runtime",
                "-d",
                "main.jar",
            ],
            env: &[],
            limits: KOTLINC_LIMITS,
            cache: &["main.jar?"],
        }),
        run: JVM_RUN,
        probe: &["/usr/local/bin/kotlinc", "-version"],
    },
    Language {
        id: "javascript",
        name: "node",
        source: SourceName::Fixed("main.js"),
        compile: None,
        run: Stage {
            args: &["/usr/bin/node", "main.js"],
            env: &[],
            limits: RUN_LIMITS,
            cache: &[],
        },
        probe: &["/usr/bin/node", "--version"],
    },
];

/// Looks up a language by Monaco ID.
pub(crate) fn find(id: &str) -> Option<&'static Language> {
    LANGUAGES.iter().find(|lang| lang.id == id)
}

/// Returns the first match of `\d+(\.\d+)+` in `output`.
pub(crate) fn parse_version(output: &str) -> Option<String> {
    let bytes = output.as_bytes();
    let digits_end = |mut i: usize| {
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        i
    };
    let mut start = 0;
    while start < bytes.len() {
        if !bytes[start].is_ascii_digit() {
            start += 1;
            continue;
        }
        let mut end = digits_end(start);
        let mut groups = 0;
        while end < bytes.len() && bytes[end] == b'.' && digits_end(end + 1) > end + 1 {
            end = digits_end(end + 1);
            groups += 1;
        }
        if groups > 0 {
            return Some(output[start..end].to_string());
        }
        start = end;
    }
    None
}

/// File and class of a Java source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct JavaEntry {
    pub file: String,
    pub class: String,
}

fn is_ident(word: &str) -> bool {
    !word.is_empty()
        && !word.starts_with(|c: char| c.is_ascii_digit())
        && word
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
}

/// Returns the index just past the literal that starts at `start` and ends at
/// the first unescaped `close`. An unterminated literal ends at the end of the
/// line, or of the source when `multiline` is set.
fn skip_literal(source: &str, start: usize, close: &str, multiline: bool) -> usize {
    // Bytes, not chars: an escape may skip into a multi-byte char, and
    // `close` and `\n` are ASCII, so every returned index is a char boundary.
    let bytes = source.as_bytes();
    let mut i = start;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            i += 2;
        } else if bytes[i..].starts_with(close.as_bytes()) {
            return i + close.len();
        } else if bytes[i] == b'\n' && !multiline {
            return i;
        } else {
            i += 1;
        }
    }
    bytes.len()
}

/// Splits source into words of `[A-Za-z0-9_$.]` and single other characters,
/// skipping whitespace and comments. Each string, char literal or text block
/// becomes one `"` or `'` token.
fn java_tokens(source: &str) -> Vec<&str> {
    let mut tokens = Vec::new();
    let mut start = None;
    let mut i = 0;
    while i < source.len() {
        let rest = &source[i..];
        let c = rest.chars().next().unwrap_or_default();
        let word = c.is_alphanumeric() || c == '_' || c == '$' || c == '.';
        if let (false, Some(s)) = (word, start) {
            tokens.push(&source[s..i]);
            start = None;
        }
        if word {
            start.get_or_insert(i);
            i += c.len_utf8();
        } else if rest.starts_with("//") {
            i = rest.find('\n').map_or(source.len(), |n| i + n);
        } else if rest.starts_with("/*") {
            i = rest[2..].find("*/").map_or(source.len(), |n| i + 2 + n + 2);
        } else if rest.starts_with("\"\"\"") {
            tokens.push("\"");
            i = skip_literal(source, i + 3, "\"\"\"", true);
        } else if c == '"' || c == '\'' {
            tokens.push(&source[i..i + 1]);
            i = skip_literal(source, i + 1, &source[i..i + 1], false);
        } else {
            if !c.is_whitespace() {
                tokens.push(&source[i..i + c.len_utf8()]);
            }
            i += c.len_utf8();
        }
    }
    if let Some(s) = start {
        tokens.push(&source[s..]);
    }
    tokens
}

/// Finds the class to run and the file to save it in. The class is the first
/// `public` top-level class, else the first `class`, else `Main`. A package
/// counts only as a leading `package <name>;`.
pub(crate) fn java_entry(source: &str) -> JavaEntry {
    let tokens = java_tokens(source);
    let package = match tokens.as_slice() {
        ["package", name, ";", ..] if name.split('.').all(is_ident) => Some(*name),
        _ => None,
    };

    let mut public = None;
    let mut first = None;
    let mut depth = 0usize;
    for (i, token) in tokens.iter().enumerate() {
        match *token {
            "{" => depth += 1,
            "}" => depth = depth.saturating_sub(1),
            _ => {}
        }
        if *token != "class" || depth != 0 {
            continue;
        }
        let name = tokens.get(i + 1).copied();
        if first.is_none() {
            first = Some(name);
        }
        let mut j = i;
        while j > 0 && matches!(tokens[j - 1], "final" | "abstract") {
            j -= 1;
        }
        if j > 0 && tokens[j - 1] == "public" {
            public = Some(name);
            break;
        }
    }

    let name = public
        .or(first)
        .flatten()
        .filter(|n| is_ident(n))
        .unwrap_or("Main");
    JavaEntry {
        file: format!("{name}.java"),
        class: match package {
            Some(package) => format!("{package}.{name}"),
            None => name.to_string(),
        },
    }
}

/// Returns the source file name and the class name for placeholders. The class
/// is empty unless the language names it.
pub(crate) fn source_and_class(lang: &Language, source: &str) -> (String, String) {
    match lang.source {
        SourceName::Fixed(name) => (name.to_string(), String::new()),
        SourceName::JavaClass => {
            let entry = java_entry(source);
            (entry.file, entry.class)
        }
    }
}

/// Replaces whole-token `{file}` and `{class}` placeholders.
pub(crate) fn expand(args: &[&str], file: &str, class: &str) -> Vec<String> {
    args.iter()
        .map(|arg| match *arg {
            "{file}" => file.to_string(),
            "{class}" => class.to_string(),
            other => other.to_string(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_version_cases() {
        let cases = [
            ("Python 3.13.5\n", Some("3.13.5")),
            ("14.2.0\n", Some("14.2.0")),
            ("openjdk version \"21.0.9\" 2025-10-21", Some("21.0.9")),
            ("info: kotlinc-jvm 2.2.21 (JRE 21.0.9+10)", Some("2.2.21")),
            ("v20.19.2\n", Some("20.19.2")),
            ("no digits", None),
            ("version 7", None),
        ];
        for (input, want) in cases {
            assert_eq!(parse_version(input).as_deref(), want, "{input:?}");
        }
    }

    fn entry(src: &str, file: &str, class: &str) {
        let e = java_entry(src);
        assert_eq!((e.file.as_str(), e.class.as_str()), (file, class));
    }

    #[test]
    fn java_entry_public_class() {
        entry(
            "public class Solution { public static void main(String[] a) {} }",
            "Solution.java",
            "Solution",
        );
    }

    #[test]
    fn java_entry_without_public_class() {
        entry("class Foo {}\nclass Bar {}", "Foo.java", "Foo");
    }

    #[test]
    fn java_entry_public_final() {
        entry("public final class App {}", "App.java", "App");
    }

    #[test]
    fn java_entry_defaults_to_main() {
        entry("interface X {}", "Main.java", "Main");
    }

    #[test]
    fn java_entry_handles_package() {
        entry(
            "package demo.app;\npublic class Solution {}",
            "Solution.java",
            "demo.app.Solution",
        );
    }

    #[test]
    fn java_entry_ignores_nested_public_class() {
        entry(
            "class Main { public final class Inner {} }",
            "Main.java",
            "Main",
        );
        entry(
            "public class Outer { public static class Inner {} }",
            "Outer.java",
            "Outer",
        );
    }

    #[test]
    fn java_entry_non_ascii_falls_back() {
        entry("public class Café {}", "Main.java", "Main");
    }

    #[test]
    fn java_entry_handles_generics() {
        entry("public class Foo<T> {}", "Foo.java", "Foo");
    }

    #[test]
    fn java_entry_ignores_package_in_comment() {
        entry(
            "// Solution for the package delivery problem\npublic class Solution { public static void main(String[] a) {} }",
            "Solution.java",
            "Solution",
        );
    }

    #[test]
    fn java_entry_ignores_package_in_string() {
        entry(
            "public class Solution { public static void main(String[] a) { System.out.println(\"package received\"); } }",
            "Solution.java",
            "Solution",
        );
    }

    #[test]
    fn java_entry_ignores_class_in_comment() {
        entry(
            "// Simple class to test\nclass Main { public static void main(String[] a) {} }",
            "Main.java",
            "Main",
        );
        entry(
            "/* a class to\n   test */ class Main {}",
            "Main.java",
            "Main",
        );
    }

    #[test]
    fn java_entry_ignores_braces_in_strings_and_comments() {
        entry(
            "class A { String s = \"{\"; }\npublic class B {}",
            "B.java",
            "B",
        );
        entry(
            "class A { char c = '{'; String e = \"\\\"{\"; } // {\npublic class B {}",
            "B.java",
            "B",
        );
        entry(
            "class A { String t = \"\"\"\n  { \" class X\n  \"\"\"; }\npublic class B {}",
            "B.java",
            "B",
        );
    }

    #[test]
    fn java_entry_survives_bad_literals() {
        entry(
            "class A { String s = \"\\é{\"; }\npublic class B {}",
            "B.java",
            "B",
        );
        entry("public class B { String s = \"open\n}", "B.java", "B");
        entry("public class B { /* open", "B.java", "B");
        entry("class A { String s = \"\\", "A.java", "A");
    }

    #[test]
    fn java_entry_package_must_lead() {
        entry(
            "/* header */\npackage demo;\nimport java.util.*;\npublic class App {}",
            "App.java",
            "demo.App",
        );
        entry(
            "import java.util.*;\npackage demo;\npublic class App {}",
            "App.java",
            "App",
        );
        entry("package demo\npublic class App {}", "App.java", "App");
    }

    #[test]
    fn parse_version_trailing_dot() {
        assert_eq!(parse_version("3.13.").as_deref(), Some("3.13"));
    }

    #[test]
    fn expand_replaces_placeholders() {
        assert_eq!(
            expand(&["sh", "{file}", "{class}", "x{file}"], "A.java", "A"),
            ["sh", "A.java", "A", "x{file}"]
        );
    }

    #[test]
    fn table_ids_are_editor_languages() {
        let editor: serde_json::Value =
            serde_json::from_str(include_str!("../../../src/languages.json")).unwrap();
        let text = editor.to_string();
        for lang in LANGUAGES {
            assert!(text.contains(&format!("\"{}\"", lang.id)), "{}", lang.id);
        }
        let ids: Vec<_> = LANGUAGES.iter().map(|l| l.id).collect();
        assert_eq!(ids, ["python", "c", "cpp", "java", "kotlin", "javascript"]);
    }

    #[test]
    fn find_unknown_is_none() {
        assert!(find("markdown").is_none());
    }
}
