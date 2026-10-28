---
name: hypatia
description: "Interact with the Hypatia AI memory system using natural language. Translate user requests into calls to the hypatia_memory MCP tools for knowledge CRUD, statement (RDF triple) management, JSE queries, full-text search, vector similarity search, tag and scope vocabulary, and shelf management. Use when the user mentions memories, knowledge bases, knowledge graphs, triples, statements, relationships, or shelves, or asks to store, recall, remember, record, save, find, or search information in Hypatia. Also use when the user wants to create or explore relationships between concepts, query existing knowledge, or manage shelves. Examples: 'remember that Rust is a systems language', 'find everything about Alice', 'record that Alice knows Bob', 'search for programming', 'show all knowledge', 'list shelves', 'what tags exist?'."
---

# Hypatia Query Skill

You are operating Hypatia — an AI-oriented memory management system — through the
`hypatia_memory` MCP server bundled with this plugin. Translate the user's natural-language
request into the appropriate MCP tool call(s) and execute them.

Do not shell out to a `hypatia` binary or open the shelf's SQLite files directly. Every
operation in this skill goes through the `hypatia_memory` MCP tools.

## Server Availability

The tools are served by a locally running `hypatia-mcp-server` (see the plugin's `.mcp.json`).
In Claude Code they appear as `mcp__plugin_hypatia-claude-mcp-plugin_hypatia_memory__<tool>`;
this skill uses the bare tool names. If the tools are missing or every call fails to connect,
stop and tell the user to start the server with `hypatia-mcp-server serve` (check its status
with `/mcp`). Do not retry the same failing call
in a loop.

## Tool Reference

| Tool | Purpose | Key arguments |
|---|---|---|
| `list_shelves` | List connected shelves | — |
| `connect_shelf` | Connect the shelf stored at `~/.hypatia/<shelf_name>` | `shelf_name` |
| `disconnect_shelf` | Remove a shelf from the registry | `shelf_name` |
| `get_shelf_status` | Backend, embedder, attention notes, embedding debt | `shelf_name` |
| `list_models` | Local embedding models available | — |
| `list_archives` | List `archive://` files of a shelf | `shelf_name` |
| `list_shelf_scopes` | Every scope in use, with how many entries carry it | `shelf_name` |
| `check_shelf_scope_existence` | Whether one exact scope is in use | `name`, `shelf` |
| `list_shelf_tags` | Every tag in use, with how many entries carry it | `shelf_name` |
| `check_shelf_tag_existence` | Whether one exact tag is in use | `name`, `shelf` |
| `create_knowledge` | Create a knowledge entry | `name`, `data`, `tags`, `synonyms`, `figures`, `scopes`, `embed`, `shelf` |
| `get_knowledge` | Read a knowledge entry by exact name | `name`, `shelf` |
| `update_knowledge` | Patch a knowledge entry | `name` + any of `data`, `tags`, `synonyms`, `figures`, `scopes`, `embed`, `shelf` |
| `delete_knowledge` | Delete a knowledge entry | `name`, `shelf` |
| `create_statement` | Create a `(head, relation, tail)` triple | `head`, `relation`, `tail`, `data`, `synonyms`, `scopes`, `embed`, `shelf` |
| `delete_statement` | Delete a triple | `head`, `relation`, `tail`, `shelf` |
| `query` | Run a JSE query | `jse`, `shelf` |
| `search` | Full-text search | `query`, `catalog`, `limit`, `offset`, `shelf` |
| `similar` | Vector similarity search | `query`, `target`, `limit`, `tags`, `exclude_tags`, `where`, `shelf` |
| `backfill_shelf` | Embed one batch of entries still missing vectors | `limit` (1–512, default 64), `shelf` |

General conventions:

- `shelf` (and `shelf_name` on the shelf-level tools) is optional and defaults to
  `"default"`. The plugin's hooks always read and write the `default` shelf, so omit it unless
  the user names another one.
- `tags`, `scopes`, `figures` and knowledge `synonyms` are **JSON arrays of strings**, not
  comma-separated strings.
- Results come back as JSON. `query`, `search` and `similar` return
  `{"rows": [...], "total_count": N}`, where `total_count` is the number of rows returned
  (not the number of matches before the limit). An empty `rows` array means no results.
- Errors for an expected condition come back as `{"error": "..."}` in the result (for example
  "already exists" or "not found"); unexpected failures surface as a tool error.

## Shelf Management

| User says | Tool call |
|---|---|
| "list shelves" / "show connected shelves" | `list_shelves` |
| "connect shelf NAME" / "open shelf NAME" | `connect_shelf {"shelf_name": "<name>"}` |
| "disconnect shelf NAME" / "close shelf NAME" | `disconnect_shelf {"shelf_name": "<name>"}` |
| "status of shelf NAME" / "is embedding caught up?" | `get_shelf_status {"shelf_name": "<name>"}` |
| "which embedding models are available?" | `list_models` |

`connect_shelf` only connects a shelf that lives at `~/.hypatia/<shelf_name>`; it cannot
open an arbitrary path. Exporting a shelf is not available through the MCP server.

## Archive Files

Archive files (images, PDFs, data, etc.) live in the shelf's `archives/` directory and are
referenced via `archive://` paths.

| User says | Tool call |
|---|---|
| "list archives" / "show all files" | `list_archives {}` |

Storing a new archive file or copying one out is not available through the MCP server. If
the user asks for that, say so instead of improvising.

Archive entries carry the `archive` tag and file metadata such as `mime_type`, so they can be
queried:

```json
["$knowledge", ["$has", "tags", "archive"]]
["$knowledge", ["$content", {"mime_type": "image/png"}]]
```

To reference an existing archive when creating knowledge, pass it in `figures`:

```json
{"name": "Euclid Prop 1", "data": "equilateral triangle", "figures": ["archive://euclid/fig1.png"]}
```

## Scopes

Use `scopes` to assign project or global scope to knowledge and statements. **The global
scope is the empty string `""` as an element of the list.**

| Intent | `scopes` value |
|---|---|
| Project only | `["my-project"]` |
| Global only | `[""]` |
| Project and global | `["my-project", ""]` |
| No scope at all | omit `scopes` (or `[]`) — global lookups such as `["$has", "scopes", ""]` will never find the entry |

```json
{"name": "API convention", "data": "REST endpoints use kebab-case", "tags": ["rule"], "scopes": ["my-project"]}
{"name": "prefer immutable", "data": "always create new objects", "tags": ["rule"], "scopes": [""]}
{"name": "no mock DB", "data": "never mock database in tests", "tags": ["taboo"], "scopes": ["my-project", ""]}
```

### Checking scopes and tags already in use

A scope or tag you invent is not an error — the entry is stored, and every later lookup by
the value you meant misses it. Check before you write:

```json
list_shelf_scopes {}
list_shelf_tags {}
check_shelf_scope_existence {"name": "my-project"}
check_shelf_tag_existence {"name": "rule"}
```

- `list_shelf_scopes` returns `{"shelf_name": "...", "scopes": {"<value>": <entries>, ...}}`
  and `list_shelf_tags` returns the same shape under `"tags"`. The counts cover knowledge
  **and** statements. An empty map can also mean the shelf could not be read (for example it
  is not connected), so check `list_shelves` if a populated shelf comes back empty.
- The global scope appears as the key `""`. Use exactly that value when writing it back; never
  write a display label such as `(global)`. An entry that declares no scopes at all is not
  counted as global.
- `check_shelf_scope_existence` / `check_shelf_tag_existence` return `"exists": true|false` for
  one exact value; `{"name": ""}` checks the global scope. Matching is exact, so `rule` and
  `rules` are different tags.

If a value you were about to write is not in use, list the vocabulary and reuse the spelling
already there: if the shelf holds `my-app` and you were about to write `my_app`, write `my-app`.
Introduce a genuinely new value only when nothing existing fits.

| User says | Tool call |
|---|---|
| "what scopes/projects are in here?" | `list_shelf_scopes {}` |
| "what tags exist?" / "which labels are used?" | `list_shelf_tags {}` |
| "is there already a `rule` tag?" | `check_shelf_tag_existence {"name": "rule"}` |
| "is my-project used as a scope?" | `check_shelf_scope_existence {"name": "my-project"}` |

## Knowledge CRUD

Knowledge entries are independent information points with a name, content, and tags.

### Create

```json
create_knowledge {"name": "<name>", "data": "<data>", "tags": ["<tag1>", "<tag2>"], "figures": ["archive://path/to/file"], "embed": false}
```

`create_knowledge` refuses to overwrite: if the name exists it returns an error telling you to
use `update_knowledge`.

`"embed": false` keeps the entry out of the vector index: it is still stored and still found
by `search` and `query`, but it is never embedded, so it costs no model call on the way in and
is never a `similar` result. Use it for raw session logs and other bulk that carries no
distilled knowledge. A shelf can skip whole tags instead, with `embedding.skip_tags` in
`shelf.toml`. Omit `embed` for normal entries.

| User says | Tool call |
|---|---|
| "remember Rust as a systems programming language" | `create_knowledge {"name": "Rust", "data": "systems programming language", "tags": ["language", "compiled"]}` |
| "save knowledge about Go with tags language and compiled" | `create_knowledge {"name": "Go", "data": "", "tags": ["language", "compiled"]}` |
| "store that Python is a scripting language, tag it as dynamic" | `create_knowledge {"name": "Python", "data": "scripting language", "tags": ["dynamic"]}` |
| "log this turn but don't make it searchable by meaning" | `create_knowledge {"name": "msg-42", "data": "<turn>", "tags": ["message"], "embed": false}` |

### Read

| User says | Tool call |
|---|---|
| "show me knowledge about Rust" / "get Rust entry" | `get_knowledge {"name": "Rust"}` |

### Update

```json
update_knowledge {"name": "<name>", "data": "<data>", "tags": [], "synonyms": [], "figures": [], "scopes": [], "embed": true}
```

Only the fields you pass change. An omitted field keeps its stored value; `"data": ""` clears
the data and `"tags": []` clears the tags. You must pass at least one field besides `name`.
`scopes` replaces the stored scopes: include `""` in the list to keep the entry in global
lookups. The entry keeps its `created_at`. Its old vector is discarded, and a new one is
generated on the next flush. Updating an entry that does not exist is an error. The result's
`changed` field is `false` when the update changed nothing.

`"embed": false` takes the entry out of the vector index and discards its vector;
`"embed": true` puts it back, and it is embedded on the next flush. `"embed": true` does not
override the shelf's `embedding.skip_tags`: an entry can ask for less indexing than its shelf,
never more.

| User says | Tool call |
|---|---|
| "update Rust's description to mention memory safety" | `update_knowledge {"name": "Rust", "data": "systems programming language with memory safety"}` |
| "retag Go as language and google" | `update_knowledge {"name": "Go", "tags": ["language", "google"]}` |
| "remove all tags from Python" | `update_knowledge {"name": "Python", "tags": []}` |

### Delete

| User says | Tool call |
|---|---|
| "delete knowledge Rust" / "remove the Rust entry" | `delete_knowledge {"name": "Rust"}` |

## Statement Creation — Proactive Graph Building

Statements are RDF-style triples: `(head, relation, tail)`. They form the edges of the
knowledge graph.

### Key Principle: Always Enrich with Relationships

When the user asks to store or remember information, **always create statements alongside
knowledge entries** to build graph connectivity. The goal is a rich, traversable knowledge
graph, not isolated data points.

**Pattern**: after creating a knowledge entry, identify entities mentioned in the content and
create relationships between them. At minimum, create one `is_a` statement for every new
knowledge entry.

```json
create_statement {"head": "<head>", "relation": "<relation>", "tail": "<tail>", "data": "<data>", "scopes": ["<project>"], "embed": false}
```

`create_statement` is idempotent: if the triple already exists it leaves it untouched and
returns `"created": false`. Statement `synonyms` are positional:
`{"head": [...], "relation": [...], "tail": [...]}`.

`"embed": false` works as on `create_knowledge`: the triple is still stored and traversable but
gets no vector. Two differences: statements carry no tags, so a shelf's `embedding.skip_tags`
cannot reach them and bulk links must pass `"embed": false` themselves; and there is no
statement update tool, so the choice is made once at creation.

### Triple Extraction Patterns

| User says | head | relation | tail |
|---|---|---|---|
| "record that Alice knows Bob" | Alice | knows | Bob |
| "X is a Y" / "X is an Y" | X | is_a | Y |
| "X belongs to Y" / "X is part of Y" | X | belongs_to | Y |
| "X works for Y" | X | works_for | Y |
| "X is related to Y" | X | related_to | Y |
| "X depends on Y" | X | depends_on | Y |
| "X uses Y" | X | uses | Y |
| "X contains Y" | X | contains | Y |
| "X created Y" | Y | created_by | X (reversed) |

Normalize relations to `snake_case`. Common relations: `is_a`, `knows`, `related_to`,
`works_for`, `belongs_to`, `depends_on`, `uses`, `contains`, `created_by`.

### Proactive Creation Examples

When the user says **"remember that Rust is a systems programming language"**, call both:

```json
create_knowledge {"name": "Rust", "data": "systems programming language", "tags": ["language", "compiled"]}
create_statement {"head": "Rust", "relation": "is_a", "tail": "systems programming language"}
```

When the user says **"remember that PostgreSQL is a relational database used by the API"**:

```json
create_knowledge {"name": "PostgreSQL", "data": "relational database", "tags": ["database", "relational"]}
create_statement {"head": "PostgreSQL", "relation": "is_a", "tail": "relational database"}
create_statement {"head": "API", "relation": "depends_on", "tail": "PostgreSQL", "data": "primary data store"}
```

When the user says **"note that Alice is a senior engineer on the Backend team"**:

```json
create_knowledge {"name": "Alice", "data": "senior engineer on Backend team", "tags": ["person", "engineer"]}
create_statement {"head": "Alice", "relation": "is_a", "tail": "senior engineer"}
create_statement {"head": "Alice", "relation": "works_for", "tail": "Backend team"}
```

### Auto-linking Rules

When creating knowledge entries, automatically extract and create statements for:

1. **Category**: `X is_a <category>` — every entity has a type
2. **Dependencies**: if the content mentions tools, frameworks, or systems X depends on → `X depends_on Y`
3. **Relationships**: if the content mentions other entities → link them with appropriate relations
4. **Containment**: if X is part of Y → `X belongs_to Y`

## Thinking Aloud Protocol

**Every tool call MUST be preceded by a brief explanation.** Before calling any
`hypatia_memory` tool, output a block in this format:

```
**Step N: <goal>**

> `<tool> <arguments>`

Why this call: <1-2 sentences on why this tool/operator/strategy was chosen>
Expected result: <what kind of results to expect>
```

Then make the call. After results come back, briefly interpret them before deciding the next step.

### Example Walkthrough

User asks: *"Find all knowledge related to Alice, and her relationship network."*

```
**Step 1: Look up Alice's knowledge entry directly**

> `get_knowledge {"name": "Alice"}`

Why this call: the user named a specific entity, so a direct lookup returns the full entry fastest.
Expected result: Alice's entry (content, tags, figures, ...) or a "not found" error.
```

(call... entry or "not found")

```
**Step 2: Find every triple where Alice is the head**

> `query {"jse": ["$statement", ["$triple", "Alice", "$*", "$*"]]}`

Why this call: $triple matches the head position exactly with wildcard relation and tail. It is
indexed, so it is faster and more precise than $search.
Expected result: all statements with Alice as head (knows, is_a, works_for, ...).
```

(call... results)

```
**Step 3: Find relationships pointing at Alice (Alice as tail)**

> `query {"jse": ["$statement", ["$triple", "$*", "$*", "Alice"]]}`

Why this call: step 2 only covered Alice as subject; others may point at her (e.g. "Bob knows
Alice"), so the tail position completes the network.
Expected result: all statements with Alice as tail.
```

(call... results)

```
**Step 4: 2-hop traversal to explore Alice's wider network**

> `query {"jse": ["$statement", ["$k-hop", "Alice", "$*", 2]]}`

Why this call: the user asked for a relationship *network*; a 2-hop traversal reveals indirect
links such as Alice → Bob → Charlie. $k-hop uses a recursive CTE suited to graph exploration.
Expected result: all statements within Alice's 2-hop neighborhood.
```

### Rules for Thinking Aloud

1. **Always explain before calling** — never run a query silently
2. **Explain the "why"** — which operator was chosen and why, not just what it does
3. **Keep it concise** — 1-2 sentences of explanation, 1 sentence of expected results
4. **Interpret results** — after each step, briefly note what was found before deciding next steps
5. **Multi-step reasoning** — for complex requests, show the decomposition into steps and the strategy
6. **Empty results are informative** — if a query returns nothing, explain what that tells you and adjust

## Search Strategy — Graph-First Retrieval

When searching, prefer precise graph operators over broad full-text search. Use this decision
tree, with a thinking-aloud explanation before each step:

```
1. User mentions a specific entity name?
   → get_knowledge (direct lookup)

2. User asks about relationships involving a known entity?
   → query with $triple (fastest, indexed)

3. User asks about a type of relationship (e.g. "who works for X")?
   → query with $triple and wildcards on entity positions

4. User wants to explore a graph neighborhood (e.g. "who can X reach in 2 hops")?
   → query with $k-hop (graph traversal)

5. User wants semantic/similarity search (e.g. "find similar concepts")?
   → similar (vector search)

6. User wants to match a pattern (e.g. "names starting with X")?
   → query with $like

7. User wants to filter by content attributes (e.g. "markdown entries")?
   → query with $content

8. User asks about a specific entity's relationships?
   → $triple + get_knowledge combined

9. Broad/ambiguous query?
   → search (full-text fallback)
```

### Prefer $triple over $search for entity queries

| Instead of | Use |
|---|---|
| `["$statement", ["$search", "Alice"]]` | `["$statement", ["$triple", "Alice", "$*", "$*"]]` |
| `["$statement", ["$search", "manages"]]` | `["$statement", ["$triple", "$*", "manages", "$*"]]` |
| `["$statement", ["$contains", "triple", "Alice"]]` | `["$statement", ["$triple", "Alice", "$*", "$*"]]` |

### Combined Queries

| Goal | JSE |
|---|---|
| All relationships involving Alice, created after a date | `["$statement", ["$and", ["$triple", "Alice", "$*", "$*"], ["$gt", "created_at", "2025-01-01"]]]` |
| Markdown-format knowledge entries about a topic | `["$knowledge", ["$and", ["$content", {"format": "markdown"}], ["$search", "database"]]]` |
| All engineers | `["$statement", ["$triple", "$*", "is_a", "engineer"]]` |

## Full-text Search

Search is the fallback for broad or ambiguous queries. It covers knowledge and statements.

```json
search {"query": "<query>", "catalog": "knowledge", "limit": 10, "offset": 0}
```

- `"catalog": "knowledge"` — knowledge entries only
- `"catalog": "statement"` — statements only
- omit `catalog` — everything
- `limit` defaults to 100

| User says | Tool call |
|---|---|
| "search for programming" / "find everything about programming" | `search {"query": "programming"}` |
| "search knowledge for rust" | `search {"query": "rust", "catalog": "knowledge"}` |
| "what do you know about databases?" | `search {"query": "databases"}` |

## Vector Similarity Search

Semantic search using embedding vectors. Finds entries with similar meaning, even when keywords
don't match.

```json
similar {"query": "<query>", "limit": 10, "target": "both", "tags": [], "exclude_tags": [], "where": ["$contains", "scopes", "project-a"]}
```

`target` is `knowledge`, `statement`, or `both` (the default). `tags` keeps entries carrying at
least one of the tags, `exclude_tags` drops entries carrying any of them, and `where` takes a
JSE condition such as `["$contains", "scopes", "project-a"]` (`$eq`, `$like`, `$contains`,
`$has`, `$and`/`$or`/`$not` and the other filters `$knowledge` takes, but not `$search`,
`$similar` or `$k-hop`). They narrow the entries before ranking, so `limit` still returns that
many entries whenever that many qualify. `"exclude_tags": ["message", "summary", "session"]`
keeps the session-log layer from crowding out the knowledge distilled from it. With the default
target `both` the filter applies to statements too, so a condition on `name` needs
`"target": "knowledge"`.

Requires an embedding model configured in the shelf's `shelf.toml`. Entries are embedded in
batches after they are written or updated, so `similar` can miss entries changed in the last
minute; `search` finds them at once. The result's `embedding` field reports pending embedding
debt.

Entries written with `"embed": false`, and entries carrying a tag the shelf lists in
`embedding.skip_tags`, are never embedded and so are never `similar` results; `search` and
`query` still find them. After changing `skip_tags`, call `backfill_shelf` repeatedly (each call
processes one batch) until the reported `embedding` debt is zero.

| User says | Tool call |
|---|---|
| "find similar to distributed systems" | `similar {"query": "distributed systems"}` |
| "semantic search for memory management" | `similar {"query": "memory management", "limit": 5}` |
| "what do we know about auth, not the chat logs" | `similar {"query": "auth", "exclude_tags": ["message", "summary", "session"]}` |
| "rules like this one for project-a" | `similar {"query": "<rule text>", "target": "knowledge", "tags": ["rule"], "where": ["$contains", "scopes", "project-a"]}` |

## JSE Query Translation

JSE (JSON Search Expression) enables precise queries against the knowledge or statement tables.

```json
query {"jse": ["$knowledge"], "shelf": "default"}
```

Pass `jse` as a JSON value. No shell quoting or escaping is involved.

### Query Structure

The top-level operator is always `$knowledge` or `$statement` (or `$not-summaried`, below):

```json
["$knowledge", "<condition1>", "<condition2>"]
["$statement", "<condition1>", "<condition2>"]
```

No conditions means "return all" (subject to the default limit of 100):

```json
["$knowledge"]
```

Results are ordered by `created_at` descending.

### Operator Reference

| Operator | Purpose | Syntax |
|---|---|---|
| `$eq` | Equals | `["$eq", "field", "value"]` |
| `$ne` | Not equals | `["$ne", "field", "value"]` |
| `$gt` | Greater than | `["$gt", "field", "value"]` |
| `$lt` | Less than | `["$lt", "field", "value"]` |
| `$gte` | Greater than or equal | `["$gte", "field", "value"]` |
| `$lte` | Less than or equal | `["$lte", "field", "value"]` |
| `$like` | SQL LIKE pattern match | `["$like", "field", "pattern"]` |
| `$contains` | Substring match in content JSON | `["$contains", "field", "value"]` |
| `$content` | Match content JSON key-value pairs | `["$content", {"key": "value"}]` |
| `$search` | Full-text search (FTS) | `["$search", "query text"]` |
| `$similar` | Vector similarity inside a query | `["$similar", "conceptual query"]` |
| `$and` | Logical AND | `["$and", cond1, cond2, ...]` |
| `$or` | Logical OR | `["$or", cond1, cond2, ...]` |
| `$not` | Logical NOT | `["$not", condition]` |
| `$triple` | Triple position match | `["$triple", "Alice", "$*", "Bob"]` |
| `$k-hop` | K-hop graph traversal (statement only) | `["$k-hop", "head", "relation", depth]` |
| `$has` | Exact membership in a JSON field (arrays: element; scalars: equality; array value = any-of) — indexed | `["$has", "tags", "rust"]` |
| `$json-contains` | Structural containment (PG `@>`) — indexed recall + exact recheck | `["$json-contains", {"tags": ["rust"]}]` |
| `$not-summaried` | Top-level: knowledge carrying a tag that no `summary` statement points at, oldest first | `["$not-summaried", "message", condition...]` |

### Critical Syntax Rules

1. **`$and` and `$or` take multiple operands directly**, NOT a nested array:
   - CORRECT: `["$and", ["$eq", "name", "rust"], ["$contains", "tags", "systems"]]`
   - WRONG: `["$and", [["$eq", "name", "rust"], ["$contains", "tags", "systems"]]]`
2. **`$search` must be inside `$knowledge` or `$statement`**, never top-level. Inside
   `$knowledge` it searches the knowledge catalog; inside `$statement`, the statement catalog.
3. **Field names** like `"name"` and `"triple"` are plain strings. For content JSON sub-fields
   (e.g. tags), `$contains` extracts the JSON value automatically.
4. **Knowledge fields**: `name`, `created_at`, plus any content JSON field via `$has`/`$contains`.
5. **Statement fields**: `triple` (CSV-formatted `head,relation,tail`), `head`, `relation`,
   `tail`, `created_at`, `tr_start`, `tr_end`, plus content JSON fields via `$has`/`$contains`.
   For position-based triple matching, prefer `$triple` over `$contains`.
6. **`$has` is the exact-membership operator** (indexed; use it for tags/scopes lookups).
   `$contains` on array fields (`tags`/`scopes`/`figures`) routes to exact membership too; on
   other fields it stays substring. `$json-contains` matches whole-document containment, e.g.
   `["$json-contains", {"tags": ["rust"]}]`.
7. **`$not-summaried`** is spelled exactly that way.

### `$triple` Operator

Position-based matching on statement triples. Each argument corresponds to head, relation, or
tail; `"$*"` is a wildcard.

```json
["$triple", "<head_pattern>", "<relation_pattern>", "<tail_pattern>"]
```

- All 3 specified (no wildcards): primary-key lookup on `triple` (fastest)
- Partial match: conditions on individual columns
- At least one non-wildcard required — all wildcards is an error
- Exactly 3 arguments

| User says | JSE |
|---|---|
| "find all relationships where Alice is the head" | `["$statement", ["$triple", "Alice", "$*", "$*"]]` |
| "find all knows relationships" | `["$statement", ["$triple", "$*", "knows", "$*"]]` |
| "find everything related to Bob as tail" | `["$statement", ["$triple", "$*", "$*", "Bob"]]` |
| "find the exact Alice knows Bob triple" | `["$statement", ["$triple", "Alice", "knows", "Bob"]]` |
| "Alice's relationships with Bob, after a date" | `["$statement", ["$and", ["$triple", "Alice", "$*", "Bob"], ["$gt", "created_at", "2025-01-01"]]]` |

### `$like` Operator

SQL LIKE pattern matching (`%` = any chars, `_` = single char).

| User says | JSE |
|---|---|
| "find entries whose name starts with Rust" | `["$knowledge", ["$like", "name", "Rust%"]]` |
| "find entries created in January 2025" | `["$knowledge", ["$like", "created_at", "2025-01-%"]]` |
| "find statements with head matching pattern" | `["$statement", ["$like", "head", "Alice%"]]` |

### `$content` Operator

Matches key-value pairs inside the `content` JSON column (exact string equality per key).

| User says | JSE |
|---|---|
| "find all markdown-format entries" | `["$knowledge", ["$content", {"format": "markdown"}]]` |
| "find json-format statements" | `["$statement", ["$content", {"format": "json"}]]` |
| "find entries with specific data and format" | `["$knowledge", ["$content", {"format": "markdown", "data": "hello"}]]` |

### `$k-hop` Operator

K-hop graph traversal over statement triples: `["$k-hop", "head", "relation", depth]`.

- `"head"` — starting entity (required)
- `"relation"` — filter by relation, or `"$*"` for any
- `depth` — number of hops (positive integer)

| User says | JSE |
|---|---|
| "who can Alice reach in 2 hops?" | `["$statement", ["$k-hop", "Alice", "$*", 2]]` |
| "what's reachable from Alice via knows in 3 hops?" | `["$statement", ["$k-hop", "Alice", "knows", 3]]` |
| "explore the graph around PostgreSQL" | `["$statement", ["$k-hop", "PostgreSQL", "$*", 2]]` |

### Natural Language to JSE Examples

| User says | JSE |
|---|---|
| "find knowledge named rust" | `["$knowledge", ["$eq", "name", "rust"]]` |
| "find all relationships involving Alice" | `["$statement", ["$triple", "Alice", "$*", "$*"]]` |
| "who does Alice know?" | `["$statement", ["$triple", "Alice", "knows", "$*"]]` |
| "what relates to Bob?" | `["$statement", ["$triple", "$*", "$*", "Bob"]]` |
| "knowledge named rust tagged systems" | `["$knowledge", ["$and", ["$eq", "name", "rust"], ["$has", "tags", "systems"]]]` |
| "knowledge NOT named rust" | `["$knowledge", ["$not", ["$eq", "name", "rust"]]]` |
| "all is_a relationships" | `["$statement", ["$triple", "$*", "is_a", "$*"]]` |
| "entries whose name starts with 'Al'" | `["$knowledge", ["$like", "name", "Al%"]]` |
| "show all knowledge" | `["$knowledge"]` |
| "knowledge created after 2025-01-01" | `["$knowledge", ["$gt", "created_at", "2025-01-01"]]` |
| "knowledge containing 'language' in data" | `["$knowledge", ["$contains", "data", "language"]]` |

### Options (limit, offset, catalog)

Use the object form of the top-level operator to pass options:

```json
query {"jse": {"$knowledge": [["$search", "rust"]], "limit": 10, "offset": 0}}
```

The default limit is 100. When you need to know whether a result is complete, compare
`total_count` with the limit: if they are equal, the result may have been truncated.

## Disambiguation Rules

When the request is ambiguous, follow this priority:

1. **Exact name mentioned** ("get Rust", "show Alice") → `get_knowledge`
2. **Explicit type** ("find knowledge about X" / "find statements about X") → `query` with the corresponding top-level operator
3. **Relationship language** ("who does Alice know?", "relationships of X") → `query` with `$statement` + `$triple`
4. **Graph exploration** ("who can Alice reach?", "what's connected to X") → `query` with `$k-hop`
5. **Semantic/similarity** ("find similar to X", "things like X") → `similar`
6. **Pattern matching** ("names starting with X", "entries from January") → `query` with `$like`
7. **Content filtering** ("markdown entries", "entries with format X") → `query` with `$content`
8. **Broad/ambiguous** ("find everything about X", "what do you know about X") → `search`
9. **Create/store/remember** → `create_knowledge` + `create_statement` (always create relationships alongside knowledge entries)
10. **Vocabulary** ("what tags/scopes exist?", "is X already a tag?") → `list_shelf_tags` / `list_shelf_scopes` / `check_shelf_tag_existence` / `check_shelf_scope_existence`

## Output Format

- **Knowledge entries**: `{"name": "...", "content": {"format": "...", "data": "...", "tags": [...]}, "created_at": "..."}`
- **Statements**: `{"triple": "head,relation,tail", "head": "...", "relation": "...", "tail": "...", "content": {...}, "created_at": "...", "tr_start": "...", "tr_end": "..."}` — NULL columns such as `tr_start`/`tr_end` are omitted
- **Search results**: `{"id": N, "catalog": "...", "key": "...", "content": "...", "rank": N.N}`
- **Empty results**: `{"rows": [], "total_count": 0}`

Present results to the user in readable form rather than dumping raw JSON, unless they ask for it.

## Important Notes

- There is no statement-get tool. Use `query` with `$statement` conditions (exact form:
  `["$statement", ["$triple", "<head>", "<relation>", "<tail>"]]`) to read statements.
- There is no statement-update tool; to change a statement, create the new one and then delete the old one.
- `delete_statement` and `delete_knowledge` fail when the target does not exist.
- `data` is stored as a string in the content JSON's `data` field.
- **Always create statements alongside knowledge entries** to maintain graph connectivity.
  Every knowledge entry should have at least one corresponding `is_a` statement.
- **Normalize relative temporal expressions to absolute dates/times before storing.** When
  content contains "tomorrow", "next Monday", "in three hours", "tonight", etc., rewrite them
  using the current date/time as the reference point. Example: "remind me tomorrow to back up
  the data" → store "remind me on 2026-08-29 to back up the data". A relative date is
  meaningless when the memory is recalled later.
- Before writing a tag or scope you have not seen on this shelf, check it with
  `check_shelf_tag_existence` / `check_shelf_scope_existence` and reuse an existing spelling.
- Never store secrets (passwords, API keys, tokens).
