/// Built-in code snippets (Zed gets these from LSP, we provide built-in ones)
pub struct Snippet {
    pub trigger: &'static str,  // what user types
    pub label: &'static str,    // display name
    pub body: &'static str,     // template with $1, $2, $0
}

pub fn get_snippets(language: &str) -> Vec<Snippet> {
    match language {
        "rust" => vec![
            Snippet { trigger: "fn", label: "fn function", body: "fn $1($2) {\n    $0\n}" },
            Snippet { trigger: "pfn", label: "pub fn function", body: "pub fn $1($2) {\n    $0\n}" },
            Snippet { trigger: "test", label: "#[test] fn", body: "#[test]\nfn $1() {\n    $0\n}" },
            Snippet { trigger: "impl", label: "impl block", body: "impl $1 {\n    $0\n}" },
            Snippet { trigger: "struct", label: "struct definition", body: "struct $1 {\n    $0\n}" },
            Snippet { trigger: "enum", label: "enum definition", body: "enum $1 {\n    $0\n}" },
            Snippet { trigger: "match", label: "match expression", body: "match $1 {\n    $0\n}" },
            Snippet { trigger: "if", label: "if block", body: "if $1 {\n    $0\n}" },
            Snippet { trigger: "ifl", label: "if let", body: "if let $1 = $2 {\n    $0\n}" },
            Snippet { trigger: "for", label: "for loop", body: "for $1 in $2 {\n    $0\n}" },
            Snippet { trigger: "while", label: "while loop", body: "while $1 {\n    $0\n}" },
            Snippet { trigger: "loop", label: "loop", body: "loop {\n    $0\n}" },
            Snippet { trigger: "println", label: "println!", body: "println!(\"$1\", $0);" },
            Snippet { trigger: "vec", label: "vec![]", body: "vec![$0]" },
            Snippet { trigger: "derive", label: "#[derive()]", body: "#[derive($0)]" },
            Snippet { trigger: "mod", label: "mod block", body: "mod $1 {\n    $0\n}" },
            Snippet { trigger: "use", label: "use statement", body: "use $0;" },
        ],
        "javascript" | "typescript" | "jsx" | "tsx" => vec![
            Snippet { trigger: "fn", label: "function", body: "function $1($2) {\n    $0\n}" },
            Snippet { trigger: "afn", label: "arrow function", body: "const $1 = ($2) => {\n    $0\n};" },
            Snippet { trigger: "if", label: "if block", body: "if ($1) {\n    $0\n}" },
            Snippet { trigger: "ife", label: "if-else", body: "if ($1) {\n    $0\n} else {\n    \n}" },
            Snippet { trigger: "for", label: "for loop", body: "for (let $1 = 0; $1 < $2; $1++) {\n    $0\n}" },
            Snippet { trigger: "forof", label: "for...of", body: "for (const $1 of $2) {\n    $0\n}" },
            Snippet { trigger: "forin", label: "for...in", body: "for (const $1 in $2) {\n    $0\n}" },
            Snippet { trigger: "while", label: "while loop", body: "while ($1) {\n    $0\n}" },
            Snippet { trigger: "try", label: "try-catch", body: "try {\n    $0\n} catch ($1) {\n    \n}" },
            Snippet { trigger: "cl", label: "console.log", body: "console.log($0);" },
            Snippet { trigger: "imp", label: "import", body: "import { $0 } from '$1';" },
            Snippet { trigger: "exp", label: "export", body: "export $0" },
            Snippet { trigger: "class", label: "class", body: "class $1 {\n    constructor($2) {\n        $0\n    }\n}" },
            Snippet { trigger: "switch", label: "switch", body: "switch ($1) {\n    case $2:\n        $0\n        break;\n}" },
            Snippet { trigger: "async", label: "async function", body: "async function $1($2) {\n    $0\n}" },
            Snippet { trigger: "await", label: "await", body: "await $0" },
            Snippet { trigger: "map", label: ".map()", body: ".map(($1) => $0)" },
            Snippet { trigger: "filter", label: ".filter()", body: ".filter(($1) => $0)" },
        ],
        "python" => vec![
            Snippet { trigger: "def", label: "def function", body: "def $1($2):\n    $0" },
            Snippet { trigger: "class", label: "class", body: "class $1:\n    def __init__(self$2):\n        $0" },
            Snippet { trigger: "if", label: "if block", body: "if $1:\n    $0" },
            Snippet { trigger: "ife", label: "if-else", body: "if $1:\n    $0\nelse:\n    " },
            Snippet { trigger: "for", label: "for loop", body: "for $1 in $2:\n    $0" },
            Snippet { trigger: "while", label: "while loop", body: "while $1:\n    $0" },
            Snippet { trigger: "try", label: "try-except", body: "try:\n    $0\nexcept $1:\n    pass" },
            Snippet { trigger: "with", label: "with statement", body: "with $1 as $2:\n    $0" },
            Snippet { trigger: "print", label: "print()", body: "print($0)" },
            Snippet { trigger: "imp", label: "import", body: "import $0" },
            Snippet { trigger: "from", label: "from import", body: "from $1 import $0" },
            Snippet { trigger: "main", label: "if __name__", body: "if __name__ == '__main__':\n    $0" },
        ],
        "go" => vec![
            Snippet { trigger: "fn", label: "func", body: "func $1($2) $3 {\n    $0\n}" },
            Snippet { trigger: "main", label: "func main", body: "func main() {\n    $0\n}" },
            Snippet { trigger: "if", label: "if block", body: "if $1 {\n    $0\n}" },
            Snippet { trigger: "ife", label: "if-else", body: "if $1 {\n    $0\n} else {\n    \n}" },
            Snippet { trigger: "iferr", label: "if err != nil", body: "if err != nil {\n    $0\n}" },
            Snippet { trigger: "for", label: "for loop", body: "for $1 {\n    $0\n}" },
            Snippet { trigger: "forr", label: "for range", body: "for $1, $2 := range $3 {\n    $0\n}" },
            Snippet { trigger: "switch", label: "switch", body: "switch $1 {\ncase $2:\n    $0\n}" },
            Snippet { trigger: "struct", label: "struct", body: "type $1 struct {\n    $0\n}" },
            Snippet { trigger: "interface", label: "interface", body: "type $1 interface {\n    $0\n}" },
            Snippet { trigger: "fmt", label: "fmt.Println", body: "fmt.Println($0)" },
        ],
        "php" => vec![
            Snippet { trigger: "fn", label: "function", body: "function $1($2) {\n    $0\n}" },
            Snippet { trigger: "class", label: "class", body: "class $1 {\n    $0\n}" },
            Snippet { trigger: "if", label: "if block", body: "if ($1) {\n    $0\n}" },
            Snippet { trigger: "for", label: "for loop", body: "for ($1 = 0; $1 < $2; $1++) {\n    $0\n}" },
            Snippet { trigger: "foreach", label: "foreach", body: "foreach ($1 as $2) {\n    $0\n}" },
            Snippet { trigger: "echo", label: "echo", body: "echo $0;" },
            Snippet { trigger: "try", label: "try-catch", body: "try {\n    $0\n} catch (\\Exception $1) {\n    \n}" },
            Snippet { trigger: "pub", label: "public function", body: "public function $1($2) {\n    $0\n}" },
        ],
        "html" => vec![
            Snippet { trigger: "html", label: "HTML5 boilerplate", body: "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n    <meta charset=\"UTF-8\">\n    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n    <title>$1</title>\n</head>\n<body>\n    $0\n</body>\n</html>" },
            Snippet { trigger: "div", label: "<div>", body: "<div$1>\n    $0\n</div>" },
            Snippet { trigger: "a", label: "<a href>", body: "<a href=\"$1\">$0</a>" },
            Snippet { trigger: "img", label: "<img>", body: "<img src=\"$1\" alt=\"$0\">" },
            Snippet { trigger: "ul", label: "<ul>", body: "<ul>\n    <li>$0</li>\n</ul>" },
            Snippet { trigger: "input", label: "<input>", body: "<input type=\"$1\" name=\"$2\" $0>" },
            Snippet { trigger: "link", label: "<link css>", body: "<link rel=\"stylesheet\" href=\"$0\">" },
            Snippet { trigger: "script", label: "<script>", body: "<script src=\"$0\"></script>" },
        ],
        "css" | "scss" => vec![
            Snippet { trigger: "flex", label: "display: flex", body: "display: flex;\njustify-content: $1;\nalign-items: $0;" },
            Snippet { trigger: "grid", label: "display: grid", body: "display: grid;\ngrid-template-columns: $0;" },
            Snippet { trigger: "media", label: "@media query", body: "@media (max-width: $1px) {\n    $0\n}" },
            Snippet { trigger: "var", label: "CSS variable", body: "var(--$0)" },
            Snippet { trigger: "trans", label: "transition", body: "transition: $1 $2s ease$0;" },
        ],
        _ => vec![],
    }
}

