//! File templates — boilerplate that's auto-inserted when the user creates a new file
//! via the sidebar `+` button or the New File context menu. Matched by extension first,
//! then by exact filename. Returns `None` for unknown types (the new file is empty).

pub fn get_template(filename: &str) -> Option<&'static str> {
    let path = std::path::Path::new(filename);
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let lower_name = name.to_lowercase();

    // Filename-based first — beats extension matching for files like Dockerfile,
    // Makefile, Cargo.toml that should always get their canonical boilerplate.
    if let Some(t) = template_by_name(&lower_name) { return Some(t); }
    template_by_ext(ext)
}

fn template_by_name(lower_name: &str) -> Option<&'static str> {
    Some(match lower_name {
        "dockerfile" | "containerfile" => DOCKERFILE_TEMPLATE,
        ".gitignore" => GITIGNORE_TEMPLATE,
        ".gitattributes" => GITATTRIBUTES_TEMPLATE,
        ".env" | ".env.local" | ".env.example" | ".env.development" | ".env.production" => ENV_TEMPLATE,
        ".editorconfig" => EDITORCONFIG_TEMPLATE,
        ".prettierrc" => PRETTIERRC_TEMPLATE,
        ".eslintrc" | ".eslintrc.json" => ESLINTRC_TEMPLATE,
        "docker-compose.yml" | "docker-compose.yaml" => DOCKER_COMPOSE_TEMPLATE,
        "makefile" | "gnumakefile" => MAKEFILE_TEMPLATE,
        "justfile" => JUSTFILE_TEMPLATE,
        "cmakelists.txt" => CMAKELISTS_TEMPLATE,
        "readme.md" => README_TEMPLATE,
        "license" | "license.md" | "license.txt" => LICENSE_TEMPLATE,
        "cargo.toml" => CARGO_TOML_TEMPLATE,
        "package.json" => PACKAGE_JSON_TEMPLATE,
        "tsconfig.json" => TSCONFIG_TEMPLATE,
        "jsconfig.json" => JSCONFIG_TEMPLATE,
        "go.mod" => GOMOD_TEMPLATE,
        "pyproject.toml" => PYPROJECT_TEMPLATE,
        "requirements.txt" => REQUIREMENTS_TEMPLATE,
        "composer.json" => COMPOSER_TEMPLATE,
        "gemfile" => GEMFILE_TEMPLATE,
        "podfile" => PODFILE_TEMPLATE,
        _ => return None,
    })
}

fn template_by_ext(ext: &str) -> Option<&'static str> {
    Some(match ext {
        // Web markup
        "html" | "htm" => HTML_TEMPLATE,
        "vue" => VUE_TEMPLATE,
        "svelte" => SVELTE_TEMPLATE,
        "astro" => ASTRO_TEMPLATE,
        // JS/TS
        "ts" => TS_TEMPLATE,
        "tsx" => TSX_COMPONENT,
        "js" | "mjs" => JS_TEMPLATE,
        "jsx" => JSX_COMPONENT,
        // Systems
        "rs" => RUST_TEMPLATE,
        "go" => GO_TEMPLATE,
        "c" => C_TEMPLATE,
        "h" => C_HEADER_TEMPLATE,
        "cpp" | "cc" | "cxx" => CPP_TEMPLATE,
        "hpp" | "hh" | "hxx" => CPP_HEADER_TEMPLATE,
        "java" => JAVA_TEMPLATE,
        "kt" | "kts" => KOTLIN_TEMPLATE,
        "swift" => SWIFT_TEMPLATE,
        "zig" => ZIG_TEMPLATE,
        "cs" => CSHARP_TEMPLATE,
        // Scripting
        "py" => PYTHON_TEMPLATE,
        "rb" => RUBY_TEMPLATE,
        "lua" => LUA_TEMPLATE,
        "pl" => PERL_TEMPLATE,
        "sh" | "bash" => SHELL_TEMPLATE,
        "ps1" => POWERSHELL_TEMPLATE,
        // Web backend
        "php" => PHP_TEMPLATE,
        "ex" | "exs" => ELIXIR_TEMPLATE,
        "dart" => DART_TEMPLATE,
        // Styling
        "css" => CSS_TEMPLATE,
        "scss" | "sass" => SCSS_TEMPLATE,
        // Data / config
        "json" => JSON_TEMPLATE,
        "yaml" | "yml" => YAML_TEMPLATE,
        "toml" => TOML_TEMPLATE,
        "xml" => XML_TEMPLATE,
        // SQL / db
        "sql" => SQL_TEMPLATE,
        // Docs
        "md" | "mdx" => MARKDOWN_TEMPLATE,
        "tex" => LATEX_TEMPLATE,
        _ => return None,
    })
}

