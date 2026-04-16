//! Integration tests for Local Code Search Service.

use local_code_search::index::{CodeIndex, SymbolKind};
use local_code_search::search::SearchService;
use local_code_search::symbols::{detect_language, extract_symbols};
use local_code_search::LocalCodeSearchConfig;
use tantivy::doc;
use tempfile::TempDir;

#[test]
fn test_symbol_extraction_rust() {
    let code = r#"
pub struct User {
    pub name: String,
}

pub fn create_user(name: String) -> User {
    User { name }
}

trait Greeter {
    fn greet(&self);
}

enum Status {
    Active,
    Inactive,
}
"#;
    let symbols = extract_symbols(code, "rust");
    let names: Vec<_> = symbols.iter().map(|s| s.name.as_str()).collect();

    assert!(names.contains(&"User"));
    assert!(names.contains(&"create_user"));
    assert!(names.contains(&"Greeter"));
    assert!(names.contains(&"Status"));
}

#[test]
fn test_symbol_extraction_typescript() {
    let code = r#"
export interface User {
    name: string;
}

export class UserService {
    getUser(id: string): User {
        return { name: "test" };
    }
}

export function createUser(name: string): User {
    return { name };
}
"#;
    let symbols = extract_symbols(code, "typescript");
    assert_eq!(symbols.len(), 3);

    let names: Vec<_> = symbols.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"UserService"));
    assert!(names.contains(&"createUser"));
    assert!(names.contains(&"User"));
}

#[test]
fn test_symbol_extraction_python() {
    let code = r#"
class UserService:
    def __init__(self):
        pass
    
    def get_user(self, user_id):
        return {"name": "test"}

def create_user(name):
    return {"name": name}
"#;
    let symbols = extract_symbols(code, "python");
    let names: Vec<_> = symbols.iter().map(|s| s.name.as_str()).collect();

    assert!(names.contains(&"UserService"));
    assert!(names.contains(&"__init__"));
    assert!(names.contains(&"get_user"));
    assert!(names.contains(&"create_user"));
}

#[test]
fn test_language_detection() {
    assert_eq!(detect_language("file.rs"), "rust");
    assert_eq!(detect_language("file.ts"), "typescript");
    assert_eq!(detect_language("file.tsx"), "typescript");
    assert_eq!(detect_language("file.js"), "javascript");
    assert_eq!(detect_language("file.jsx"), "javascript");
    assert_eq!(detect_language("file.py"), "python");
    assert_eq!(detect_language("file.go"), "go");
    assert_eq!(detect_language("file.unknown"), "unknown");
}

#[test]
fn test_config_defaults() {
    let config = LocalCodeSearchConfig::default();
    assert_eq!(config.service.port, 3004);
    assert_eq!(config.service.bind_address, "127.0.0.1");
    assert!(config.service.watch_enabled);
    assert_eq!(config.service.watch_debounce_ms, 500);
    assert!(config.indexes.is_empty());
}

#[test]
fn test_index_creation_and_search() {
    let temp_dir = TempDir::new().unwrap();
    let index_dir = temp_dir.path().join("indexes");
    std::fs::create_dir_all(&index_dir).unwrap();

    // Create index
    let mut index = CodeIndex::open_or_create("test-index", &index_dir).unwrap();

    // Add a test document
    let title_field = index.title_field();
    let content_field = index.content_field();
    let file_path_field = index.file_path_field();
    let relative_path_field = index.relative_path_field();
    let index_name_field = index.index_name_field();
    let language_field = index.language_field();
    let line_number_field = index.line_number_field();
    let content_hash_field = index.content_hash_field();
    let indexed_at_field = index.indexed_at_field();
    let symbols_field = index.symbols_field();

    let content = "fn main() { println!(\"Hello, world!\"); }";
    let symbols = extract_symbols(content, "rust");
    let symbols_json = serde_json::to_string(&symbols).unwrap();

    index
        .writer()
        .add_document(tantivy::doc!(
            title_field => "hello.rs".to_string(),
            content_field => content.to_string(),
            file_path_field => "/test/hello.rs".to_string(),
            relative_path_field => "hello.rs".to_string(),
            index_name_field => "test-index".to_string(),
            language_field => "rust".to_string(),
            line_number_field => 0u64,
            content_hash_field => "abc123".to_string(),
            indexed_at_field => tantivy::DateTime::from_timestamp_secs(chrono::Utc::now().timestamp()),
            symbols_field => symbols_json,
        ))
        .unwrap();

    index.commit().unwrap();

    // Search
    let results = SearchService::search(&index.index, "hello", 10);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title, "hello.rs");
    assert!(!results[0].content.is_empty());
}

#[test]
fn test_symbol_search() {
    let temp_dir = TempDir::new().unwrap();
    let index_dir = temp_dir.path().join("indexes");
    std::fs::create_dir_all(&index_dir).unwrap();

    let mut index = CodeIndex::open_or_create("test-index", &index_dir).unwrap();

    let title_field = index.title_field();
    let content_field = index.content_field();
    let file_path_field = index.file_path_field();
    let relative_path_field = index.relative_path_field();
    let index_name_field = index.index_name_field();
    let language_field = index.language_field();
    let line_number_field = index.line_number_field();
    let content_hash_field = index.content_hash_field();
    let indexed_at_field = index.indexed_at_field();
    let symbols_field = index.symbols_field();

    let content = r#"
pub struct UserManager {
    pub name: String,
}

impl UserManager {
    pub fn create_user(&self) -> User {
        User { name: "test".to_string() }
    }
}
"#;
    let symbols = extract_symbols(content, "rust");
    let symbols_json = serde_json::to_string(&symbols).unwrap();

    index
        .writer()
        .add_document(tantivy::doc!(
            title_field => "user_manager.rs".to_string(),
            content_field => content.to_string(),
            file_path_field => "/test/user_manager.rs".to_string(),
            relative_path_field => "user_manager.rs".to_string(),
            index_name_field => "test-index".to_string(),
            language_field => "rust".to_string(),
            line_number_field => 0u64,
            content_hash_field => "def456".to_string(),
            indexed_at_field => tantivy::DateTime::from_timestamp_secs(chrono::Utc::now().timestamp()),
            symbols_field => symbols_json,
        ))
        .unwrap();

    index.commit().unwrap();

    // Symbol search for "UserManager"
    let results = SearchService::symbol_search(&index.index, "UserManager", 10);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].symbols.len(), 1);
    assert_eq!(results[0].symbols[0].name, "UserManager");
    assert_eq!(results[0].symbols[0].kind, SymbolKind::Struct);
}

#[test]
fn test_should_index_language() {
    use local_code_search::symbols::should_index_language;

    // Empty config means index all
    assert!(should_index_language(&[], "rust"));

    // "all" means index all
    assert!(should_index_language(&["all".to_string()], "rust"));
    assert!(should_index_language(&["all".to_string()], "python"));

    // Specific languages
    assert!(should_index_language(&["rust".to_string()], "rust"));
    assert!(!should_index_language(&["rust".to_string()], "python"));

    // Alias handling
    assert!(should_index_language(&["ts".to_string()], "typescript"));
    assert!(should_index_language(&["js".to_string()], "javascript"));
    assert!(should_index_language(&["py".to_string()], "python"));
}