/// Expand snippet body — replace $1,$2 with empty, $0 marks final cursor position
/// Returns (expanded_text, cursor_offset_from_start)
pub fn expand_snippet(body: &str, indent: &str) -> (String, usize) {
    let mut result = String::new();
    let mut cursor_pos = None;
    let mut i = 0;
    let chars: Vec<char> = body.chars().collect();

    while i < chars.len() {
        if chars[i] == '$' && i + 1 < chars.len() {
            if chars[i + 1] == '0' {
                cursor_pos = Some(result.len());
                i += 2;
            } else if chars[i + 1].is_ascii_digit() {
                // Skip tab stop markers ($1, $2, etc.)
                i += 2;
            } else if chars[i + 1] == '{' {
                // Skip ${1:placeholder} — just insert placeholder text
                if let Some(colon) = chars[i+2..].iter().position(|&c| c == ':') {
                    let start = i + 2 + colon + 1;
                    if let Some(end) = chars[start..].iter().position(|&c| c == '}') {
                        let placeholder: String = chars[start..start+end].iter().collect();
                        result.push_str(&placeholder);
                        i = start + end + 1;
                    } else {
                        i += 2;
                    }
                } else {
                    i += 2;
                }
            } else {
                result.push('$');
                i += 1;
            }
        } else if chars[i] == '\n' {
            result.push('\n');
            result.push_str(indent);
            i += 1;
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    let pos = cursor_pos.unwrap_or(result.len());
    (result, pos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_snippets_exist() {
        let snippets = get_snippets("rust");
        assert!(snippets.len() > 10);
        assert!(snippets.iter().any(|s| s.trigger == "fn"));
        assert!(snippets.iter().any(|s| s.trigger == "struct"));
    }

    #[test]
    fn js_snippets_exist() {
        let snippets = get_snippets("javascript");
        assert!(snippets.iter().any(|s| s.trigger == "cl"));
        assert!(snippets.iter().any(|s| s.trigger == "afn"));
    }

    #[test]
    fn python_snippets_exist() {
        let snippets = get_snippets("python");
        assert!(snippets.iter().any(|s| s.trigger == "def"));
        assert!(snippets.iter().any(|s| s.trigger == "class"));
    }

    #[test]
    fn unknown_language_empty() {
        let snippets = get_snippets("brainfuck");
        assert!(snippets.is_empty());
    }

    #[test]
    fn expand_simple() {
        let (text, pos) = expand_snippet("hello $0 world", "");
        assert_eq!(text, "hello  world");
        assert_eq!(pos, 6);
    }

    #[test]
    fn expand_with_indent() {
        let (text, _) = expand_snippet("if $1 {\n    $0\n}", "    ");
        assert!(text.contains("    ")); // indent preserved
    }

    #[test]
    fn expand_with_placeholder() {
        let (text, _) = expand_snippet("fn ${1:name}() {}", "");
        assert!(text.contains("name"));
    }

    #[test]
    fn expand_no_cursor() {
        let (text, pos) = expand_snippet("hello", "");
        assert_eq!(text, "hello");
        assert_eq!(pos, 5); // cursor at end
    }
}
