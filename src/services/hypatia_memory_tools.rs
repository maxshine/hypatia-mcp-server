use super::combined_server::CombinedServer;
use hypatia;
use rmcp::{
    ErrorData,
    handler::server::wrapper::{Json, Parameters},
    schemars, serde,
    serde_json::{Value, json},
    tool, tool_router,
};
use std::collections::HashMap;

// type ToolResult = Result<Value, String>;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct ShelfNameParams {
    shelf_name: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct GenericNameArgs {
    name: String,
    shelf: Option<String>,
}

#[expect(dead_code, reason = "reserved for a future field/value tool")]
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct GenericNameValueArgs {
    name: String,
    value: String,
    shelf: Option<String>,
}

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
struct ShelfList {
    shelves: Vec<String>,
}

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
struct ShelfStatus {
    name: String,
    postgres: bool,
    embedder: String,
    semantic_search_off: String,
    attention: String,
    has_debt: bool,
}

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
struct ModelList {
    models: Vec<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct KnowledgeParameter {
    name: String,
    data: Option<String>,
    tags: Option<Vec<String>>,
    synonyms: Option<Vec<String>>,
    figures: Option<Vec<String>>,
    scopes: Option<Vec<String>>,
    embed: Option<bool>,
    shelf: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct PositionalSynonyms {
    head: Option<Vec<String>>,
    relation: Option<Vec<String>>,
    tail: Option<Vec<String>>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct StatementParameter {
    head: String,
    relation: String,
    tail: String,
    data: Option<String>,
    synonyms: Option<PositionalSynonyms>,
    scopes: Option<Vec<String>>,
    embed: Option<bool>,
    shelf: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct StatementTripleRequestParameter {
    head: String,
    relation: String,
    tail: String,
    shelf: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct QueryRequestParameter {
    /// JSE query, e.g. ["$knowledge", ["$has", "tags", "rule"]] or
    /// {"$statement": [["$triple", "Alice", "$*", "$*"]], "limit": 10}
    jse: Value,
    shelf: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct SearchRequestParameter {
    query: String,
    catalog: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
    shelf: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct SimilarityRequestParameter {
    query: String,
    target: Option<String>,
    limit: Option<i64>,
    tags: Option<Vec<String>>,
    exclude_tags: Option<Vec<String>>,
    /// JSE condition applied before ranking, e.g. ["$has", "scopes", "my-app"];
    /// $search, $similar and $k-hop are not allowed
    #[serde(rename = "where")]
    condition: Option<Value>,
    shelf: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct BackfillRequestParameter {
    limit: Option<usize>,
    shelf: Option<String>,
}

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
struct ToolResult {
    result: Value,
}
const DEFAULT_LIMIT: i64 = 100;
const BACKFILL_DEFAULT_LIMIT: usize = 64;
const BACKFILL_MAX_LIMIT: usize = 512;

pub fn dirs_home() -> std::path::PathBuf {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
}

fn clean_list(items: Vec<String>) -> Vec<String> {
    items
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Trimmed scopes without duplicates; `""` stays, since it marks the global scope.
fn clean_scopes(items: Vec<String>) -> Vec<String> {
    let mut scopes: Vec<String> = Vec::new();
    for scope in items {
        let scope = scope.trim().to_string();
        if !scopes.contains(&scope) {
            scopes.push(scope);
        }
    }
    scopes
}

fn flat_synonyms(items: Vec<String>) -> Option<hypatia::model::Synonyms> {
    let list = clean_list(items);
    if list.is_empty() {
        None
    } else {
        Some(hypatia::model::Synonyms::Flat(list))
    }
}

/// Per-position synonyms, cleaned; `None` when every position is empty.
fn positional_synonyms(synonyms: PositionalSynonyms) -> Option<hypatia::model::Synonyms> {
    let mut map = HashMap::new();
    for (position, items) in [
        ("head", synonyms.head),
        ("relation", synonyms.relation),
        ("tail", synonyms.tail),
    ] {
        let items = clean_list(items.unwrap_or_default());
        if !items.is_empty() {
            map.insert(position.to_string(), items);
        }
    }
    if map.is_empty() {
        None
    } else {
        Some(hypatia::model::Synonyms::Positional(map))
    }
}

fn debt(lab: &hypatia::lab::Lab, shelf: &str) -> Value {
    lab.embedding_debt(shelf)
        .ok()
        .and_then(|debt| rmcp::serde_json::to_value(debt).ok())
        .unwrap_or(Value::Null)
}

/// The schema's minimums, enforced: a negative limit would mean "everything" on some paths and
/// "nothing" on others.
fn paging(limit: Option<i64>, offset: Option<i64>) -> Result<(i64, i64), String> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT);
    let offset = offset.unwrap_or(0);
    if limit < 1 {
        return Err(format!("limit must be at least 1, got {limit}"));
    }
    if offset < 0 {
        return Err(format!("offset must not be negative, got {offset}"));
    }
    Ok((limit, offset))
}

/// A JSE argument, which hosts send either as JSON or as a JSON string.
fn jse_arg(value: Value, name: &str) -> Result<Value, String> {
    match value {
        Value::String(text) => {
            rmcp::serde_json::from_str(&text).map_err(|e| format!("invalid {name} JSON: {e}"))
        }
        other => Ok(other),
    }
}

fn rows(result: hypatia::model::QueryResult) -> Value {
    json!({ "rows": result.rows, "total_count": result.total_count })
}

fn knowledge_json(k: &hypatia::model::Knowledge) -> Value {
    json!({ "name": k.name, "content": k.content, "created_at": k.created_at.to_string() })
}

#[tool_router(router = memory_router, vis = "pub")]
impl CombinedServer {
    #[tool(description = "Connect to a hypatia memory shelf")]
    fn connect_shelf(
        &self,
        Parameters(ShelfNameParams { shelf_name }): Parameters<ShelfNameParams>,
    ) -> Result<String, ErrorData> {
        let mut hypatia_lab = hypatia::lab::Lab::new().map_err(|e| {
            ErrorData::internal_error(format!("Failed to create hypatia lab: {e}"), None)
        })?;
        let shelf_name = self.resolve_shelf(shelf_name);
        let shelf_path = dirs_home().join(".hypatia").join(&shelf_name);
        let result = hypatia_lab
            .connect_shelf(&shelf_path, Some(shelf_name.as_str()))
            .map_err(|e| {
                ErrorData::internal_error(format!("Failed to connect shelf: {e}"), None)
            })?;
        Ok(format!("{} hypatia shelf connected", result))
    }
    #[tool(description = "List available hypatia memory shelves")]
    fn list_shelves(&self) -> Result<Json<ShelfList>, ErrorData> {
        let hypatia_lab = hypatia::lab::Lab::new().map_err(|e| {
            ErrorData::internal_error(format!("Failed to create hypatia lab: {e}"), None)
        })?;
        let shelves = hypatia_lab.list_shelves();
        let ret: Vec<String> = shelves
            .iter()
            .map(|(name, _, _)| name.to_string())
            .collect();
        Ok(Json(ShelfList { shelves: ret }))
    }
    #[tool(description = "Disconnect a hypatia memory shelf from registry")]
    fn disconnect_shelf(
        &self,
        Parameters(ShelfNameParams { shelf_name }): Parameters<ShelfNameParams>,
    ) -> Result<String, ErrorData> {
        let mut hypatia_lab = hypatia::lab::Lab::new().map_err(|e| {
            ErrorData::internal_error(format!("Failed to create hypatia lab: {e}"), None)
        })?;
        let shelf_name = self.resolve_shelf(shelf_name);
        hypatia_lab
            .disconnect_shelf(shelf_name.as_str())
            .map_err(|e| {
                ErrorData::internal_error(format!("Failed to disconnect shelf: {e}"), None)
            })?;
        Ok(format!("{} hypatia shelf disconnected", shelf_name))
    }

    #[tool(description = "Show a shelf general status")]
    fn get_shelf_status(
        &self,
        Parameters(ShelfNameParams { shelf_name }): Parameters<ShelfNameParams>,
    ) -> Result<Json<ShelfStatus>, ErrorData> {
        let hypatia_lab = hypatia::lab::Lab::new().map_err(|e| {
            ErrorData::internal_error(format!("Failed to create hypatia lab: {e}"), None)
        })?;
        let shelf_name = self.resolve_shelf(shelf_name);
        let status = hypatia_lab.shelf_status(&shelf_name).map_err(|e| {
            ErrorData::internal_error(format!("Failed to get shelf status: {e}"), None)
        })?;
        Ok(Json(ShelfStatus {
            name: status.name,
            postgres: status.postgres,
            embedder: status.embedder,
            semantic_search_off: status.semantic_search_off.unwrap_or_default(),
            attention: status.attention.unwrap_or_default(),
            has_debt: status.debt.pending_knowledge > 0 || status.debt.pending_statement > 0,
        }))
    }

    #[tool(description = "List local models available for hypatia memory")]
    fn list_models(&self) -> Result<Json<ModelList>, ErrorData> {
        let models: Vec<String> = hypatia::embedding::config::list_local_models()
            .into_iter()
            .map(|(name, _)| name.to_string())
            .collect();
        Ok(Json(ModelList { models }))
    }

    #[tool(description = "List archives for a hypatia memory shelf")]
    fn list_archives(
        &self,
        Parameters(ShelfNameParams { shelf_name }): Parameters<ShelfNameParams>,
    ) -> Result<Json<ToolResult>, ErrorData> {
        let shelf = self.resolve_shelf(shelf_name);
        let mut hypatia_lab = hypatia::lab::Lab::new().map_err(|e| {
            ErrorData::internal_error(format!("Failed to create hypatia lab: {e}"), None)
        })?;
        hypatia_lab
            .flush_if_overdue(&shelf)
            .map_err(|e| ErrorData::internal_error(format!("Failed to flush shelf: {e}"), None))?;
        let files: Vec<String> = hypatia_lab
            .list_archives(&shelf)
            .map_err(|e| ErrorData::internal_error(format!("Failed to list archives: {e}"), None))?
            .into_iter()
            .map(|f| format!("archive://{f}"))
            .collect();
        Ok(Json(ToolResult {
            result: json!({ "files": files }),
        }))
    }

    #[tool(description = "Create a knowledge record for a hypatia memory shelf")]
    fn create_knowledge(
        &self,
        Parameters(KnowledgeParameter {
            name,
            data,
            tags,
            synonyms,
            figures,
            scopes,
            embed,
            shelf,
        }): Parameters<KnowledgeParameter>,
    ) -> Result<Json<ToolResult>, ErrorData> {
        let mut hypatia_lab = hypatia::lab::Lab::new().map_err(|e| {
            ErrorData::internal_error(format!("Failed to create hypatia lab: {e}"), None)
        })?;
        let shelf = self.resolve_shelf(shelf);
        hypatia_lab.flush_if_overdue(&shelf).map_err(|e| {
            ErrorData::internal_error(format!("Failed to flush shelf embedding state: {e}"), None)
        })?;
        match hypatia_lab.get_knowledge(&shelf, &name) {
            Ok(Some(_)) => {
                return Err(ErrorData::invalid_request(
                    format!(
                        "knowledge '{}' already exists; use knowledge_update to change it",
                        name
                    ),
                    None,
                ));
            }
            Ok(None) => {}
            Err(e) => {
                return Err(ErrorData::internal_error(format!("Failed to check knowledge: {e}"), None));
            }
        }
        let content = hypatia::model::Content::new(data.unwrap_or_default())
            .with_tags(clean_list(tags.unwrap_or_default()))
            .with_synonyms(flat_synonyms(synonyms.unwrap_or_default()))
            .with_figures(clean_list(figures.unwrap_or_default()))
            .with_scopes(clean_scopes(scopes.unwrap_or_default()))
            .with_embed(embed);
        let k = hypatia_lab
            .create_knowledge(&shelf, &name, content)
            .map_err(|e| {
                ErrorData::internal_error(format!("Failed to create knowledge: {e}"), None)
            })?;
        Ok(Json(ToolResult {
            result: json!({ "name": k.name, "created": true, "embedding": debt(&mut hypatia_lab, &shelf) }),
        }))
    }

    #[tool(description = "Get a knowledge record from a hypatia memory shelf")]
    fn get_knowledge(
        &self,
        Parameters(GenericNameArgs { name, shelf }): Parameters<GenericNameArgs>,
    ) -> Result<Json<ToolResult>, ErrorData> {
        let mut hypatia_lab = hypatia::lab::Lab::new().map_err(|e| {
            ErrorData::internal_error(format!("Failed to create hypatia lab: {e}"), None)
        })?;
        let shelf = self.resolve_shelf(shelf);
        hypatia_lab.flush_if_overdue(&shelf).map_err(|e| {
            ErrorData::internal_error(format!("Failed to flush shelf embedding state: {e}"), None)
        })?;
        match hypatia_lab
            .get_knowledge(&shelf, &name)
            .map_err(|e| ErrorData::internal_error(format!("Failed to get knowledge: {e}"), None))?
        {
            Some(k) => Ok(Json(ToolResult {
                result: knowledge_json(&k),
            })),
            None => Ok(Json(ToolResult {
                result: json!({
                    "error": format!("not found: knowledge '{}'", name)
                }),
            })),
        }
    }

    #[tool(description = "Update a knowledge record in a hypatia memory shelf")]
    fn update_knowledge(
        &self,
        Parameters(KnowledgeParameter {
            name,
            data,
            tags,
            synonyms,
            figures,
            scopes,
            embed,
            shelf,
        }): Parameters<KnowledgeParameter>,
    ) -> Result<Json<ToolResult>, ErrorData> {
        let mut hypatia_lab = hypatia::lab::Lab::new().map_err(|e| {
            ErrorData::internal_error(format!("Failed to create hypatia lab: {e}"), None)
        })?;
        let shelf = self.resolve_shelf(shelf);
        let patch = hypatia::service::KnowledgePatch {
            data,
            tags: tags.map(clean_list),
            synonyms: synonyms.map(flat_synonyms),
            figures: figures.map(clean_list),
            scopes: scopes.map(clean_scopes),
            embed,
        };
        if patch.is_empty() {
            return Err(ErrorData::invalid_request(
                "nothing to update: pass at least one of data, tags, synonyms, figures, scopes, embed".to_string(),
                None,
            ));
        }
        hypatia_lab.flush_if_overdue(&shelf).map_err(|e| {
            ErrorData::internal_error(format!("Failed to flush shelf embedding state: {e}"), None)
        })?;
        let updated = hypatia_lab
            .patch_knowledge(&shelf, &name, &patch)
            .map_err(|e| {
                ErrorData::internal_error(format!("Failed to patch knowledge: {e}"), None)
            })?;
        Ok(Json(ToolResult {
            result: json!({
                "name": updated.knowledge.name,
                "changed": updated.changed,
                "embedding": debt(&mut hypatia_lab, &shelf)
            }),
        }))
    }

    #[tool(description = "Delete a knowledge record in a hypatia memory shelf")]
    fn delete_knowledge(
        &self,
        Parameters(GenericNameArgs { name, shelf }): Parameters<GenericNameArgs>,
    ) -> Result<Json<ToolResult>, ErrorData> {
        let mut hypatia_lab = hypatia::lab::Lab::new().map_err(|e| {
            ErrorData::internal_error(format!("Failed to create hypatia lab: {e}"), None)
        })?;
        let shelf = self.resolve_shelf(shelf);
        hypatia_lab.flush_if_overdue(&shelf).map_err(|e| {
            ErrorData::internal_error(format!("Failed to flush shelf embedding state: {e}"), None)
        })?;
        hypatia_lab.delete_knowledge(&shelf, &name).map_err(|e| {
            ErrorData::internal_error(format!("Failed to delete knowledge: {e}"), None)
        })?;
        Ok(Json(ToolResult {
            result: json!({ "name": name, "deleted": true }),
        }))
    }

    #[tool(description = "Create a statement in a hypatia memory shelf")]
    fn create_statement(
        &self,
        Parameters(StatementParameter {
            head,
            relation,
            tail,
            data,
            synonyms,
            scopes,
            embed,
            shelf,
        }): Parameters<StatementParameter>,
    ) -> Result<Json<ToolResult>, ErrorData> {
        let mut hypatia_lab = hypatia::lab::Lab::new().map_err(|e| {
            ErrorData::internal_error(format!("Failed to create hypatia lab: {e}"), None)
        })?;
        let shelf = self.resolve_shelf(shelf);
        hypatia_lab.flush_if_overdue(&shelf).map_err(|e| {
            ErrorData::internal_error(format!("Failed to flush shelf embedding state: {e}"), None)
        })?;
        let key = hypatia::model::StatementKey::new(&head, &relation, &tail);
        let content = hypatia::model::Content::new(data.unwrap_or_default())
            .with_synonyms(synonyms.and_then(positional_synonyms))
            .with_scopes(clean_scopes(scopes.unwrap_or_default()))
            .with_embed(embed);
        let outcome = hypatia_lab
            .create_statement(&shelf, &key, content, None, None)
            .map_err(|e| {
                ErrorData::internal_error(format!("Failed to create statement: {e}"), None)
            })?;
        Ok(Json(ToolResult {
            result: json!({
            "head": key.head,
            "relation": key.relation,
            "tail": key.tail,
            "created": outcome.created,
            "embedding": debt(&mut hypatia_lab, &shelf)
            }),
        }))
    }

    #[tool(description = "Delete a statement in a hypatia memory shelf")]
    fn delete_statement(
        &self,
        Parameters(StatementTripleRequestParameter {
            head,
            relation,
            tail,
            shelf,
        }): Parameters<StatementTripleRequestParameter>,
    ) -> Result<Json<ToolResult>, ErrorData> {
        let mut hypatia_lab = hypatia::lab::Lab::new().map_err(|e| {
            ErrorData::internal_error(format!("Failed to create hypatia lab: {e}"), None)
        })?;
        let shelf = self.resolve_shelf(shelf);
        hypatia_lab.flush_if_overdue(&shelf).map_err(|e| {
            ErrorData::internal_error(format!("Failed to flush shelf embedding state: {e}"), None)
        })?;
        let key = hypatia::model::StatementKey::new(&head, &relation, &tail);
        hypatia_lab.delete_statement(&shelf, &key).map_err(|e| {
            ErrorData::internal_error(format!("Failed to delete statement: {e}"), None)
        })?;
        Ok(Json(ToolResult {
            result: json!({ "head": key.head, "relation": key.relation, "tail": key.tail, "deleted": true }),
        }))
    }

    #[tool(description = "Query knowledges and statements from a hypatia memory shelf")]
    fn query(
        &self,
        Parameters(QueryRequestParameter { jse, shelf }): Parameters<QueryRequestParameter>,
    ) -> Result<Json<ToolResult>, ErrorData> {
        let mut hypatia_lab = hypatia::lab::Lab::new().map_err(|e| {
            ErrorData::internal_error(format!("Failed to create hypatia lab: {e}"), None)
        })?;
        let shelf = self.resolve_shelf(shelf);
        hypatia_lab.flush_if_overdue(&shelf).map_err(|e| {
            ErrorData::internal_error(format!("Failed to flush shelf embedding state: {e}"), None)
        })?;
        let jse = jse_arg(jse, "JSE").unwrap_or_default();
        let semantic = hypatia::lab::uses_similar(&jse);
        let mut out =
            rows(hypatia_lab.query(&shelf, &jse).map_err(|e| {
                ErrorData::internal_error(format!("Failed to query shelf: {e}"), None)
            })?);
        if semantic {
            out["embedding"] = debt(&mut hypatia_lab, &shelf);
        }
        Ok(Json(ToolResult { result: out }))
    }

    #[tool(description = "Search for knowledges and statements in a hypatia memory shelf")]
    fn search(
        &self,
        Parameters(SearchRequestParameter {
            query,
            catalog,
            shelf,
            limit,
            offset,
        }): Parameters<SearchRequestParameter>,
    ) -> Result<Json<ToolResult>, ErrorData> {
        let mut hypatia_lab = hypatia::lab::Lab::new().map_err(|e| {
            ErrorData::internal_error(format!("Failed to create hypatia lab: {e}"), None)
        })?;
        let shelf = self.resolve_shelf(shelf);
        hypatia_lab.flush_if_overdue(&shelf).map_err(|e| {
            ErrorData::internal_error(format!("Failed to flush shelf embedding state: {e}"), None)
        })?;
        let (limit, offset) = paging(limit, offset).map_err(|e| {
            ErrorData::internal_error(format!("Failed to get pagination setting: {e}"), None)
        })?;
        let opts = hypatia::model::SearchOpts {
            catalog,
            limit,
            offset,
        };
        Ok(Json(ToolResult {
            result: rows(hypatia_lab.search(&shelf, &query, opts).map_err(|e| {
                ErrorData::internal_error(format!("Failed to search shelf: {e}"), None)
            })?),
        }))
    }

    #[tool(description = "Find similar knowledges and statements in a hypatia memory shelf")]
    fn similar(
        &self,
        Parameters(SimilarityRequestParameter {
            query,
            target,
            limit,
            shelf,
            condition,
            tags,
            exclude_tags,
        }): Parameters<SimilarityRequestParameter>,
    ) -> Result<Json<ToolResult>, ErrorData> {
        let (limit, _) = paging(limit, None).map_err(|e| {
            ErrorData::internal_error(format!("Failed to get pagination setting: {e}"), None)
        })?;
        let shelf = self.resolve_shelf(shelf);
        let mut hypatia_lab = hypatia::lab::Lab::new().map_err(|e| {
            ErrorData::internal_error(format!("Failed to create hypatia lab: {e}"), None)
        })?;
        let condition = condition
            .map(|c| jse_arg(c, "where"))
            .transpose()
            .map_err(|e| {
                ErrorData::internal_error(format!("Failed to parse condition: {e}"), None)
            })?;
        hypatia_lab.flush_if_overdue(&shelf).map_err(|e| {
            ErrorData::internal_error(format!("Failed to flush shelf embedding state: {e}"), None)
        })?;
        let target = target.unwrap_or_else(|| "both".to_string());
        let filter = hypatia::engine::filter::similar_filter(
            &clean_list(tags.unwrap_or_default()),
            &clean_list(exclude_tags.unwrap_or_default()),
            condition,
        );
        let result = hypatia_lab
            .similar_where(&shelf, &query, &target, limit, filter.as_ref())
            .map_err(|e| {
                ErrorData::internal_error(format!("Failed to find similar items: {e}"), None)
            })?;
        let mut out = rows(result);
        out["embedding"] = debt(&mut hypatia_lab, &shelf);
        Ok(Json(ToolResult { result: out }))
    }

    #[tool(description = "Backfill knowledges and statements in a hypatia memory shelf")]
    fn backfill_shelf(
        &self,
        Parameters(BackfillRequestParameter { limit, shelf }): Parameters<BackfillRequestParameter>,
    ) -> Result<Json<ToolResult>, ErrorData> {
        let mut hypatia_lab = hypatia::lab::Lab::new().map_err(|e| {
            ErrorData::internal_error(format!("Failed to create hypatia lab: {e}"), None)
        })?;
        let shelf = self.resolve_shelf(shelf);
        let limit = limit.unwrap_or(BACKFILL_DEFAULT_LIMIT);
        if !(1..=BACKFILL_MAX_LIMIT).contains(&limit) {
            return Err(ErrorData::internal_error(
                format!("limit must be between 1 and {BACKFILL_MAX_LIMIT}, got {limit}"),
                None,
            ));
        }
        let stats = hypatia_lab.backfill_batch(&shelf, limit).map_err(|e| {
            ErrorData::internal_error(format!("Failed to backfill shelf: {e}"), None)
        })?;
        let result = json!({
            "installed": stats.installed,
            "skipped": stats.skipped,
            "failed": stats.failed,
            "error": stats.error,
            "embedding": debt(&mut hypatia_lab, &shelf)
        });
        Ok(Json(ToolResult { result: result }))
    }

    #[tool(description = "List available tags and their occurrences for a shelf")]
    fn list_shelf_tags(
        &self,
        Parameters(ShelfNameParams { shelf_name }): Parameters<ShelfNameParams>,
    ) -> Result<Json<ToolResult>, ErrorData> {
        let hypatia_lab = hypatia::lab::Lab::new().map_err(|e| {
            ErrorData::internal_error(format!("Failed to create hypatia lab: {e}"), None)
        })?;
        let shelf_name = self.resolve_shelf(shelf_name);
        let tag_tuples = hypatia_lab
            .field_values(&shelf_name, "tags")
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        let tags: HashMap<String, i64> = tag_tuples.into_iter().collect();
        return Ok(Json(ToolResult {
            result: json!({
                "shelf_name": shelf_name,
                "tags": tags,
            }),
        }));
    }

    #[tool(description = "Check a tag's existence in the available tags for a shelf")]
    fn check_shelf_tag_existence(
        &self,
        Parameters(GenericNameArgs { name, shelf }): Parameters<GenericNameArgs>,
    ) -> Result<Json<ToolResult>, ErrorData> {
        let hypatia_lab = hypatia::lab::Lab::new().map_err(|e| {
            ErrorData::internal_error(format!("Failed to create hypatia lab: {e}"), None)
        })?;
        let shelf_name = self.resolve_shelf(shelf);
        let tag_existence = hypatia_lab
            .field_value_exists(&shelf_name, "tags", &name)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        if tag_existence {
            return Ok(Json(ToolResult {
                result: json!({
                    "shelf_name": shelf_name,
                    "tag_value": name,
                    "exists": true
                }),
            }));
        }
        return Ok(Json(ToolResult {
            result: json!({
                "shelf_name": shelf_name,
                "tag_value": name,
                "exists": false,
            }),
        }));
    }

    #[tool(description = "List available scopes and their occurrences for a shelf")]
    fn list_shelf_scopes(
        &self,
        Parameters(ShelfNameParams { shelf_name }): Parameters<ShelfNameParams>,
    ) -> Result<Json<ToolResult>, ErrorData> {
        let hypatia_lab = hypatia::lab::Lab::new().map_err(|e| {
            ErrorData::internal_error(format!("Failed to create hypatia lab: {e}"), None)
        })?;
        let shelf_name = self.resolve_shelf(shelf_name);
        let scope_tuples = hypatia_lab
            .field_values(&shelf_name, "scopes")
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        let scopes: HashMap<String, i64> = scope_tuples.into_iter().collect();
        return Ok(Json(ToolResult {
            result: json!({
                "shelf_name": shelf_name,
                "scopes": scopes,
            }),
        }));
    }

    #[tool(description = "Check a scope's existence in the available scopes for a shelf")]
    fn check_shelf_scope_existence(
        &self,
        Parameters(GenericNameArgs { name, shelf }): Parameters<GenericNameArgs>,
    ) -> Result<Json<ToolResult>, ErrorData> {
        let hypatia_lab = hypatia::lab::Lab::new().map_err(|e| {
            ErrorData::internal_error(format!("Failed to create hypatia lab: {e}"), None)
        })?;
        let shelf_name = self.resolve_shelf(shelf);
        let scope_existence = hypatia_lab
            .field_value_exists(&shelf_name, "scopes", &name)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        if scope_existence {
            return Ok(Json(ToolResult {
                result: json!({
                    "shelf_name": shelf_name,
                    "scope_value": name,
                    "exists": true
                }),
            }));
        }
        return Ok(Json(ToolResult {
            result: json!({
                "shelf_name": shelf_name,
                "scope_value": name,
                "exists": false,
            }),
        }));
    }
}
