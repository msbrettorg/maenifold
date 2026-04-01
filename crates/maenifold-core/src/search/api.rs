use crate::db::{self, GraphDatabase};
use crate::error::{MaenifoldError, Result};
use crate::search::display::load_help_doc;
use crate::search::fulltext::perform_fulltext_search;
use crate::search::hybrid::perform_hybrid_search;
use crate::search::semantic::perform_semantic_search;
use crate::search::text::{informative_query_terms, slugify_query, tokenize_query};
use crate::tools::ToolContext;
use crate::tools::with_hint;
use serde_json::Value;

pub(crate) struct SearchRequest {
    pub(crate) query: String,
    pub(crate) mode: String,
    pub(crate) page_size: usize,
    pub(crate) page: usize,
    pub(crate) min_score: f64,
    pub(crate) folder_prefix: Option<String>,
    pub(crate) requested_tags: Option<Vec<String>>,
}

impl SearchRequest {
    fn from_payload(payload: &Value) -> Result<Self> {
        let query = payload
            .get("query")
            .and_then(Value::as_str)
            .ok_or(MaenifoldError::MissingProperty("query"))?
            .to_string();
        let mode = payload
            .get("mode")
            .and_then(Value::as_str)
            .unwrap_or("Hybrid")
            .to_string();
        let page_size = payload
            .get("pageSize")
            .and_then(Value::as_u64)
            .unwrap_or(10) as usize;
        let page = payload.get("page").and_then(Value::as_u64).unwrap_or(1) as usize;
        let min_score = payload
            .get("minScore")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        let folder_prefix = payload
            .get("folder")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|folder| !folder.is_empty())
            .map(|folder| format!("memory://{}", folder.trim_matches('/')));
        let requested_tags = payload
            .get("tags")
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::trim)
                    .filter(|tag| !tag.is_empty())
                    .map(|tag| tag.to_ascii_lowercase())
                    .collect::<Vec<_>>()
            })
            .filter(|tags| !tags.is_empty());

        Ok(Self {
            query,
            mode,
            page_size,
            page,
            min_score,
            folder_prefix,
            requested_tags,
        })
    }

    pub(crate) fn folder_prefix(&self) -> Option<&str> {
        self.folder_prefix.as_deref()
    }

    pub(crate) fn requested_tags(&self) -> Option<&[String]> {
        self.requested_tags.as_deref()
    }

    fn is_semantic(&self) -> bool {
        self.mode.eq_ignore_ascii_case("semantic")
    }

    fn is_fulltext(&self) -> bool {
        self.mode.eq_ignore_ascii_case("fulltext") || self.mode.eq_ignore_ascii_case("full-text")
    }
}

pub fn search_memories(context: &ToolContext, payload: &Value) -> Result<String> {
    let learn = payload
        .get("learn")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if learn {
        return Ok(load_help_doc(&context.config.assets_path, "SearchMemories")
            .unwrap_or_else(|| "ERROR: Help file not found for SearchMemories".to_string()));
    }

    let query = payload
        .get("query")
        .and_then(Value::as_str)
        .ok_or(MaenifoldError::MissingProperty("query"))?;
    if informative_query_terms(query).is_empty() {
        return Ok(with_hint(
            "ERROR: Query contains no informative keywords. Try adding specific terms (e.g. \"machine learning\", \"NLP\").",
            "SearchMemories",
            "Provide at least one non-stopword keyword",
        ));
    }

    let request = SearchRequest::from_payload(payload)?;
    GraphDatabase::initialize_database(&context.config)?;
    let connection = db::open_connection(&context.config.database_path)?;
    let query_terms = tokenize_query(&request.query);
    let query_slug = slugify_query(&request.query);

    if request.is_semantic() {
        return perform_semantic_search(context, &connection, &request, &query_terms, &query_slug);
    }
    if request.is_fulltext() {
        return perform_fulltext_search(context, &connection, &request, &query_terms, &query_slug);
    }

    perform_hybrid_search(context, &connection, &request, &query_terms, &query_slug)
}
