use std::path::Path;

use tantivy::{
    collector::TopDocs,
    doc,
    query::QueryParser,
    schema::{Field, Schema, Value, STORED, STRING, TEXT},
    snippet::SnippetGenerator,
    Index, IndexReader, IndexWriter, ReloadPolicy, TantivyDocument, Term,
};

#[derive(Clone)]
pub struct KeywordIndex {
    index: Index,
    reader: IndexReader,
    path_field: Field,
    scope_id_field: Field,
    title_field: Field,
    content_field: Field,
}

#[derive(Debug, Clone)]
pub struct KeywordHit {
    pub scope_id: String,
    pub path: String,
    pub title: String,
    pub snippet: String,
    pub score: f32,
}

impl KeywordIndex {
    pub fn open_or_create(index_dir: &str) -> Result<Self, String> {
        let mut builder = Schema::builder();
        let _ = builder.add_text_field("path", STRING | STORED);
        let _ = builder.add_text_field("scope_id", STRING | STORED);
        let _ = builder.add_text_field("title", TEXT | STORED);
        let _ = builder.add_text_field("content", TEXT | STORED);
        let schema = builder.build();

        let dir = Path::new(index_dir);
        std::fs::create_dir_all(dir).map_err(|e| format!("index_dir_create_failed: {e}"))?;

        let index = Index::open_in_dir(dir)
            .or_else(|_| Index::create_in_dir(dir, schema.clone()))
            .map_err(|e| format!("index_open_failed: {e}"))?;

        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .map_err(|e| format!("index_reader_failed: {e}"))?;

        let schema = index.schema();
        let path_field = schema
            .get_field("path")
            .map_err(|_| "index_schema_missing_path".to_string())?;
        let scope_id_field = schema
            .get_field("scope_id")
            .map_err(|_| "index_schema_missing_scope_id".to_string())?;
        let title_field = schema
            .get_field("title")
            .map_err(|_| "index_schema_missing_title".to_string())?;
        let content_field = schema
            .get_field("content")
            .map_err(|_| "index_schema_missing_content".to_string())?;

        Ok(Self {
            index,
            reader,
            path_field,
            scope_id_field,
            title_field,
            content_field,
        })
    }

    pub fn upsert(&self, scope_id: &str, path: &str, content: &str) -> Result<(), String> {
        let title = Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(path)
            .to_string();

        let mut writer: IndexWriter<TantivyDocument> = self
            .index
            .writer(20_000_000)
            .map_err(|e| format!("index_writer_failed: {e}"))?;

        writer.delete_term(Term::from_field_text(self.path_field, path));
        writer.add_document(doc!(
            self.path_field => path,
            self.scope_id_field => scope_id,
            self.title_field => title,
            self.content_field => content.to_string(),
        ))
        .map_err(|e| format!("index_add_doc_failed: {e}"))?;

        writer
            .commit()
            .map_err(|e| format!("index_commit_failed: {e}"))?;
        Ok(())
    }

    pub fn delete_path(&self, path: &str) -> Result<(), String> {
        let mut writer: IndexWriter<TantivyDocument> = self
            .index
            .writer(20_000_000)
            .map_err(|e| format!("index_writer_failed: {e}"))?;
        writer.delete_term(Term::from_field_text(self.path_field, path));
        writer
            .commit()
            .map_err(|e| format!("index_commit_failed: {e}"))?;
        Ok(())
    }

    pub fn search(&self, q: &str, limit: usize) -> Result<Vec<KeywordHit>, String> {
        let _ = self.reader.reload();
        let searcher = self.reader.searcher();
        let parser = QueryParser::for_index(
            &self.index,
            vec![self.title_field, self.path_field, self.content_field],
        );
        let query = parser
            .parse_query(q)
            .map_err(|e| format!("index_query_parse_failed: {e}"))?;

        let top_docs = searcher
            .search(&query, &TopDocs::with_limit(limit))
            .map_err(|e| format!("index_search_failed: {e}"))?;

        let mut snippets = SnippetGenerator::create(&searcher, &*query, self.content_field)
            .map_err(|e| format!("index_snippet_generator_failed: {e}"))?;
        snippets.set_max_num_chars(200);

        let mut out = Vec::new();
        for (score, addr) in top_docs {
            let retrieved: TantivyDocument = searcher
                .doc(addr)
                .map_err(|e| format!("index_doc_fetch_failed: {e}"))?;

            let path = retrieved
                .get_first(self.path_field)
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let scope_id = retrieved
                .get_first(self.scope_id_field)
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let title = retrieved
                .get_first(self.title_field)
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let content = retrieved
                .get_first(self.content_field)
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let snippet = snippets.snippet(content).to_html();

            out.push(KeywordHit {
                scope_id,
                path,
                title,
                snippet,
                score,
            });
        }

        Ok(out)
    }
}
