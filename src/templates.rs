/// File templates by extension — inserted when creating new files
pub fn get_template(filename: &str) -> Option<&'static str> {
    let ext = std::path::Path::new(filename)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    match ext {
        "html" | "htm" => Some(HTML_TEMPLATE),
        "vue" => Some(VUE_TEMPLATE),
        "svelte" => Some(SVELTE_TEMPLATE),
        "tsx" => Some(TSX_COMPONENT),
        "jsx" => Some(JSX_COMPONENT),
        "py" => Some(PYTHON_TEMPLATE),
        "rs" => Some(""),  // Rust — empty (cargo manages)
        "sh" | "bash" => Some(SHELL_TEMPLATE),
        "php" => Some(PHP_TEMPLATE),
        "css" => Some(CSS_TEMPLATE),
        "json" => Some(JSON_TEMPLATE),
        "yaml" | "yml" => Some(YAML_TEMPLATE),
        "md" => Some(MARKDOWN_TEMPLATE),
        "dockerfile" => Some(DOCKERFILE_TEMPLATE),
        _ => {
            // Check by filename
            let name = std::path::Path::new(filename)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
            match name.to_lowercase().as_str() {
                "dockerfile" | "containerfile" => Some(DOCKERFILE_TEMPLATE),
                ".gitignore" => Some(GITIGNORE_TEMPLATE),
                ".env" | ".env.local" | ".env.example" => Some(ENV_TEMPLATE),
                ".editorconfig" => Some(EDITORCONFIG_TEMPLATE),
                "docker-compose.yml" | "docker-compose.yaml" => Some(DOCKER_COMPOSE_TEMPLATE),
                _ => None,
            }
        }
    }
}

const HTML_TEMPLATE: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Document</title>
</head>
<body>

</body>
</html>
"#;

const VUE_TEMPLATE: &str = r#"<template>
  <div>

  </div>
</template>

<script setup>

</script>

<style scoped>

</style>
"#;

const SVELTE_TEMPLATE: &str = r#"<script>

</script>

<main>

</main>

<style>

</style>
"#;

const TSX_COMPONENT: &str = r#"export default function Component() {
    return (
        <div>

        </div>
    );
}
"#;

const JSX_COMPONENT: &str = r#"export default function Component() {
    return (
        <div>

        </div>
    );
}
"#;

const PYTHON_TEMPLATE: &str = r#"#!/usr/bin/env python3
"""Module docstring."""


def main():
    pass


if __name__ == "__main__":
    main()
"#;

const SHELL_TEMPLATE: &str = r#"#!/usr/bin/env bash
set -euo pipefail

"#;

const PHP_TEMPLATE: &str = r#"<?php

declare(strict_types=1);

"#;

const CSS_TEMPLATE: &str = r#"* {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
}

"#;

const JSON_TEMPLATE: &str = r#"{

}
"#;

const YAML_TEMPLATE: &str = r#"---

"#;

const MARKDOWN_TEMPLATE: &str = r#"# Title

"#;

const DOCKERFILE_TEMPLATE: &str = r#"FROM node:20-alpine

WORKDIR /app

COPY package*.json ./
RUN npm install

COPY . .

EXPOSE 3000
CMD ["npm", "start"]
"#;

const GITIGNORE_TEMPLATE: &str = r#"# Dependencies
node_modules/
vendor/

# Build
dist/
build/
target/

# Environment
.env
.env.local

# IDE
.idea/
.vscode/
*.swp
*.swo

# OS
.DS_Store
Thumbs.db
"#;

const ENV_TEMPLATE: &str = r#"# Application
APP_NAME=
APP_ENV=development
APP_PORT=3000

# Database
DB_HOST=localhost
DB_PORT=5432
DB_NAME=
DB_USER=
DB_PASSWORD=

"#;

const EDITORCONFIG_TEMPLATE: &str = r#"root = true

[*]
indent_style = space
indent_size = 4
end_of_line = lf
charset = utf-8
trim_trailing_whitespace = true
insert_final_newline = true

[*.{yml,yaml,json}]
indent_size = 2

[Makefile]
indent_style = tab
"#;

const DOCKER_COMPOSE_TEMPLATE: &str = r#"version: '3.8'

services:
  app:
    build: .
    ports:
      - "3000:3000"
    volumes:
      - .:/app
    environment:
      - NODE_ENV=development
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_template() {
        let t = get_template("index.html");
        assert!(t.is_some());
        assert!(t.unwrap().contains("<!DOCTYPE html>"));
    }

    #[test]
    fn vue_template() {
        let t = get_template("App.vue");
        assert!(t.is_some());
        assert!(t.unwrap().contains("<template>"));
    }

    #[test]
    fn tsx_template() {
        let t = get_template("Component.tsx");
        assert!(t.is_some());
        assert!(t.unwrap().contains("export default"));
    }

    #[test]
    fn python_template() {
        let t = get_template("main.py");
        assert!(t.is_some());
        assert!(t.unwrap().contains("def main"));
    }

    #[test]
    fn dockerfile_by_name() {
        let t = get_template("Dockerfile");
        assert!(t.is_some());
        assert!(t.unwrap().contains("FROM"));
    }

    #[test]
    fn gitignore_template() {
        let t = get_template(".gitignore");
        assert!(t.is_some());
        assert!(t.unwrap().contains("node_modules"));
    }

    #[test]
    fn env_template() {
        let t = get_template(".env");
        assert!(t.is_some());
        assert!(t.unwrap().contains("DB_HOST"));
    }

    #[test]
    fn unknown_extension() {
        assert!(get_template("data.xyz").is_none());
    }

    #[test]
    fn rust_empty() {
        let t = get_template("main.rs");
        assert!(t.is_some());
        assert!(t.unwrap().is_empty()); // Rust = empty
    }

    #[test]
    fn shell_template() {
        let t = get_template("deploy.sh");
        assert!(t.is_some());
        assert!(t.unwrap().contains("#!/usr/bin/env bash"));
    }
}
