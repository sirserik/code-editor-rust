use egui::Color32;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HighlightKind {
    Keyword,
    Type,
    Function,
    String,
    Number,
    Comment,
    Operator,
    Punctuation,
    Variable,
    Constant,
    Attribute,
    Tag,
    Normal,
}

impl HighlightKind {
    pub fn color(&self, dark: bool) -> Color32 {
        if dark {
            // JetBrains Darcula-inspired dark theme colors
            match self {
                HighlightKind::Keyword => Color32::from_rgb(204, 120, 50),    // Darcula orange keywords
                HighlightKind::Type => Color32::from_rgb(152, 118, 170),      // Darcula purple types
                HighlightKind::Function => Color32::from_rgb(255, 198, 109),  // Darcula yellow functions
                HighlightKind::String => Color32::from_rgb(106, 171, 115),    // Darcula green strings
                HighlightKind::Number => Color32::from_rgb(104, 151, 210),    // Darcula blue numbers
                HighlightKind::Comment => Color32::from_rgb(128, 128, 128),   // Gray comments
                HighlightKind::Operator => Color32::from_rgb(187, 187, 187),  // Default text
                HighlightKind::Punctuation => Color32::from_rgb(187, 187, 187), // Default text
                HighlightKind::Variable => Color32::from_rgb(152, 118, 170),  // Purple variables
                HighlightKind::Constant => Color32::from_rgb(152, 118, 170),  // Purple constants
                HighlightKind::Attribute => Color32::from_rgb(187, 181, 41),  // Yellow annotations
                HighlightKind::Tag => Color32::from_rgb(232, 191, 106),       // Gold tags
                HighlightKind::Normal => Color32::from_rgb(187, 187, 187),    // Default text
            }
        } else {
            // JetBrains IntelliJ Light theme colors
            match self {
                HighlightKind::Keyword => Color32::from_rgb(0, 51, 179),      // JB dark blue keywords
                HighlightKind::Type => Color32::from_rgb(0, 112, 128),        // JB teal types
                HighlightKind::Function => Color32::from_rgb(120, 73, 42),    // JB brown functions
                HighlightKind::String => Color32::from_rgb(6, 125, 23),       // JB green strings
                HighlightKind::Number => Color32::from_rgb(23, 80, 235),      // JB blue numbers
                HighlightKind::Comment => Color32::from_rgb(138, 138, 138),   // JB gray comments
                HighlightKind::Operator => Color32::from_rgb(40, 40, 40),     // Dark operators
                HighlightKind::Punctuation => Color32::from_rgb(40, 40, 40),  // Dark punctuation
                HighlightKind::Variable => Color32::from_rgb(100, 0, 128),    // JB purple variables
                HighlightKind::Constant => Color32::from_rgb(100, 0, 128),    // JB purple constants
                HighlightKind::Attribute => Color32::from_rgb(148, 115, 0),   // JB dark gold annotations
                HighlightKind::Tag => Color32::from_rgb(0, 51, 179),         // JB blue tags
                HighlightKind::Normal => Color32::from_rgb(40, 40, 40),       // Default text
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct HighlightSpan {
    pub start: usize,
    pub end: usize,
    pub kind: HighlightKind,
}

/// Simple regex-based syntax highlighter (no tree-sitter dependency issues)
pub fn highlight_line(line: &str, language: &str) -> Vec<HighlightSpan> {
    let mut spans = Vec::new();

    match language {
        "rust" | "rs" => highlight_rust(line, &mut spans),
        "javascript" | "js" | "jsx" | "mjs" | "cjs" => highlight_js(line, &mut spans),
        "typescript" | "ts" | "tsx" | "mts" | "cts" => highlight_ts(line, &mut spans),
        "python" | "py" => highlight_python(line, &mut spans),
        "go" => highlight_go(line, &mut spans),
        "html" | "htm" | "xml" | "svg" => highlight_html(line, &mut spans),
        "css" | "scss" | "sass" => highlight_css(line, &mut spans),
        "json" => highlight_json(line, &mut spans),
        "toml" => highlight_toml(line, &mut spans),
        "yaml" | "yml" => highlight_yaml(line, &mut spans),
        "sh" | "bash" | "zsh" => highlight_shell(line, &mut spans),
        "php" => highlight_php(line, &mut spans),
        "java" | "c" | "cpp" | "h" | "hpp" => highlight_c_like(line, &mut spans),
        "sql" => highlight_sql(line, &mut spans),
        "markdown" | "md" => highlight_markdown(line, &mut spans),
        _ => {} // No highlighting
    }

    spans
}

pub fn detect_language(path: &str) -> &str {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    match ext {
        "rs" => "rust",
        "js" | "mjs" | "cjs" => "javascript",
        "jsx" => "jsx",
        "ts" | "mts" | "cts" => "typescript",
        "tsx" => "tsx",
        "py" => "python",
        "go" => "go",
        "html" | "htm" => "html",
        "xml" | "svg" => "xml",
        "css" => "css",
        "scss" | "sass" => "scss",
        "json" => "json",
        "toml" => "toml",
        "yaml" | "yml" => "yaml",
        "sh" | "bash" | "zsh" => "sh",
        "php" => "php",
        "java" => "java",
        "c" | "h" => "c",
        "cpp" | "cc" | "cxx" | "hpp" => "cpp",
        "sql" => "sql",
        "md" | "markdown" => "markdown",
        "rb" => "ruby",
        "swift" => "swift",
        "kt" | "kts" => "kotlin",
        "vue" => "vue",
        "svelte" => "svelte",
        "Dockerfile" => "dockerfile",
        _ => {
            // Check filename
            let name = std::path::Path::new(path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
            match name {
                "Dockerfile" => "dockerfile",
                "Makefile" | "makefile" => "makefile",
                ".gitignore" | ".dockerignore" => "gitignore",
                _ => "text",
            }
        }
    }
}

// ---- Language-specific highlighters ----

fn highlight_rust(line: &str, spans: &mut Vec<HighlightSpan>) {
    let keywords = &[
        "fn", "let", "mut", "const", "static", "struct", "enum", "impl", "trait", "type",
        "pub", "mod", "use", "crate", "self", "super", "as", "if", "else", "match",
        "for", "while", "loop", "break", "continue", "return", "where", "in", "ref",
        "move", "async", "await", "unsafe", "extern", "dyn", "macro_rules",
    ];
    let types = &[
        "bool", "char", "str", "i8", "i16", "i32", "i64", "i128", "isize",
        "u8", "u16", "u32", "u64", "u128", "usize", "f32", "f64",
        "String", "Vec", "Option", "Result", "Box", "Rc", "Arc", "HashMap", "HashSet",
        "Self", "Some", "None", "Ok", "Err", "true", "false",
    ];
    highlight_generic(line, spans, keywords, types, "//", &[("\"", "\""), ("'", "'")]);
}

fn highlight_js(line: &str, spans: &mut Vec<HighlightSpan>) {
    let keywords = &[
        "const", "let", "var", "function", "return", "if", "else", "for", "while", "do",
        "switch", "case", "break", "continue", "new", "this", "class", "extends", "super",
        "import", "export", "from", "default", "async", "await", "try", "catch", "finally",
        "throw", "typeof", "instanceof", "in", "of", "yield", "delete", "void",
    ];
    let types = &[
        "true", "false", "null", "undefined", "NaN", "Infinity", "console", "window",
        "document", "Array", "Object", "String", "Number", "Boolean", "Map", "Set",
        "Promise", "Error", "JSON", "Math", "Date", "RegExp",
    ];
    highlight_generic(line, spans, keywords, types, "//", &[("\"", "\""), ("'", "'"), ("`", "`")]);
}

fn highlight_ts(line: &str, spans: &mut Vec<HighlightSpan>) {
    let keywords = &[
        "const", "let", "var", "function", "return", "if", "else", "for", "while", "do",
        "switch", "case", "break", "continue", "new", "this", "class", "extends", "super",
        "import", "export", "from", "default", "async", "await", "try", "catch", "finally",
        "throw", "typeof", "instanceof", "in", "of", "type", "interface", "enum",
        "implements", "abstract", "declare", "namespace", "module", "readonly", "keyof",
        "as", "is", "satisfies", "infer",
    ];
    let types = &[
        "true", "false", "null", "undefined", "void", "never", "any", "unknown",
        "string", "number", "boolean", "object", "symbol", "bigint",
        "Array", "Record", "Partial", "Required", "Readonly", "Pick", "Omit",
        "Promise", "Map", "Set",
    ];
    highlight_generic(line, spans, keywords, types, "//", &[("\"", "\""), ("'", "'"), ("`", "`")]);
}

fn highlight_python(line: &str, spans: &mut Vec<HighlightSpan>) {
    let keywords = &[
        "def", "class", "if", "elif", "else", "for", "while", "break", "continue",
        "return", "yield", "import", "from", "as", "try", "except", "finally", "raise",
        "with", "pass", "lambda", "and", "or", "not", "in", "is", "global", "nonlocal",
        "assert", "del", "async", "await", "match", "case",
    ];
    let types = &[
        "True", "False", "None", "int", "float", "str", "bool", "list", "dict", "tuple",
        "set", "frozenset", "bytes", "bytearray", "type", "object", "range", "print",
        "len", "self", "cls", "super", "property", "staticmethod", "classmethod",
    ];
    highlight_generic(line, spans, keywords, types, "#", &[("\"\"\"", "\"\"\""), ("\"", "\""), ("'", "'")]);
}

fn highlight_go(line: &str, spans: &mut Vec<HighlightSpan>) {
    let keywords = &[
        "func", "var", "const", "type", "struct", "interface", "map", "chan",
        "if", "else", "for", "range", "switch", "case", "default", "break", "continue",
        "return", "go", "defer", "select", "package", "import", "fallthrough", "goto",
    ];
    let types = &[
        "bool", "byte", "int", "int8", "int16", "int32", "int64",
        "uint", "uint8", "uint16", "uint32", "uint64", "float32", "float64",
        "complex64", "complex128", "string", "rune", "error",
        "true", "false", "nil", "iota", "append", "len", "cap", "make", "new",
    ];
    highlight_generic(line, spans, keywords, types, "//", &[("\"", "\""), ("`", "`")]);
}

fn highlight_html(line: &str, spans: &mut Vec<HighlightSpan>) {
    let chars: Vec<char> = line.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        // HTML comment
        if i + 3 < len && chars[i] == '<' && chars[i+1] == '!' && chars[i+2] == '-' && chars[i+3] == '-' {
            let start = i;
            i += 4;
            while i + 2 < len && !(chars[i] == '-' && chars[i+1] == '-' && chars[i+2] == '>') { i += 1; }
            if i + 2 < len { i += 3; }
            spans.push(HighlightSpan { start, end: i, kind: HighlightKind::Comment });
        }
        // Tag
        else if chars[i] == '<' {
            // < and optional /
            let start = i;
            i += 1;
            if i < len && chars[i] == '/' { i += 1; }
            spans.push(HighlightSpan { start, end: i, kind: HighlightKind::Punctuation });

            // Tag name
            let name_start = i;
            while i < len && (chars[i].is_alphanumeric() || chars[i] == '-' || chars[i] == '!' || chars[i] == ':') { i += 1; }
            if i > name_start {
                spans.push(HighlightSpan { start: name_start, end: i, kind: HighlightKind::Tag });
            }

            // Attributes
            while i < len && chars[i] != '>' {
                // Skip whitespace
                while i < len && chars[i].is_whitespace() { i += 1; }
                if i >= len || chars[i] == '>' || chars[i] == '/' { break; }

                // Attribute name
                let attr_start = i;
                while i < len && chars[i] != '=' && chars[i] != '>' && chars[i] != '/' && !chars[i].is_whitespace() { i += 1; }
                if i > attr_start {
                    spans.push(HighlightSpan { start: attr_start, end: i, kind: HighlightKind::Attribute });
                }

                // = sign
                if i < len && chars[i] == '=' {
                    spans.push(HighlightSpan { start: i, end: i + 1, kind: HighlightKind::Operator });
                    i += 1;
                }

                // Attribute value (quoted string)
                if i < len && (chars[i] == '"' || chars[i] == '\'') {
                    let quote = chars[i];
                    let val_start = i;
                    i += 1;
                    while i < len && chars[i] != quote { i += 1; }
                    if i < len { i += 1; }
                    spans.push(HighlightSpan { start: val_start, end: i, kind: HighlightKind::String });
                }
            }

            // /> or >
            if i < len {
                let close_start = i;
                if chars[i] == '/' { i += 1; }
                if i < len && chars[i] == '>' { i += 1; }
                spans.push(HighlightSpan { start: close_start, end: i, kind: HighlightKind::Punctuation });
            }
        }
        // Entity &..;
        else if chars[i] == '&' {
            let start = i;
            while i < len && chars[i] != ';' { i += 1; }
            if i < len { i += 1; }
            spans.push(HighlightSpan { start, end: i, kind: HighlightKind::Constant });
        }
        else {
            i += 1;
        }
    }
}

fn highlight_css(line: &str, spans: &mut Vec<HighlightSpan>) {
    let chars: Vec<char> = line.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        // Comments
        if i + 1 < len && chars[i] == '/' && chars[i + 1] == '*' {
            let start = i;
            i += 2;
            while i + 1 < len && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            if i + 1 < len {
                i += 2;
            }
            spans.push(HighlightSpan { start, end: i, kind: HighlightKind::Comment });
        }
        // Strings
        else if chars[i] == '"' || chars[i] == '\'' {
            let quote = chars[i];
            let start = i;
            i += 1;
            while i < len && chars[i] != quote {
                if chars[i] == '\\' { i += 1; }
                i += 1;
            }
            if i < len { i += 1; }
            spans.push(HighlightSpan { start, end: i, kind: HighlightKind::String });
        }
        // Numbers
        else if chars[i].is_ascii_digit() || (chars[i] == '#' && i + 1 < len && chars[i+1].is_ascii_hexdigit()) {
            let start = i;
            i += 1;
            while i < len && (chars[i].is_ascii_alphanumeric() || chars[i] == '.') {
                i += 1;
            }
            spans.push(HighlightSpan { start, end: i, kind: HighlightKind::Number });
        }
        // Properties (word before colon)
        else if chars[i].is_alphabetic() || chars[i] == '-' {
            let start = i;
            while i < len && (chars[i].is_alphanumeric() || chars[i] == '-' || chars[i] == '_') {
                i += 1;
            }
            // Check if followed by colon
            let mut j = i;
            while j < len && chars[j].is_whitespace() { j += 1; }
            if j < len && chars[j] == ':' {
                spans.push(HighlightSpan { start, end: i, kind: HighlightKind::Attribute });
            } else {
                spans.push(HighlightSpan { start, end: i, kind: HighlightKind::Normal });
            }
        }
        else {
            i += 1;
        }
    }
}

fn highlight_json(line: &str, spans: &mut Vec<HighlightSpan>) {
    let chars: Vec<char> = line.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        if chars[i] == '"' {
            let start = i;
            i += 1;
            while i < len && chars[i] != '"' {
                if chars[i] == '\\' { i += 1; }
                i += 1;
            }
            if i < len { i += 1; }
            // Check if it's a key (followed by :)
            let mut j = i;
            while j < len && chars[j].is_whitespace() { j += 1; }
            let kind = if j < len && chars[j] == ':' {
                HighlightKind::Attribute
            } else {
                HighlightKind::String
            };
            spans.push(HighlightSpan { start, end: i, kind });
        } else if chars[i].is_ascii_digit() || chars[i] == '-' {
            let start = i;
            i += 1;
            while i < len && (chars[i].is_ascii_digit() || chars[i] == '.' || chars[i] == 'e' || chars[i] == 'E' || chars[i] == '+' || chars[i] == '-') {
                i += 1;
            }
            spans.push(HighlightSpan { start, end: i, kind: HighlightKind::Number });
        } else {
            let word_start = i;
            if chars[i].is_alphabetic() {
                while i < len && chars[i].is_alphabetic() { i += 1; }
                let word: String = chars[word_start..i].iter().collect();
                match word.as_str() {
                    "true" | "false" | "null" => {
                        spans.push(HighlightSpan { start: word_start, end: i, kind: HighlightKind::Keyword });
                    }
                    _ => {}
                }
            } else {
                i += 1;
            }
        }
    }
}

fn highlight_toml(line: &str, spans: &mut Vec<HighlightSpan>) {
    let trimmed = line.trim();
    if trimmed.starts_with('#') {
        spans.push(HighlightSpan { start: 0, end: line.len(), kind: HighlightKind::Comment });
        return;
    }
    if trimmed.starts_with('[') {
        spans.push(HighlightSpan { start: 0, end: line.len(), kind: HighlightKind::Tag });
        return;
    }
    highlight_generic(line, spans, &[], &["true", "false"], "#", &[("\"", "\""), ("'", "'")]);
}

fn highlight_yaml(line: &str, spans: &mut Vec<HighlightSpan>) {
    let trimmed = line.trim();
    if trimmed.starts_with('#') {
        spans.push(HighlightSpan { start: 0, end: line.len(), kind: HighlightKind::Comment });
        return;
    }
    // Keys before colon
    if let Some(colon_pos) = line.find(':') {
        let key_part = &line[..colon_pos];
        if !key_part.trim().is_empty() && !key_part.trim().starts_with('-') {
            spans.push(HighlightSpan { start: 0, end: colon_pos, kind: HighlightKind::Attribute });
        }
    }
    highlight_generic(line, spans, &[], &["true", "false", "null", "yes", "no"], "#", &[("\"", "\""), ("'", "'")]);
}

fn highlight_shell(line: &str, spans: &mut Vec<HighlightSpan>) {
    let keywords = &[
        "if", "then", "else", "elif", "fi", "for", "while", "do", "done", "case", "esac",
        "function", "return", "exit", "local", "export", "source", "alias", "unalias",
        "echo", "printf", "read", "set", "unset", "shift", "test",
    ];
    highlight_generic(line, spans, keywords, &["true", "false"], "#", &[("\"", "\""), ("'", "'")]);
}

fn highlight_php(line: &str, spans: &mut Vec<HighlightSpan>) {
    let keywords = &[
        "function", "class", "public", "private", "protected", "static", "abstract",
        "interface", "extends", "implements", "new", "return", "if", "else", "elseif",
        "for", "foreach", "while", "do", "switch", "case", "break", "continue",
        "try", "catch", "finally", "throw", "use", "namespace", "require", "include",
        "echo", "print", "isset", "unset", "empty", "array", "match", "fn", "readonly",
    ];
    let types = &[
        "true", "false", "null", "int", "float", "string", "bool", "array", "object",
        "void", "mixed", "never", "self", "parent", "static",
    ];
    highlight_generic(line, spans, keywords, types, "//", &[("\"", "\""), ("'", "'")]);
}

fn highlight_c_like(line: &str, spans: &mut Vec<HighlightSpan>) {
    let keywords = &[
        "int", "char", "float", "double", "void", "long", "short", "unsigned", "signed",
        "struct", "union", "enum", "typedef", "class", "public", "private", "protected",
        "virtual", "override", "const", "static", "extern", "register", "volatile",
        "if", "else", "for", "while", "do", "switch", "case", "break", "continue",
        "return", "goto", "sizeof", "new", "delete", "try", "catch", "throw",
        "namespace", "using", "template", "typename", "auto", "constexpr", "nullptr",
        "include", "define", "ifdef", "ifndef", "endif", "pragma",
    ];
    highlight_generic(line, spans, keywords, &["true", "false", "NULL", "nullptr", "this"], "//", &[("\"", "\""), ("'", "'")]);
}

fn highlight_sql(line: &str, spans: &mut Vec<HighlightSpan>) {
    let keywords = &[
        "SELECT", "FROM", "WHERE", "JOIN", "LEFT", "RIGHT", "INNER", "OUTER", "ON",
        "INSERT", "INTO", "VALUES", "UPDATE", "SET", "DELETE", "CREATE", "ALTER", "DROP",
        "TABLE", "INDEX", "VIEW", "DATABASE", "SCHEMA", "GRANT", "REVOKE",
        "AND", "OR", "NOT", "IN", "EXISTS", "BETWEEN", "LIKE", "IS", "NULL",
        "ORDER", "BY", "GROUP", "HAVING", "LIMIT", "OFFSET", "UNION", "ALL",
        "AS", "DISTINCT", "COUNT", "SUM", "AVG", "MIN", "MAX", "CASE", "WHEN", "THEN", "END",
        "select", "from", "where", "join", "left", "right", "inner", "outer", "on",
        "insert", "into", "values", "update", "set", "delete", "create", "alter", "drop",
        "table", "index", "view", "and", "or", "not", "in", "exists", "between", "like",
        "is", "null", "order", "by", "group", "having", "limit", "offset", "as",
    ];
    highlight_generic(line, spans, keywords, &["TRUE", "FALSE", "true", "false"], "--", &[("'", "'")]);
}

fn highlight_markdown(line: &str, spans: &mut Vec<HighlightSpan>) {
    let trimmed = line.trim();
    if trimmed.starts_with('#') {
        spans.push(HighlightSpan { start: 0, end: line.len(), kind: HighlightKind::Keyword });
    } else if trimmed.starts_with("```") {
        spans.push(HighlightSpan { start: 0, end: line.len(), kind: HighlightKind::Comment });
    } else if trimmed.starts_with('>') {
        spans.push(HighlightSpan { start: 0, end: line.len(), kind: HighlightKind::String });
    } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("1.") {
        spans.push(HighlightSpan { start: 0, end: 2.min(line.len()), kind: HighlightKind::Keyword });
    }
}

/// Generic highlighter that handles keywords, types, comments, and strings
fn highlight_generic(
    line: &str,
    spans: &mut Vec<HighlightSpan>,
    keywords: &[&str],
    types: &[&str],
    line_comment: &str,
    string_delimiters: &[(&str, &str)],
) {
    let chars: Vec<char> = line.chars().collect();
    let len = chars.len();
    // Build char-index to byte-offset mapping for safe string slicing
    let byte_offsets: Vec<usize> = line.char_indices().map(|(b, _)| b).chain(std::iter::once(line.len())).collect();
    let byte_at = |ci: usize| -> usize { byte_offsets.get(ci).copied().unwrap_or(line.len()) };
    let mut i = 0;

    while i < len {
        // Check for line comment
        if line[byte_at(i)..].starts_with(line_comment) {
            spans.push(HighlightSpan {
                start: i,
                end: len,
                kind: HighlightKind::Comment,
            });
            return;
        }

        // Check for block comment start
        if i + 1 < len && chars[i] == '/' && chars[i + 1] == '*' {
            let start = i;
            i += 2;
            while i + 1 < len && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            if i + 1 < len {
                i += 2;
            }
            spans.push(HighlightSpan { start, end: i, kind: HighlightKind::Comment });
            continue;
        }

        // Check for strings
        let mut found_string = false;
        for &(open, close) in string_delimiters {
            if line[byte_at(i)..].starts_with(open) {
                let start = i;
                i += open.chars().count();
                while i < len {
                    if chars[i] == '\\' {
                        i += 2;
                        continue;
                    }
                    if line[byte_at(i)..].starts_with(close) {
                        i += close.chars().count();
                        break;
                    }
                    i += 1;
                }
                spans.push(HighlightSpan { start, end: i, kind: HighlightKind::String });
                found_string = true;
                break;
            }
        }
        if found_string {
            continue;
        }

        // Numbers
        if chars[i].is_ascii_digit() || (chars[i] == '.' && i + 1 < len && chars[i + 1].is_ascii_digit()) {
            let start = i;
            i += 1;
            while i < len && (chars[i].is_ascii_alphanumeric() || chars[i] == '.' || chars[i] == '_') {
                i += 1;
            }
            spans.push(HighlightSpan { start, end: i, kind: HighlightKind::Number });
            continue;
        }

        // Words (identifiers, keywords, types)
        if chars[i].is_alphabetic() || chars[i] == '_' {
            let start = i;
            while i < len && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();

            if keywords.contains(&word.as_str()) {
                spans.push(HighlightSpan { start, end: i, kind: HighlightKind::Keyword });
            } else if types.contains(&word.as_str()) {
                spans.push(HighlightSpan { start, end: i, kind: HighlightKind::Type });
            } else if i < len && chars[i] == '(' {
                spans.push(HighlightSpan { start, end: i, kind: HighlightKind::Function });
            } else if word.chars().next().map_or(false, |c| c.is_uppercase()) {
                spans.push(HighlightSpan { start, end: i, kind: HighlightKind::Type });
            }
            continue;
        }

        // Operators
        if "+-*/%=<>!&|^~?".contains(chars[i]) {
            let start = i;
            i += 1;
            while i < len && "+-*/%=<>!&|^~?".contains(chars[i]) {
                i += 1;
            }
            spans.push(HighlightSpan { start, end: i, kind: HighlightKind::Operator });
            continue;
        }

        // Punctuation
        if "{}[]();:,.@#$".contains(chars[i]) {
            spans.push(HighlightSpan { start: i, end: i + 1, kind: HighlightKind::Punctuation });
            i += 1;
            continue;
        }

        i += 1;
    }
}

// ---- Syntax error detection ----

#[derive(Debug, Clone)]
pub struct SyntaxError {
    pub line: usize,
    pub col: usize,
    pub length: usize,
    pub message: String,
}

/// Check for basic syntax errors: mismatched brackets, unclosed strings
pub fn check_syntax(content: &str, language: &str) -> Vec<SyntaxError> {
    let mut errors = Vec::new();

    // Skip non-code languages
    match language {
        "text" | "markdown" | "md" | "gitignore" => return errors,
        _ => {}
    }

    check_brackets(content, &mut errors);
    check_strings(content, language, &mut errors);

    errors
}

fn check_brackets(content: &str, errors: &mut Vec<SyntaxError>) {
    // Stack: (char, line, col)
    let mut stack: Vec<(char, usize, usize)> = Vec::new();
    let mut in_string = false;
    let mut string_char: char = '"';
    let mut in_line_comment = false;
    let mut in_block_comment = false;
    let mut prev_char = '\0';
    let mut line = 0usize;
    let mut col = 0usize;

    let chars: Vec<char> = content.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        let ch = chars[i];

        // Track line/col
        if ch == '\n' {
            line += 1;
            col = 0;
            in_line_comment = false;
            prev_char = ch;
            i += 1;
            continue;
        }

        // Block comment start
        if !in_string && !in_line_comment && !in_block_comment && ch == '/' && i + 1 < len && chars[i + 1] == '*' {
            in_block_comment = true;
            i += 2;
            col += 2;
            prev_char = '*';
            continue;
        }

        // Block comment end
        if in_block_comment && ch == '*' && i + 1 < len && chars[i + 1] == '/' {
            in_block_comment = false;
            i += 2;
            col += 2;
            prev_char = '/';
            continue;
        }

        if in_block_comment || in_line_comment {
            prev_char = ch;
            i += 1;
            col += 1;
            continue;
        }

        // Line comment
        if !in_string && ch == '/' && i + 1 < len && chars[i + 1] == '/' {
            in_line_comment = true;
            prev_char = ch;
            i += 1;
            col += 1;
            continue;
        }
        if !in_string && ch == '#' {
            // Python/shell/yaml comments (but not inside strings)
            in_line_comment = true;
            prev_char = ch;
            i += 1;
            col += 1;
            continue;
        }

        // String handling
        if !in_string && (ch == '"' || ch == '\'' || ch == '`') {
            in_string = true;
            string_char = ch;
            prev_char = ch;
            i += 1;
            col += 1;
            continue;
        }
        if in_string {
            if ch == '\\' {
                // Skip escaped char
                i += 2;
                col += 2;
                prev_char = if i > 0 && i - 1 < len { chars[i - 1] } else { ch };
                continue;
            }
            if ch == string_char {
                in_string = false;
            }
            prev_char = ch;
            i += 1;
            col += 1;
            continue;
        }

        // Bracket matching
        match ch {
            '(' | '[' | '{' => {
                stack.push((ch, line, col));
            }
            ')' | ']' | '}' => {
                let expected = match ch {
                    ')' => '(',
                    ']' => '[',
                    '}' => '{',
                    _ => unreachable!(),
                };
                if let Some(&(open, _, _)) = stack.last() {
                    if open == expected {
                        stack.pop();
                    } else {
                        errors.push(SyntaxError {
                            line,
                            col,
                            length: 1,
                            message: format!("Mismatched '{}', expected closing for '{}'", ch, open),
                        });
                    }
                } else {
                    errors.push(SyntaxError {
                        line,
                        col,
                        length: 1,
                        message: format!("Unexpected '{}'", ch),
                    });
                }
            }
            _ => {}
        }

        prev_char = ch;
        i += 1;
        col += 1;
    }

    // Remaining unclosed brackets
    for (ch, l, c) in stack {
        errors.push(SyntaxError {
            line: l,
            col: c,
            length: 1,
            message: format!("Unclosed '{}'", ch),
        });
    }
}

fn check_strings(content: &str, language: &str, errors: &mut Vec<SyntaxError>) {
    let lines: Vec<&str> = content.lines().collect();
    let quote_chars: &[char] = match language {
        "python" | "py" => &['"', '\''],
        "json" => &['"'],
        _ => &['"', '\''],
    };

    for (line_idx, line_str) in lines.iter().enumerate() {
        let chars: Vec<char> = line_str.chars().collect();
        let len = chars.len();
        let mut i = 0;
        let mut in_line_comment = false;

        while i < len {
            // Skip comments
            if !in_line_comment && i + 1 < len && chars[i] == '/' && chars[i + 1] == '/' {
                break;
            }
            if !in_line_comment && chars[i] == '#' && language != "css" && language != "scss" {
                break;
            }

            // Check for string
            if quote_chars.contains(&chars[i]) {
                let quote = chars[i];
                // Skip template literals (backtick) — can be multiline
                if quote == '`' {
                    i += 1;
                    continue;
                }
                let start_col = i;
                i += 1;
                let mut closed = false;
                while i < len {
                    if chars[i] == '\\' {
                        i += 2;
                        continue;
                    }
                    if chars[i] == quote {
                        closed = true;
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                if !closed {
                    // Check if it's a triple-quote (Python)
                    if language == "python" || language == "py" {
                        // Don't flag — could be multiline
                    } else {
                        errors.push(SyntaxError {
                            line: line_idx,
                            col: start_col,
                            length: len - start_col,
                            message: format!("Unclosed string"),
                        });
                    }
                }
                continue;
            }
            i += 1;
        }
    }
}