// ─── Web markup ──────────────────────────────────────────────────────────────

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

<script setup lang="ts">

</script>

<style scoped>

</style>
"#;

const SVELTE_TEMPLATE: &str = r#"<script lang="ts">

</script>

<main>

</main>

<style>

</style>
"#;

const ASTRO_TEMPLATE: &str = r#"---
// Component script (server-side)
---

<main>

</main>

<style>

</style>
"#;

// ─── JS / TS ─────────────────────────────────────────────────────────────────

const TS_TEMPLATE: &str = r#"export {};

"#;

const TSX_COMPONENT: &str = r#"interface Props {
}

export default function Component({}: Props) {
    return (
        <div>

        </div>
    );
}
"#;

const JS_TEMPLATE: &str = r#"";

"#;

const JSX_COMPONENT: &str = r#"export default function Component() {
    return (
        <div>

        </div>
    );
}
"#;

// ─── Systems ────────────────────────────────────────────────────────────────

const RUST_TEMPLATE: &str = r#""#;

const GO_TEMPLATE: &str = r#"package main

import "fmt"

func main() {
    fmt.Println("Hello, World!")
}
"#;

const C_TEMPLATE: &str = r#"#include <stdio.h>

int main(int argc, char *argv[]) {
    printf("Hello, World!\n");
    return 0;
}
"#;

const C_HEADER_TEMPLATE: &str = r#"#ifndef HEADER_H
#define HEADER_H

#ifdef __cplusplus
extern "C" {
#endif

#ifdef __cplusplus
}
#endif

#endif // HEADER_H
"#;

const CPP_TEMPLATE: &str = r#"#include <iostream>

int main(int argc, char *argv[]) {
    std::cout << "Hello, World!" << std::endl;
    return 0;
}
"#;

const CPP_HEADER_TEMPLATE: &str = r#"#pragma once

"#;

const JAVA_TEMPLATE: &str = r#"public class Main {
    public static void main(String[] args) {
        System.out.println("Hello, World!");
    }
}
"#;

const KOTLIN_TEMPLATE: &str = r#"fun main() {
    println("Hello, World!")
}
"#;

const SWIFT_TEMPLATE: &str = r#"import Foundation

print("Hello, World!")
"#;

const ZIG_TEMPLATE: &str = r#"const std = @import("std");

pub fn main() !void {
    const stdout = std.io.getStdOut().writer();
    try stdout.print("Hello, {s}!\n", .{"World"});
}
"#;

const CSHARP_TEMPLATE: &str = r#"using System;

namespace MyApp;

public class Program
{
    public static void Main(string[] args)
    {
        Console.WriteLine("Hello, World!");
    }
}
"#;

// ─── Scripting ───────────────────────────────────────────────────────────────

const PYTHON_TEMPLATE: &str = r#"#!/usr/bin/env python3
"""Module docstring."""


def main() -> None:
    pass


if __name__ == "__main__":
    main()
"#;

const RUBY_TEMPLATE: &str = r#"#!/usr/bin/env ruby
# frozen_string_literal: true

"#;

const LUA_TEMPLATE: &str = r#"#!/usr/bin/env lua

"#;

const PERL_TEMPLATE: &str = r#"#!/usr/bin/env perl
use strict;
use warnings;

"#;

const SHELL_TEMPLATE: &str = r#"#!/usr/bin/env bash
set -euo pipefail

"#;

const POWERSHELL_TEMPLATE: &str = r#"#!/usr/bin/env pwsh
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

"#;

// ─── Web backend ─────────────────────────────────────────────────────────────

const PHP_TEMPLATE: &str = r#"<?php

declare(strict_types=1);

"#;

const ELIXIR_TEMPLATE: &str = r#"defmodule Module do
  @moduledoc """
  Documentation for `Module`.
  """
end
"#;

const DART_TEMPLATE: &str = r#"void main() {
  print('Hello, World!');
}
"#;

// ─── Styling ────────────────────────────────────────────────────────────────

