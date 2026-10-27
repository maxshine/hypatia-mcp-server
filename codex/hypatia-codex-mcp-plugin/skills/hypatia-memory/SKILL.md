---
name: hypatia-memory
description: "Automatic memory management for the Hypatia knowledge graph in Codex. Use when the Hypatia hooks inject a `[hypatia-memory]` signal (TRIGGER:log, TRIGGER:immediate, TRIGGER:extract, TRIGGER:summary) or a Hypatia Memory Context block, and when the user explicitly asks to remember, recall, or forget something. Covers following injected rules and taboos, the hierarchical summary cascade over logged messages, semantic extraction of work units, rules and taboos, and explicit remember/recall/forget operations, all through the hypatia_memory MCP tools."
---

# Hypatia Memory System

You are an automatic memory management system built on Hypatia. The plugin's lifecycle hooks
already do the deterministic work — logging every message and injecting context — and this
skill covers the AI-heavy work they leave to you:

1. **Follow injected rules and taboos** for the current project.
2. **Summarize the conversation log** through a hierarchical summary cascade.
3. **Extract semantic memories** (rules, taboos, work units) from completed work.
4. **Execute explicit memory operations** (remember, recall, forget).

All reads and writes go through the `hypatia_memory` MCP tools (`create_knowledge`,
`get_knowledge`, `update_knowledge`, `delete_knowledge`, `create_statement`,
`delete_statement`, `query`, `search`, `similar`). See the `hypatia` skill for full tool and
JSE syntax. The hooks always use the `default` shelf, so omit the `shelf` argument everywhere
in this skill.

## What the Hooks Do

The plugin's `hooks/hooks.json` calls hook tools on the `hypatia_memory` MCP server:

| Codex event | MCP hook tool | What it does |
|---|---|---|
| `SessionStart` | `agent_session_start` | Loads rules and taboos for the project and the global scope and injects a **Hypatia Memory Context** block (`project`, `session`, rules, taboos) |
| `UserPromptSubmit` | `user_prompt_submission` | Logs the user message, counts session turns, emits trigger signals, and injects **Hypatia Memory Instructions** with a **Reference Information** section recalled by full-text search |
| `Stop` | `agent_stop` | Logs the assistant's final message (side effect only; Stop hooks cannot inject context) |

The hooks never call a model. They write message entries and emit signals; the synthesis
steps below are yours.

### Trigger Signals

`user_prompt_submission` emits a line such as `Signals: [hypatia-memory] TRIGGER:log, TRIGGER:extract`:

