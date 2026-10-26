use super::combined_server::CombinedServer;
use super::hypatia_memory_tools::dirs_home;
use hypatia;
use rmcp::{
    ErrorData,
    handler::server::wrapper::{Json, Parameters},
    serde_json::{Map, Value, from_str, json, to_string},
    tool, tool_router,
};
use std::{
    fs::OpenOptions,
    io::{Read, Write},
};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

pub fn hypatia_home() -> std::path::PathBuf {
    dirs_home().join(".hypatia")
}

pub fn hypatia_hook_session_dir() -> std::path::PathBuf {
    hypatia_home().join("sessions")
}

pub fn hypatia_hook_hook_log() -> std::path::PathBuf {
    hypatia_home().join("hook.log")
}

/// Author of a logged message; each role gets its own entry per turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageRole {
    User,
    Assistant,
}

impl MessageRole {
    fn suffix(self) -> &'static str {
        match self {
            MessageRole::User => "u",
            MessageRole::Assistant => "a",
        }
    }
}

pub fn format_message_name(session_id: &str, turn_id: &str, role: MessageRole) -> String {
    format!("msg_{}_{}_{}", session_id, turn_id, role.suffix())
}

fn parse_tags(raw: &str) -> Vec<String> {
    if raw.is_empty() {
        Vec::new()
    } else {
        raw.split(',').map(|s| s.trim().to_string()).collect()
    }
}