const CSS_TEMPLATE: &str = r#"* {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
}

"#;

const SCSS_TEMPLATE: &str = r#"// Variables

// Mixins

// Styles

"#;

// ─── Data / config ──────────────────────────────────────────────────────────

const JSON_TEMPLATE: &str = r#"{

}
"#;

const YAML_TEMPLATE: &str = r#"---

"#;

const TOML_TEMPLATE: &str = r#"# TOML configuration

"#;

const XML_TEMPLATE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>

"#;

const SQL_TEMPLATE: &str = r#"-- SQL script

"#;

const MARKDOWN_TEMPLATE: &str = r#"# Title

"#;

const LATEX_TEMPLATE: &str = r#"\documentclass{article}
\usepackage[utf8]{inputenc}

\title{Title}
\author{Author}
\date{\today}

\begin{document}

\maketitle

\end{document}
"#;

// ─── Project files (by exact name) ──────────────────────────────────────────

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
*.o
*.exe

# Environment
.env
.env.local
.env.*.local

# IDE
.idea/
.vscode/
*.swp
*.swo

# OS
.DS_Store
Thumbs.db
"#;

const GITATTRIBUTES_TEMPLATE: &str = r#"* text=auto eol=lf

*.png binary
*.jpg binary
*.gif binary
*.ico binary
*.pdf binary
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

const PRETTIERRC_TEMPLATE: &str = r#"{
    "semi": true,
    "singleQuote": true,
    "tabWidth": 2,
    "printWidth": 100,
    "trailingComma": "all",
    "arrowParens": "always"
}
"#;

const ESLINTRC_TEMPLATE: &str = r#"{
    "root": true,
    "env": {
        "browser": true,
        "node": true,
        "es2022": true
    },
    "extends": [
        "eslint:recommended"
    ],
    "parserOptions": {
        "ecmaVersion": "latest",
        "sourceType": "module"
    },
    "rules": {}
}
"#;

const DOCKER_COMPOSE_TEMPLATE: &str = r#"services:
  app:
    build: .
    ports:
      - "3000:3000"
    volumes:
      - .:/app
    environment:
      - NODE_ENV=development
"#;

const MAKEFILE_TEMPLATE: &str = "# Makefile\n\n.PHONY: all build test clean\n\nall: build\n\nbuild:\n\n\ntest:\n\n\nclean:\n\n";

const JUSTFILE_TEMPLATE: &str = r#"# Justfile — task runner

default:
    @just --list

build:

test:

clean:
"#;

const CMAKELISTS_TEMPLATE: &str = r#"cmake_minimum_required(VERSION 3.20)
project(my_project VERSION 0.1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

add_executable(${PROJECT_NAME} src/main.cpp)
"#;

const README_TEMPLATE: &str = r#"# Project

Brief description.

## Install

```bash

```

## Usage

```bash

```

## License

MIT
"#;

const LICENSE_TEMPLATE: &str = r#"MIT License

Copyright (c) YEAR YOUR_NAME

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
"#;

const CARGO_TOML_TEMPLATE: &str = r#"[package]
name = "my_crate"
version = "0.1.0"
edition = "2021"

[dependencies]
"#;

const PACKAGE_JSON_TEMPLATE: &str = r#"{
    "name": "my-package",
    "version": "0.1.0",
    "description": "",
    "type": "module",
    "main": "index.js",
    "scripts": {
        "test": "echo \"Error: no test specified\" && exit 1"
    },
    "keywords": [],
    "author": "",
    "license": "MIT"
}
"#;

const TSCONFIG_TEMPLATE: &str = r#"{
    "compilerOptions": {
        "target": "ES2022",
        "module": "ESNext",
        "moduleResolution": "Bundler",
        "strict": true,
        "esModuleInterop": true,
        "skipLibCheck": true,
        "forceConsistentCasingInFileNames": true,
        "resolveJsonModule": true
    },
    "include": ["src/**/*"]
}
"#;

const JSCONFIG_TEMPLATE: &str = r#"{
    "compilerOptions": {
        "target": "ES2022",
        "module": "ESNext",
        "moduleResolution": "Node",
        "baseUrl": "./",
        "paths": {
            "@/*": ["src/*"]
        }
    },
    "include": ["src/**/*"]
}
"#;

const GOMOD_TEMPLATE: &str = r#"module example.com/my-app

