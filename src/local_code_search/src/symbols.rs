//! Symbol extraction from source code using regex patterns.

use crate::index::{SymbolInfo, SymbolKind};
use regex::Regex;

pub fn extract_symbols(content: &str, language: &str) -> Vec<SymbolInfo> {
    match language {
        "rust" => extract_rust_symbols(content),
        "ts" | "typescript" => extract_typescript_symbols(content),
        "js" | "javascript" => extract_javascript_symbols(content),
        "py" | "python" => extract_python_symbols(content),
        _ => extract_generic_symbols(content),
    }
}

fn extract_rust_symbols(content: &str) -> Vec<SymbolInfo> {
    let mut symbols = Vec::new();

    let fn_re = Regex::new(r"(?m)^(\s*(?:pub\s+)?(?:async\s+)?fn\s+(\w+)\s*\()").unwrap();
    for cap in fn_re.captures_iter(content) {
        let name = cap.get(2).map(|m| m.as_str()).unwrap_or("");
        let line = content[..cap.get(1).unwrap().start()].lines().count() as u32 + 1;
        symbols.push(SymbolInfo {
            name: name.to_string(),
            kind: SymbolKind::Function,
            line_start: line,
            line_end: line,
        });
    }

    let struct_re = Regex::new(r"(?m)^(\s*(?:pub\s+)?struct\s+(\w+))").unwrap();
    for cap in struct_re.captures_iter(content) {
        let name = cap.get(2).map(|m| m.as_str()).unwrap_or("");
        let line = content[..cap.get(1).unwrap().start()].lines().count() as u32 + 1;
        symbols.push(SymbolInfo {
            name: name.to_string(),
            kind: SymbolKind::Struct,
            line_start: line,
            line_end: line,
        });
    }

    let trait_re = Regex::new(r"(?m)^(\s*(?:pub\s+)?trait\s+(\w+))").unwrap();
    for cap in trait_re.captures_iter(content) {
        let name = cap.get(2).map(|m| m.as_str()).unwrap_or("");
        let line = content[..cap.get(1).unwrap().start()].lines().count() as u32 + 1;
        symbols.push(SymbolInfo {
            name: name.to_string(),
            kind: SymbolKind::Trait,
            line_start: line,
            line_end: line,
        });
    }

    let enum_re = Regex::new(r"(?m)^(\s*(?:pub\s+)?enum\s+(\w+))").unwrap();
    for cap in enum_re.captures_iter(content) {
        let name = cap.get(2).map(|m| m.as_str()).unwrap_or("");
        let line = content[..cap.get(1).unwrap().start()].lines().count() as u32 + 1;
        symbols.push(SymbolInfo {
            name: name.to_string(),
            kind: SymbolKind::Enum,
            line_start: line,
            line_end: line,
        });
    }

    symbols
}

fn extract_typescript_symbols(content: &str) -> Vec<SymbolInfo> {
    let mut symbols = Vec::new();

    let fn_re = Regex::new(r"(?m)^(\s*(?:export\s+)?(?:async\s+)?function\s+(\w+))").unwrap();
    for cap in fn_re.captures_iter(content) {
        let name = cap.get(2).map(|m| m.as_str()).unwrap_or("");
        let line = content[..cap.get(1).unwrap().start()].lines().count() as u32 + 1;
        symbols.push(SymbolInfo {
            name: name.to_string(),
            kind: SymbolKind::Function,
            line_start: line,
            line_end: line,
        });
    }

    let class_re = Regex::new(r"(?m)^(\s*(?:export\s+)?class\s+(\w+))").unwrap();
    for cap in class_re.captures_iter(content) {
        let name = cap.get(2).map(|m| m.as_str()).unwrap_or("");
        let line = content[..cap.get(1).unwrap().start()].lines().count() as u32 + 1;
        symbols.push(SymbolInfo {
            name: name.to_string(),
            kind: SymbolKind::Class,
            line_start: line,
            line_end: line,
        });
    }

    let interface_re = Regex::new(r"(?m)^(\s*(?:export\s+)?interface\s+(\w+))").unwrap();
    for cap in interface_re.captures_iter(content) {
        let name = cap.get(2).map(|m| m.as_str()).unwrap_or("");
        let line = content[..cap.get(1).unwrap().start()].lines().count() as u32 + 1;
        symbols.push(SymbolInfo {
            name: name.to_string(),
            kind: SymbolKind::Interface,
            line_start: line,
            line_end: line,
        });
    }

    symbols
}