| Signal | When the hook emits it | Your response |
|---|---|---|
| `TRIGGER:log` | Every user message | Nothing — the message is already stored. Do **not** write it again |
| `TRIGGER:immediate` | The prompt contains "remember", "recall" or "forget" | [Explicit Memory Operations](#explicit-memory-operations-triggerimmediate) |
| `TRIGGER:extract` | Every turn once the session has reached 5 turns | [Semantic Extraction Protocol](#semantic-extraction-protocol) |
| `TRIGGER:summary` | ≥ 16 `message` entries in the project scope are not yet summarized | [Hierarchical Summary Cascade](#hierarchical-summary-cascade) |

If no signal is present, no memory action is needed. Memory work is background work: answer
the user's actual request first or alongside it, and never let it block or derail the
conversation.

## Identifiers

| Placeholder | Source |
|---|---|
| `<PROJECT>` | The `project:` line of the injected Hypatia Memory Context (the basename of the session's working directory) |
| `<SESSION_ID>` | The `session:` line of the injected Hypatia Memory Context |
| `<DATE>` | Today's date, `YYYY-MM-DD` |

Use `<PROJECT>` exactly as injected. It is the scope the hooks write messages under and read
rules and taboos from, so any other spelling would make your writes invisible to them.

## Session Startup

When the session starts, the injected Hypatia Memory Context lists the project/global rules
(follow them) and taboos (avoid them). Internalize both for the whole session. Do not re-query
them unless the context block is missing; in that case load them yourself:

```json
query {"jse": ["$knowledge", ["$has", "tags", "rule"], ["$or", ["$has", "scopes", "<PROJECT>"], ["$has", "scopes", ""]]]}
query {"jse": ["$knowledge", ["$has", "tags", "taboo"], ["$or", ["$has", "scopes", "<PROJECT>"], ["$has", "scopes", ""]]]}
```

Before introducing a tag outside the vocabulary in [Important Rules](#important-rules), check
that it is not a respelling of one already in use (see "Checking scopes and tags already in use"
in the `hypatia` skill).

---

## Conversation Log Layer

### Message entries (written by the hooks)

Every turn is stored by the hooks as one knowledge entry:

- **Name**: `msg_<SESSION_ID>_<TURN_ID>_<ROLE>`, where `<ROLE>` is `u` for the user prompt
  (`UserPromptSubmit`) and `a` for the assistant's final reply (`Stop`), so each turn has up to
  two entries
- **Tags**: `["message"]`
- **Scopes**: `["<PROJECT>"]`
- **Data**: `## Role` (`user` or `assistant`), `## Timestamp`, `## Content` sections

Treat these as read-only input. Never create, rewrite, or batch message entries yourself, and
never use them as a place to store distilled knowledge.

### Session entry (optional)

If a session-level summary becomes available — the user asks you to wrap up the session, or
you have produced an end-of-session digest — record it:

```json
create_knowledge {"name": "session-<SESSION_ID>", "data": "<session summary>", "tags": ["session"], "scopes": ["<PROJECT>"], "embed": false}
```

- `create_knowledge` fails on an existing name. When newer summary text arrives, replace it
  with `update_knowledge {"name": "session-<SESSION_ID>", "data": "<session summary>"}`; its
  tags, scopes, `created_at` and links are kept.
- If no session summary is available, skip this — never fabricate one.

Link the session's message entries to it with the exact predicate `belongTo`:

```json
create_statement {"head": "msg_<SESSION_ID>_<TURN_ID>_<ROLE>", "relation": "belongTo", "tail": "session-<SESSION_ID>", "scopes": ["<PROJECT>"], "embed": false}
```

Find the session's message entries with
`query {"jse": ["$knowledge", ["$has", "tags", "message"], ["$like", "name", "msg_<SESSION_ID>_%"]]}`.

`"embed": false` matters on statements even on a shelf that sets `embedding.skip_tags`:
statements carry no tags, so `skip_tags` cannot reach them. These links are for graph
traversal, not semantic search, and a statement has no update tool, so the choice is made once.

---

## Hierarchical Summary Cascade

Run on `TRIGGER:summary`, and again whenever a level fills up.

**Constants:** `BATCH_SIZE = 16` at every level (the hook's `TRIGGER:summary` threshold).

**Predicate:** every summary link uses the relation `summary`: `<summary-name> summary <item-name>`.

| Level | Tags | Summarizes |
|---|---|---|
| 1 | `["summary", "summary 1"]` | `message` entries |
| 2 | `["summary", "summary 2"]` | `summary 1` entries |
| N | `["summary", "summary N"]` | `summary N-1` entries |

### Step 1: Find unsummarized items at level L

`$not-summaried` returns knowledge entries carrying the tag that no `summary` statement points
at, **oldest first**:

```json
query {"jse": ["$not-summaried", "<TAG>", ["$has", "scopes", "<PROJECT>"]]}
```

| Level | `<TAG>` |
|---|---|
| 1 | `message` |
| 2 | `summary 1` |
| N | `summary N-1` |

If at least 16 rows come back, take the **first 16** (the oldest) as the batch. Otherwise stop:
never summarize a partial batch.

### Step 2: Synthesize and store the summary

1. **Synthesize** a concise summary from the batch's content — not a verbatim concatenation.
   Apply the [content policy](#content-policy-for-everything-you-write) below.
2. **Name** it with a short, descriptive slug extracted from its content (e.g.
   `error-handling-refactor`, `api-design-discussion`). Check the name is free with
   `get_knowledge`; if taken, add a distinguishing suffix.
3. **Create** it:

```json
create_knowledge {"name": "<summary-name>", "data": "<synthesized summary markdown>", "tags": ["summary", "summary <L>"], "scopes": ["<PROJECT>"]}
```

4. **Link** it to each source item, one call per item in the batch:

```json
create_statement {"head": "<summary-name>", "relation": "summary", "tail": "<item-name>", "scopes": ["<PROJECT>"], "embed": false}
```

### Step 3: Repeat

Re-run Step 1 at level L (another full batch may remain), then at level L+1 — the new summary
may complete a batch there. Stop when no level has a full batch.

### Content policy for everything you write

Shape summaries and memories by what the **user asked for**, not by everything the assistant
produced:

| User intent | Keep |
|---|---|
| Data analysis / report request | **Report summary**: heading structure plus the opening and conclusion (≈500 chars each), not the full report |
| Operation task (run, fix, deploy, install, create…, or tools were invoked) | **Operation ledger**: wall time, tools used × count, final outcome (≤600 chars) |
| Discussion (default) | **Markdown context**: the substance of the exchange |

For tool calls, shell commands, MCP calls and other external invocations, keep **what was
called, how long it took, and whether it succeeded** — never raw outputs:

```
## Tool Calls
1. `bash` — ❌ 2.0s — Error: Cannot find module '/srv/app/config'
2. `mcp:fs.read` ×2 (total 100ms) — 2✅
```

- Collapse repeated identical calls into one line with a count.
- On failure keep only a one-line error: drop stack frames (`at ...`, `Traceback (most recent
  call last):`, `stack backtrace:`, `note:` lines) and keep the final exception line. Reduce
  native crash dumps to a one-line description such as `native crash: access violation (no
  readable error message; addresses and stack omitted)`.
- **Redact secrets**: `sk-…` keys, `apiKey=…`, AWS `AKIA…`, GitHub `ghp_…`, GitLab `glpat-…`,
  Slack `xox…`, PEM private-key blocks, passwords and tokens.
- **Absolutize dates**: rewrite "today", "yesterday", "tomorrow", "last week", "just now",
  "N days ago", etc. against the actual time, e.g. "yesterday" → `2026-09-07`.

---

## Reference Recall

The `user_prompt_submission` hook injects a **Reference Information** section: up to 5
knowledge entries found by full-text search over the start of the user's prompt. Use it as
context for your answer.

When it is empty or clearly off-topic and prior knowledge would materially help, recall more
yourself. Analyze the prompt — do not reuse it verbatim as a query:

1. Identify key entities, concepts and topics.
2. Run 1–3 targeted lookups, for example:
   - Distilled knowledge by meaning, without the log layer crowding it out:
     `similar {"query": "<conceptual query>", "target": "knowledge", "exclude_tags": ["message", "summary", "session"], "limit": 5}`
   - Keywords: `search {"query": "<derived keywords>", "catalog": "knowledge", "limit": 5}`
   - Graph: `query {"jse": ["$statement", ["$triple", "<entity>", "$*", "$*"]]}`
   - Recent unsummarized conversation in this project:
     `query {"jse": ["$not-summaried", "message", ["$has", "scopes", "<PROJECT>"]]}`
3. Use at most 5 relevant entries, and mention which ones you relied on when it matters to the user.

---

## Semantic Extraction Protocol

This layer extracts **insights** (rules, taboos, work units). It does not replace the
conversation log.

### Phase 1: Assess Topic Continuity

On `TRIGGER:extract`:

1. Read the current user message and the immediately preceding conversation (last ~5 exchanges).
2. Decide whether the current message starts a new topic.
   - **Topic changed** → the segment before the current message is a **completed work unit** → Phase 2.
   - **Topic continues** → the work unit is still in progress → output
     `[hypatia-memory] Work unit still in progress, nothing extracted.` and stop.
   - **Already extracted** → if this work unit was extracted earlier in the session, stop.

Codex Stop hooks cannot inject a session-end signal. When the user says they are wrapping up
the session, run a full pass instead: treat all conversation since the last extraction as
potentially containing completed work units, find every boundary, and extract each one.

### Phase 2: Delimit the Work Unit

1. Read backwards from just before the topic-changing message.
2. Find the boundary — the first message that introduced the topic.
3. The work unit spans from that message to the last message before the current one.

Skip short or insubstantial segments (greetings, "thanks", "ok").

### Phase 3: Classify the Work Unit

| Pattern | Signature | Extraction strategy |
|---|---|---|
| **One-shot correct** | Question → correct answer, no back-and-forth | Extract Q+A directly |
| **Correction chain** | Question → answer → user correction → fix → ... | Synthesize: initial Q + each correction + final answer |
| **Exploration** | Open-ended discussion without a single correct answer | Extract key findings, decisions, rationale |
| **Bug fix** | Bug report → investigation → root cause → fix | Extract symptoms, root cause, fix approach |
| **Design decision** | Trade-off discussion → decision → rationale | Extract options considered, decision, why |
| **Trivial** | Greeting, chitchat, simple factual lookup | **Skip** — not worth remembering |

### Phase 4: Synthesize the Memory

**One-shot correct:**

```
## Context
<1-line summary>
## Solution
<the answer or approach>
## Key Detail
<non-obvious detail>
```

**Correction chain:**

```
## Context
## Initial Attempt
## Why It Was Wrong
## Correct Approach
## Lesson
```

Synthesis rules:

- Capture the lesson, not the log.
- Be specific. "Use `Arc<Mutex<T>>`" is good; "use proper synchronization" is useless.
- Include non-obvious details.
- Name things well.

### Phase 5: Selective Extraction

**Include:** technical decisions, non-obvious solutions, error patterns, design patterns, user
preferences, project conventions.

**Discard:** full debug logs, temporary paths, verbose tool output, repetitive retries,
"thank you"/"ok" exchanges.

### Phase 6: Deduplicate

Before storing, check for similar knowledge:

```json
search {"query": "<keywords>", "catalog": "knowledge", "limit": 5}
similar {"query": "<one-line gist>", "target": "knowledge", "exclude_tags": ["message", "summary", "session"], "limit": 5}
```

- **Duplicate** (identical) → skip.
- **Supersedes** (new contradicts old) → store, then `create_statement {"head": "<new>", "relation": "supersedes", "tail": "<old>"}`.
- **Extends** (adds to old) → store, then `create_statement {"head": "<new>", "relation": "extends", "tail": "<old>"}`.

### Phase 7: Store

```json
create_knowledge {"name": "wu-<DATE>-<slug>", "data": "<synthesized content>", "tags": ["memory", "work-unit", "<topic-tag>"], "scopes": ["<PROJECT>"]}
create_statement {"head": "wu-<DATE>-<slug>", "relation": "is_a", "tail": "work-unit", "scopes": ["<PROJECT>"]}
```

Optionally link it to the conversation log:

```json
create_statement {"head": "wu-<DATE>-<slug>", "relation": "derivedFrom", "tail": "msg_<SESSION_ID>_<TURN_ID>_<ROLE>", "scopes": ["<PROJECT>"], "embed": false}
```

If the work unit establishes a lasting convention or prohibition, also store it as a `rule` or
`taboo` (see below) so future sessions get it injected at startup.

---

## Explicit Memory Operations (TRIGGER:immediate)

The hook emits `TRIGGER:immediate` on a plain substring match, so first confirm the user is
actually asking you to remember, recall, or forget something. If not, ignore the signal.

### Remember

1. Identify exactly what to remember.
2. Classify it as `rule` (something to follow), `taboo` (something to avoid), or general `memory`.
3. Choose scopes: `["<PROJECT>"]` for this project only, or `["<PROJECT>", ""]` to also make
   it global. The global scope is the `""` element; a rule without it is never injected into
   other projects.
4. Deduplicate as in Phase 6. If an existing entry says the same thing, update it instead of
   creating a near-duplicate.
5. Create it:

   ```json
   create_knowledge {"name": "<type>:<slug>", "data": "<content>", "tags": ["memory", "<type>"], "scopes": ["<SCOPES>"]}
   ```

6. Create its `is_a` statement and any relationship statements (see the `hypatia` skill's
   auto-linking rules), using the same scopes.

Rules and taboos are injected at the start of every later session, so keep their `data` to one
or two crisp sentences.

### Recall

Follow [Reference Recall](#reference-recall) with the user's topic, then answer from what you
found and say plainly when nothing relevant is stored.

### Forget

1. Find candidates: `search {"query": "<topic>", "limit": 10}` (plus `get_knowledge` for an exact name).
2. If more than one entry could match, confirm with the user which ones to delete.
3. For each entry, find its statements with
   `query {"jse": ["$statement", ["$triple", "<name>", "$*", "$*"]]}` and
   `query {"jse": ["$statement", ["$triple", "$*", "$*", "<name>"]]}`, delete them with
   `delete_statement`, then `delete_knowledge` the entry. For full erasure, include the related
   `message` and `summary` entries.
4. Confirm what was removed.

---

## Output Format

Keep memory reports to a single short line after your main answer:

```
[hypatia-memory] Cascade: +1 summary 1 (16 messages), +0 summary 2.
[hypatia-memory] Extracted 2 work units (1 one-shot, 1 correction-chain), skipped 1 trivial.
  wu-2026-05-10-sort-function → memory,work-unit,rust
[hypatia-memory] Stored: "rule:prefer-immutable-patterns" (rule, scoped to my-project).
[hypatia-memory] Removed 1 entry and 2 relationships.
[hypatia-memory] Work unit still in progress, nothing extracted.
```

---

## Important Rules

1. **Never store sensitive information** — no passwords, API keys, tokens.
2. **The log is complete; extraction is selective** — the hooks log every message; extract work units only when substantive.
3. **Never duplicate the hooks' writes** — do not create `message` entries.
4. **Be conservative with work-unit quality** — skip when unsure.
5. **Synthesize, don't transcribe** — compress summaries and memories.
6. **Correction chains are gold** — the most valuable memories come from mistakes.
7. **Use structured tags** — `message`, `session`, `summary`, `summary <N>`, `memory`, `work-unit`, `rule`, `taboo`.
8. **Don't interrupt the user** — memory operations are background tasks.
9. **Scope discipline** — every entry and statement you write carries `"scopes": ["<PROJECT>"]`; global rules add `""`. Omitting `scopes` stores no scope, and the entry drops out of every scoped lookup.

## Graph Schema Reference

```
session-<SESSION_ID>  (tags: session)
    ↑ belongTo
msg_<SESSION_ID>_<TURN_ID>_u|a  (tags: message; written by the hooks)

<summary-name>  (tags: summary, summary 1)
    ↓ summary (×16)
msg_...

<summary-name>  (tags: summary, summary 2)
    ↓ summary (×16)
<summary-name>...  (tags: summary, summary 1)

wu-<DATE>-<slug>  (tags: memory, work-unit)  ── is_a → work-unit
                                             ── derivedFrom → msg_* (optional)

<type>:<slug>  (tags: memory, rule | taboo)  ← injected at session start
```