go 1.22
"#;

const PYPROJECT_TEMPLATE: &str = r#"[project]
name = "my-package"
version = "0.1.0"
description = ""
requires-python = ">=3.10"
dependencies = []

[build-system]
requires = ["setuptools>=64"]
build-backend = "setuptools.build_meta"
"#;

const REQUIREMENTS_TEMPLATE: &str = r#""#;

const COMPOSER_TEMPLATE: &str = r#"{
    "name": "vendor/package",
    "description": "",
    "type": "library",
    "require": {
        "php": "^8.2"
    },
    "autoload": {
        "psr-4": {
            "App\\": "src/"
        }
    },
    "license": "MIT"
}
"#;

const GEMFILE_TEMPLATE: &str = r#"source "https://rubygems.org"

# Add your gems here
"#;

const PODFILE_TEMPLATE: &str = r#"platform :ios, '15.0'

target 'MyApp' do
  use_frameworks!
end
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_template() {
        let t = get_template("index.html").unwrap();
        assert!(t.contains("<!DOCTYPE html>"));
    }

    #[test]
    fn vue_template() {
        assert!(get_template("App.vue").unwrap().contains("<template>"));
    }

    #[test]
    fn tsx_template() {
        assert!(get_template("Component.tsx").unwrap().contains("export default"));
    }

    #[test]
    fn python_template() {
        assert!(get_template("main.py").unwrap().contains("def main"));
    }

    #[test]
    fn dockerfile_by_name() {
        assert!(get_template("Dockerfile").unwrap().contains("FROM"));
    }

    #[test]
    fn gitignore_template() {
        assert!(get_template(".gitignore").unwrap().contains("node_modules"));
    }

    #[test]
    fn env_template() {
        assert!(get_template(".env").unwrap().contains("DB_HOST"));
    }

    #[test]
    fn unknown_extension() {
        assert!(get_template("data.xyz").is_none());
    }

    #[test]
    fn rust_empty() {
        // Rust source files start empty — `cargo new` handles project scaffolding
        assert_eq!(get_template("main.rs"), Some(""));
    }

    #[test]
    fn shell_template() {
        assert!(get_template("deploy.sh").unwrap().contains("#!/usr/bin/env bash"));
    }

    #[test]
    fn cargo_toml_template() {
        assert!(get_template("Cargo.toml").unwrap().contains("[package]"));
    }

    #[test]
    fn package_json_template() {
        assert!(get_template("package.json").unwrap().contains("\"name\""));
    }

    #[test]
    fn tsconfig_template() {
        assert!(get_template("tsconfig.json").unwrap().contains("compilerOptions"));
    }

    #[test]
    fn readme_template() {
        assert!(get_template("README.md").unwrap().contains("# Project"));
    }

    #[test]
    fn license_template() {
        assert!(get_template("LICENSE").unwrap().contains("MIT License"));
    }

    #[test]
    fn cmakelists_template() {
        assert!(get_template("CMakeLists.txt").unwrap().contains("cmake_minimum_required"));
    }

    #[test]
    fn java_template() {
        assert!(get_template("Main.java").unwrap().contains("public class"));
    }

    #[test]
    fn swift_template() {
        assert!(get_template("App.swift").unwrap().contains("import Foundation"));
    }

    #[test]
    fn kotlin_template() {
        assert!(get_template("App.kt").unwrap().contains("fun main"));
    }

    #[test]
    fn go_template() {
        assert!(get_template("main.go").unwrap().contains("package main"));
    }

    #[test]
    fn go_mod_template() {
        assert!(get_template("go.mod").unwrap().contains("module"));
    }

    #[test]
    fn cpp_template() {
        assert!(get_template("main.cpp").unwrap().contains("#include <iostream>"));
    }

    #[test]
    fn ruby_template() {
        assert!(get_template("script.rb").unwrap().contains("frozen_string_literal"));
    }

    #[test]
    fn makefile_template() {
        assert!(get_template("Makefile").unwrap().contains(".PHONY"));
    }

    #[test]
    fn name_match_beats_extension() {
        // Cargo.toml uses name-based template, not the generic .toml one
        assert!(get_template("Cargo.toml").unwrap().contains("[package]"));
        // While random .toml files get the generic header
        assert!(get_template("config.toml").unwrap().contains("# TOML"));
    }
}
