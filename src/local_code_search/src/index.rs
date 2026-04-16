//! Code index management using tantivy.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tantivy::{
    doc, query::QueryParser, schema::*, Index, IndexReader, IndexWriter, ReloadPolicy, Term,
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum IndexError {
    #[error("Tantivy error: {0}")]
    Tantivy(#[from] tantivy::TantivyError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Index not found: {0}")]
    NotFound(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolInfo {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: SymbolKind,
    pub line_start: u32,
    pub line_end: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SymbolKind {
    Function,
    Class,
    Trait,
    Interface,
    Struct,
    Enum,
    Module,
    Constant,
    Variable,
    Method,
    Property,
    Type,
    Impl,
}

impl SymbolKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            SymbolKind::Function => "function",
            SymbolKind::Class => "class",
            SymbolKind::Trait => "trait",
            SymbolKind::Interface => "interface",
            SymbolKind::Struct => "struct",
            SymbolKind::Enum => "enum",
            SymbolKind::Module => "module",
            SymbolKind::Constant => "constant",
            SymbolKind::Variable => "variable",
            SymbolKind::Method => "method",
            SymbolKind::Property => "property",
            SymbolKind::Type => "type",
            SymbolKind::Impl => "impl",
        }
    }
}

pub fn build_schema() -> Schema {
    let mut sb = Schema::builder();
    sb.add_text_field("title", TEXT | STORED);
    sb.add_text_field("content", TEXT | STORED);
    sb.add_text_field("file_path", STRING | STORED);
    sb.add_text_field("relative_path", TEXT | STORED);
    sb.add_text_field("index_name", STRING | STORED);
    sb.add_text_field("language", STRING | STORED);
    sb.add_u64_field("line_number", STORED);
    sb.add_text_field("content_hash", STRING | STORED);
    sb.add_date_field("indexed_at", STORED);
    sb.add_text_field("symbols", TEXT | STORED);
    sb.build()
}

pub struct CodeIndex {
    pub name: String,
    pub path: PathBuf,
    pub index: Index,
    reader: IndexReader,
    writer: IndexWriter,
    schema: Schema,
}

impl CodeIndex {
    pub fn open_or_create(name: &str, index_dir: &Path) -> Result<Self, IndexError> {
        let index_path = index_dir.join(name);
        std::fs::create_dir_all(&index_path)?;

        let schema = build_schema();
        let index = Index::create_in_dir(&index_path, schema.clone())
            .or_else(|_| Index::open_in_dir(&index_path))?;

        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;

        let writer = index.writer(50_000_000)?;

        Ok(Self {
            name: name.to_string(),
            path: index_path,
            index,
            reader,
            writer,
            schema,
        })
    }

    pub fn title_field(&self) -> Field {
        self.schema.get_field("title").unwrap()
    }
    pub fn content_field(&self) -> Field {
        self.schema.get_field("content").unwrap()
    }
    pub fn file_path_field(&self) -> Field {
        self.schema.get_field("file_path").unwrap()
    }
    pub fn relative_path_field(&self) -> Field {
        self.schema.get_field("relative_path").unwrap()
    }
    pub fn index_name_field(&self) -> Field {
        self.schema.get_field("index_name").unwrap()
    }
    pub fn language_field(&self) -> Field {
        self.schema.get_field("language").unwrap()
    }
    pub fn line_number_field(&self) -> Field {
        self.schema.get_field("line_number").unwrap()
    }
    pub fn content_hash_field(&self) -> Field {
        self.schema.get_field("content_hash").unwrap()
    }
    pub fn indexed_at_field(&self) -> Field {
        self.schema.get_field("indexed_at").unwrap()
    }
    pub fn symbols_field(&self) -> Field {
        self.schema.get_field("symbols").unwrap()
    }

    pub fn query_parser(&self) -> QueryParser {
        QueryParser::for_index(&self.index, vec![self.title_field(), self.content_field()])
    }

    pub fn reader(&self) -> &IndexReader {
        &self.reader
    }
    pub fn writer(&mut self) -> &mut IndexWriter {
        &mut self.writer
    }

    pub fn commit(&mut self) -> Result<(), IndexError> {
        self.writer.commit()?;
        Ok(())
    }

    pub fn delete_by_hash(&mut self, hash: &str) -> Result<(), IndexError> {
        let field = self.content_hash_field();
        self.writer.delete_term(Term::from_field_text(field, hash));
        self.writer.commit()?;
        Ok(())
    }

    pub fn stats(&self) -> IndexStats {
        let searcher = self.reader.searcher();
        IndexStats {
            name: self.name.clone(),
            num_docs: searcher.num_docs(),
            num_segments: searcher.segment_readers().len() as u64,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStats {
    pub name: String,
    pub num_docs: u64,
    pub num_segments: u64,
}