fn extract_javascript_symbols(content: &str) -> Vec<SymbolInfo> {
    extract_typescript_symbols(content)
}

fn extract_python_symbols(content: &str) -> Vec<SymbolInfo> {
    let mut symbols = Vec::new();

    let fn_re = Regex::new(r"(?m)^(\s*def\s+(\w+)\s*\()").unwrap();
    for cap in fn_re.captures_iter(content) {
        let name = cap.get(2).map(|m| m.as_str()).unwrap_or("");
        let line = content[..cap.get(1).unwrap().start()].lines().count() as u32 + 1;
        symbols.push(SymbolInfo {
            name: name.to_string(),
            kind: SymbolKind::Function,
            line_start: line,
            line_end: line,
        });
    }

    let class_re = Regex::new(r"(?m)^(\s*class\s+(\w+))").unwrap();
    for cap in class_re.captures_iter(content) {
        let name = cap.get(2).map(|m| m.as_str()).unwrap_or("");
        let line = content[..cap.get(1).unwrap().start()].lines().count() as u32 + 1;
        symbols.push(SymbolInfo {
            name: name.to_string(),
            kind: SymbolKind::Class,
            line_start: line,
            line_end: line,
        });
    }

    symbols
}

fn extract_generic_symbols(content: &str) -> Vec<SymbolInfo> {
    let fn_re = Regex::new(r"(?m)^(\s*(?:function|def|fn|func)\s+(\w+))").unwrap();
    let mut symbols = Vec::new();
    for cap in fn_re.captures_iter(content) {
        let name = cap.get(2).map(|m| m.as_str()).unwrap_or("");
        let line = content[..cap.get(1).unwrap().start()].lines().count() as u32 + 1;
        symbols.push(SymbolInfo {
            name: name.to_string(),
            kind: SymbolKind::Function,
            line_start: line,
            line_end: line,
        });
    }
    symbols
}

pub fn detect_language(path: &str) -> &'static str {
    let ext = path.rsplit('.').next().unwrap_or("");
    match ext {
        "rs" => "rust",
        "ts" => "typescript",
        "tsx" => "typescript",
        "js" => "javascript",
        "jsx" => "javascript",
        "py" => "python",
        "go" => "go",
        "java" => "java",
        "c" => "c",
        "cpp" => "cpp",
        "h" => "c",
        "hpp" => "cpp",
        "rb" => "ruby",
        "swift" => "swift",
        "kt" => "kotlin",
        "scala" => "scala",
        "sh" => "shell",
        "bash" => "shell",
        "zsh" => "shell",
        "sql" => "sql",
        "html" => "html",
        "css" => "css",
        "json" => "json",
        "yaml" => "yaml",
        "yml" => "yaml",
        "toml" => "toml",
        "md" => "markdown",
        "txt" => "text",
        _ => "unknown",
    }
}

pub fn should_index_language(configured_languages: &[String], detected: &str) -> bool {
    if configured_languages.is_empty() {
        return true;
    }
    for lang in configured_languages {
        if lang == "all" {
            return true;
        }
        if lang == detected {
            return true;
        }
        if lang == "ts" && detected == "typescript" {
            return true;
        }
        if lang == "js" && detected == "javascript" {
            return true;
        }
        if lang == "py" && detected == "python" {
            return true;
        }
    }
    false
}