/// A comma-separated list with blank items dropped; `""` gives an empty list.
fn parse_list(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Comma-separated knowledge synonyms; `""` gives none.
fn parse_flat_synonyms(raw: &str) -> Option<hypatia::model::content::Synonyms> {
    let list = parse_list(raw);
    if list.is_empty() {
        None
    } else {
        Some(hypatia::model::content::Synonyms::Flat(list))
    }
}

/// Comma-separated scopes. A trailing comma adds the empty-string global scope, so `","`
/// is global only and `"p,"` is project plus global; `""` stores no scope at all.
fn parse_scopes(raw: &str) -> Vec<String> {
    let mut scopes = parse_list(raw);
    if raw.ends_with(',') && !scopes.contains(&String::new()) {
        scopes.push(String::new());
    }
    scopes
}

const HOOK_EVENT_FIELD_SESSION_ID: &str = "session_id";
const HOOK_EVENT_FIELD_TURN_ID: &str = "turn_id";
const HOOK_EVENT_FIELD_LAST_ASSISTANT_MESSAGE: &str = "last_assistant_message";
const HOOK_EVENT_FIELD_CWD: &str = "cwd";
const HOOK_EVENT_FIELD_TRANSCRIPT_PATH: &str = "transcript_path";
const HOOK_EVENT_FIELD_PROMPT: &str = "prompt";
const HOOK_EVENT_FIELD_HOOK_EVENT_NAME: &str = "hook_event_name";
const DEFAULT_SHELF: &str = "default";
const UNSUMMARIZED_THRESHOLD: i64 = 16;
const SEARCH_RECALL_LIMIT: i64 = 5;
const TURN_COMPACT_THRESHOLD: i64 = 5;
const SEARCH_TOKEN_LIMIT: i64 = 100;

#[tool_router(router = memory_hook_router, vis = "pub")]
impl CombinedServer {
    #[tool(description = "Tool to handle the start of an agent session")]
    fn agent_session_start(
        &self,
        Parameters(event): Parameters<Map<String, Value>>,
    ) -> Result<Json<Value>, ErrorData> {
        let required_keys = vec![
            HOOK_EVENT_FIELD_SESSION_ID,
            HOOK_EVENT_FIELD_TURN_ID,
            HOOK_EVENT_FIELD_LAST_ASSISTANT_MESSAGE,
            HOOK_EVENT_FIELD_CWD,
            HOOK_EVENT_FIELD_HOOK_EVENT_NAME,
        ];
        if required_keys.iter().any(|key| !event.contains_key(*key)) {
            return Err(ErrorData::invalid_request(
                "Missing required fields".to_string(),
                None,
            ));
        }
        let event_name = event.get(HOOK_EVENT_FIELD_HOOK_EVENT_NAME).cloned();
        if event_name.is_none()
            || !event_name
                .as_ref()
                .and_then(|v| v.as_str().map(|s| s.to_lowercase()))
                .is_some_and(|s| s.contains("sessionstart"))
        {
            return Err(ErrorData::invalid_request(
                "Invalid event name for start hook".to_string(),
                None,
            ));
        }
        let session_id = event.get(HOOK_EVENT_FIELD_SESSION_ID).cloned();
        let _turn_id = event.get(HOOK_EVENT_FIELD_TURN_ID).cloned();
        let _last_assistant_message = event.get(HOOK_EVENT_FIELD_LAST_ASSISTANT_MESSAGE).cloned();
        let _transcript_path = event.get(HOOK_EVENT_FIELD_TRANSCRIPT_PATH).cloned();
        let cwd = event.get(HOOK_EVENT_FIELD_CWD).cloned();
        let project = std::path::Path::new(cwd.as_ref().and_then(|v| v.as_str()).unwrap_or(""))
            .file_name()
            .and_then(|os_str| os_str.to_str())
            .unwrap_or("")
            .to_string();
        let timestamp = OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        let mut hypatia_lab =
            hypatia::Lab::new().map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        let mut session_hook_log = OpenOptions::new()
            .create(true)
            .append(true)
            .open(hypatia_hook_hook_log())
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        let rule_query_str = format!(
            "[\"$knowledge\", [\"$contains\", \"tags\", \"rule\"], [\"$or\", [\"$contains\", \"scopes\", \"{project}\"], [\"$contains\", \"scopes\", \"\"]]]"
        );
        let rule_query: Value = from_str(&rule_query_str).map_err(|e| {
            ErrorData::internal_error(format!("invalid rule query JSON: {e}"), None)
        })?;
        let rule_query_result = hypatia_lab
            .query(DEFAULT_SHELF, &rule_query)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        let taboo_query_str = format!(
            "[\"$knowledge\", [\"$contains\", \"tags\", \"taboo\"], [\"$or\", [\"$contains\", \"scopes\", \"{project}\"], [\"$contains\", \"scopes\", \"\"]]]"
        );
        let taboo_query: Value = from_str(&taboo_query_str).map_err(|e| {
            ErrorData::internal_error(format!("invalid taboo query JSON: {e}"), None)
        })?;
        let taboo_query_result = hypatia_lab
            .query(DEFAULT_SHELF, &taboo_query)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        let rules = rule_query_result
            .rows
            .iter()
            .filter_map(|row| {
                row.get("content")
                    .and_then(|inner_map| inner_map.get("data"))?
                    .as_str()
            })
            .collect::<Vec<_>>();
        let injection_rules = if rules.is_empty() {
            "No project rules".to_string()
        } else {
            format!(
                "### Project/global rules (follow these) \n{}",
                rules.join("\n").to_string()
            )
        };
        let taboos = taboo_query_result
            .rows
            .iter()
            .filter_map(|row| {
                row.get("content")
                    .and_then(|inner_map| inner_map.get("data"))?
                    .as_str()
            })
            .collect::<Vec<_>>();
        let injection_taboos = if taboos.is_empty() {
            "No project taboos".to_string()
        } else {
            format!(
                "### Project/global taboos (avoid these) \n{}",
                taboos.join("\n").to_string()
            )
        };
        let session_id_str = session_id.as_ref().and_then(|v| v.as_str()).unwrap_or("");
        let injection_context = format!(
            "## Hypatia Memory Context\nproject: {project}\nsession: {session_id_str}\n\nEvery message in this session is automatically stored in the Hypatia knowledge graph (tag: message, scope: {project})\n{injection_rules}\n{injection_taboos}\n"
        );
        write!(session_hook_log, "{}: {} event: handled session start event with injected context for session {} from shelf {}",
            timestamp,
            event_name.clone().unwrap_or_default(),
            session_id_str,
            DEFAULT_SHELF)
        .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        session_hook_log
            .flush()
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        Ok(Json(
            json!({"continue": true, "hookSpecificOutput": {"hookEventName": event_name, "additionalContext": injection_context}}),
        ))
    }

    #[tool(description = "Tool to handle the stop of an agent session")]
    fn agent_stop(
        &self,
        Parameters(event): Parameters<Map<String, Value>>,
    ) -> Result<Json<Value>, ErrorData> {
        let required_keys = vec![
            HOOK_EVENT_FIELD_SESSION_ID,
            HOOK_EVENT_FIELD_TURN_ID,
            HOOK_EVENT_FIELD_LAST_ASSISTANT_MESSAGE,
            HOOK_EVENT_FIELD_CWD,
            HOOK_EVENT_FIELD_HOOK_EVENT_NAME,
        ];
        if required_keys.iter().any(|key| !event.contains_key(*key)) {
            return Err(ErrorData::invalid_request(
                "Missing required fields".to_string(),
                None,
            ));
        }
        let event_name = event.get(HOOK_EVENT_FIELD_HOOK_EVENT_NAME).cloned();
        if event_name.is_none()
            || !event_name
                .as_ref()
                .and_then(|v| v.as_str().map(|s| s.to_lowercase()))
                .is_some_and(|s| s.contains("stop"))
        {
            return Err(ErrorData::invalid_request(
                "Invalid event name for stop hook".to_string(),
                None,
            ));
        }
        let session_id = event.get(HOOK_EVENT_FIELD_SESSION_ID).cloned();
        let turn_id = event.get(HOOK_EVENT_FIELD_TURN_ID).cloned();
        let last_assistant_message = event.get(HOOK_EVENT_FIELD_LAST_ASSISTANT_MESSAGE).cloned();
        let cwd = event.get(HOOK_EVENT_FIELD_CWD).cloned();
        let project = std::path::Path::new(cwd.as_ref().and_then(|v| v.as_str()).unwrap_or(""))
            .file_name()
            .and_then(|os_str| os_str.to_str())
            .unwrap_or("")
            .to_string();
        let timestamp = OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        let message_name = format_message_name(
            &session_id.as_ref().and_then(|v| v.as_str()).unwrap_or(""),
            &turn_id.as_ref().and_then(|v| v.as_str()).unwrap_or(""),
            MessageRole::Assistant,
        );
        let mut hypatia_lab =
            hypatia::Lab::new().map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        let mut session_hook_log = OpenOptions::new()
            .create(true)
            .append(true)
            .open(hypatia_hook_hook_log())
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        let existing_knowledge_result = hypatia_lab.get_knowledge(DEFAULT_SHELF, &message_name);
        match existing_knowledge_result {
            Ok(None) => {
                // Handle the case where existing knowledge is found
                let content = format!(
                    "## Role\nassistant\n\n## Timestamp\n{}\n\n## Content\n{}",
                    timestamp,
                    last_assistant_message
                        .as_ref()
                        .and_then(|v| v.as_str())
                        .unwrap_or(""),
                );
                let tags = vec!["message"];
                let scopes = vec![project];
                let knowledge_content = hypatia::model::Content::new(&content)
                    .with_tags(tags.iter().map(|s| s.to_string()).collect::<Vec<String>>())
                    .with_synonyms(parse_flat_synonyms(""))
                    .with_figures(parse_list(""))
                    .with_scopes(scopes)
                    .with_embed(false.then_some(false));
                let _ = hypatia_lab
                    .create_knowledge(DEFAULT_SHELF, &message_name, knowledge_content)
                    .map_err(|e| ErrorData::internal_error(e.to_string(), None));
                writeln!(
                    session_hook_log,
                    "{}: {} event: logged assistant msg {} in shelf {}",
                    timestamp,
                    event_name.unwrap_or_default(),
                    message_name,
                    DEFAULT_SHELF
                )
                .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
            }
            Ok(Some(_)) => {
                writeln!(
                    session_hook_log,
                    "{}: {} event: msg {} exists in shelf {}, so skip creation",
                    timestamp,
                    event_name.unwrap_or_default(),
                    message_name,
                    DEFAULT_SHELF
                )
                .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
            }
            Err(e) => {
                // Handle the case where there was an error retrieving existing knowledge
                Err(ErrorData::internal_error(e.to_string(), None))?
            }
        }
        session_hook_log
            .flush()
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        Ok(Json(json!({"continue": true})))
    }

    #[tool(description = "Tool to handle the submission of user prompts")]
    fn user_prompt_submission(
        &self,
        Parameters(event): Parameters<Map<String, Value>>,
    ) -> Result<Json<Value>, ErrorData> {
        let required_keys = vec![
            HOOK_EVENT_FIELD_SESSION_ID,
            HOOK_EVENT_FIELD_TURN_ID,
            HOOK_EVENT_FIELD_LAST_ASSISTANT_MESSAGE,
            HOOK_EVENT_FIELD_CWD,
            HOOK_EVENT_FIELD_HOOK_EVENT_NAME,
            HOOK_EVENT_FIELD_PROMPT,
        ];
        if required_keys.iter().any(|key| !event.contains_key(*key)) {
            return Err(ErrorData::invalid_request(
                "Missing required fields".to_string(),
                None,
            ));
        }
        let event_name = event.get(HOOK_EVENT_FIELD_HOOK_EVENT_NAME).cloned();
        if event_name.is_none()
            || !event_name
                .as_ref()
                .and_then(|v| v.as_str().map(|s| s.to_lowercase()))
                .is_some_and(|s| s.contains("userpromptsubmit"))
        {
            return Err(ErrorData::invalid_request(
                "Invalid event name for user prompt submission hook".to_string(),
                None,
            ));
        }
        let session_id = event.get(HOOK_EVENT_FIELD_SESSION_ID).cloned();
        let turn_id = event.get(HOOK_EVENT_FIELD_TURN_ID).cloned();
        let _last_assistant_message = event.get(HOOK_EVENT_FIELD_LAST_ASSISTANT_MESSAGE).cloned();
        let cwd = event.get(HOOK_EVENT_FIELD_CWD).cloned();
        let prompt = event.get(HOOK_EVENT_FIELD_PROMPT).cloned();
        let project = std::path::Path::new(cwd.as_ref().and_then(|v| v.as_str()).unwrap_or(""))
            .file_name()
            .and_then(|os_str| os_str.to_str())
            .unwrap_or("")
            .to_string();
        let timestamp = OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        let message_name = format_message_name(
            &session_id.as_ref().and_then(|v| v.as_str()).unwrap_or(""),
            &turn_id.as_ref().and_then(|v| v.as_str()).unwrap_or(""),
            MessageRole::User,
        );
        let prompt_str = prompt
            .and_then(|p| p.as_str().map(|s| s.to_string()))
            .unwrap_or_default();
        let mut hypatia_lab =
            hypatia::Lab::new().map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        let mut session_hook_log = OpenOptions::new()
            .create(true)
            .append(true)
            .open(hypatia_hook_hook_log())
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        let existing_knowledge_result = hypatia_lab.get_knowledge(DEFAULT_SHELF, &message_name);
        match existing_knowledge_result {
            Ok(Some(_)) => {
                // Handle the existing knowledge if needed
                writeln!(
                    session_hook_log,
                    "{}: {} event: msg {} exists in shelf {}, so skip creation",
                    timestamp,
                    event_name.clone().unwrap_or_default(),
                    message_name,
                    DEFAULT_SHELF
                )
                .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
            }
            Ok(None) => {
                let content = format!(
                    "## Role\nuser\n## Timestamp\n{}\n\n## Content\n{}\n",
                    timestamp.clone(),
                    prompt_str.clone()
                );
                let tags = vec!["message"];
                let scopes = vec![project.clone()];
                let knowledge_content = hypatia::model::Content::new(&content)
                    .with_tags(tags.iter().map(|s| s.to_string()).collect::<Vec<String>>())
                    .with_synonyms(parse_flat_synonyms(""))
                    .with_figures(parse_list(""))
                    .with_scopes(scopes)
                    .with_embed(false.then_some(false));
                let _ = hypatia_lab
                    .create_knowledge(DEFAULT_SHELF, &message_name, knowledge_content)
                    .map_err(|e| ErrorData::internal_error(e.to_string(), None));
                writeln!(
                    session_hook_log,
                    "{}: {} event: logged user msg {} in shelf {}",
                    timestamp,
                    event_name.clone().unwrap_or_default(),
                    message_name,
                    DEFAULT_SHELF
                )
                .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
            }
            Err(e) => {
                return Err(ErrorData::internal_error(e.to_string(), None));
            }
        };
        if !hypatia_hook_session_dir().exists() {
            std::fs::create_dir_all(hypatia_hook_session_dir())
                .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        }
        let session_id_str = session_id.as_ref().and_then(|v| v.as_str()).unwrap_or("");
        let session_log_path = hypatia_hook_session_dir().join(format!("{session_id_str}.log"));
        let mut turns = 1;
        if session_log_path.exists() {
            let mut session_log_file = OpenOptions::new()
                .read(true)
                .write(true)
                .open(session_log_path)
                .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
            let mut buf = String::new();
            session_log_file
                .read_to_string(&mut buf)
                .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
            let mut session_state: Map<String, Value> = from_str::<Map<String, Value>>(&buf)
                .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
            let local_turns = session_state
                .get("turns")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            turns = local_turns + 1;
            session_state.insert("turns".to_string(), Value::from(turns));
            let updated_session_state = to_string(&session_state)
                .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
            session_log_file
                .set_len(0)
                .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
            session_log_file
                .write_all(updated_session_state.as_bytes())
                .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        } else {
            let mut session_log_file = OpenOptions::new()
                .write(true)
                .create(true)
                .open(session_log_path)
                .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
            let mut session_state = Map::<String, Value>::new();
            session_state.insert("turns".to_string(), Value::from(1));
            let updated_session_state = to_string(&session_state)
                .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
            session_log_file
                .write_all(updated_session_state.as_bytes())
                .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        }
        let unsummarized_jse_str = format!(
            "[\"$not-summaried\", \"message\", [\"$contains\", \"scopes\", \"{project}\"]]"
        );
        let unsummarized_query: Value = from_str(&unsummarized_jse_str).map_err(|e| {
            ErrorData::internal_error(format!("invalid rule query JSON: {e}"), None)
        })?;
        let unsummarized_query_result = hypatia_lab
            .query(DEFAULT_SHELF, &unsummarized_query)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        let mut triggers_str = "[hypatia-memory] TRIGGER:log".to_string();
        if prompt_str.contains("remember")
            || prompt_str.contains("recall")
            || prompt_str.contains("forget")
        {
            triggers_str.push_str(", TRIGGER:immediate");
        }
        if turns.ge(&TURN_COMPACT_THRESHOLD.unsigned_abs()) {
            triggers_str.push_str(", TRIGGER:extract");
        }
        if unsummarized_query_result.total_count >= Some(UNSUMMARIZED_THRESHOLD) {
            // Handle the case where the number of unsummarized messages exceeds the threshold
            triggers_str.push_str(", TRIGGER:summary");
        }
        let mut recall = "".to_string();
        let search_q = prompt_str
            .replace("\n", " ")
            .chars()
            .take(SEARCH_TOKEN_LIMIT as usize)
            .collect::<String>();
        let recall_search_result = hypatia_lab
            .search(
                DEFAULT_SHELF,
                &search_q,
                hypatia::model::SearchOpts {
                    catalog: Some("knowledge".to_string()),
                    limit: SEARCH_RECALL_LIMIT,
                    offset: 0,
                },
            )
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        recall_search_result.rows.iter().for_each(|row| {
            recall.push_str(&format!(
                "- {}: {}",
                row.get("key")
                    .and_then(|inner_map| inner_map.as_str())
                    .unwrap_or(""),
                row.get("content")
                    .and_then(|inner_map| inner_map.as_str())
                    .unwrap_or("")
            ));
        });
        let context_str=format!("## Hypatia Memory Instructions\nSignals: {triggers_str}\n
If TRIGGER:log — a message entry was already stored; no action needed.
If TRIGGER:immediate — the user explicitly asked to remember/forget: use the hypatia skill to complete the memory operation.
If TRIGGER:extract — every 5 turns: follow the hypatia-memory skill's Semantic Extraction Protocol for the last completed work unit.
If TRIGGER:summary — too many unsummarized messages: follow the summary cascade (level 2+) for scope \"{project}\".
## Reference Information (from Hypatia):\n {recall}"
        );
        Ok(Json(
            json!({"continue": true, "hookSpecificOutput": {"hookEventName": event_name, "additionalContext": context_str}}),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_names_differ_by_role() {
        let user = format_message_name("s1", "t1", MessageRole::User);
        let assistant = format_message_name("s1", "t1", MessageRole::Assistant);
        assert_eq!(user, "msg_s1_t1_u");
        assert_eq!(assistant, "msg_s1_t1_a");
        assert_ne!(user, assistant);
    }
}
