# hypatia-mcp-server

A personal study project for practicing Rust and the Model Context Protocol (MCP).
It wraps the agent memory system from [MarchLiu/hypatia](https://github.com/MarchLiu/hypatia)
in a streamable-HTTP MCP server, so any MCP-capable agent (Codex, Claude Code, etc.)
can connect to it as a durable memory backend.

> **Status:** work in progress / hobby project. APIs, tool names, and behavior may
> change without notice. Not intended for production use.

## Version

Current release: **1.0.0**. The server and both [agent plugins](#agent-plugins) share one
version number, so plugin `X.Y.Z` is built for and tested against server `X.Y.Z`.

| Component | Version | Version source |
|---|---|---|
| `hypatia-mcp-server` | 1.0.0 | [Cargo.toml](Cargo.toml); `hypatia-mcp-server --version` |
| Codex plugin | 1.0.0 | [codex/hypatia-codex-mcp-plugin/plugin.json](codex/hypatia-codex-mcp-plugin/plugin.json) |
| Claude Code plugin | 1.0.0 | [claude/hypatia-claude-mcp-plugin/.claude-plugin/plugin.json](claude/hypatia-claude-mcp-plugin/.claude-plugin/plugin.json) |

When releasing, bump all three together.

## Credit

All core memory functionality — memory storage, embeddings, shelves, archives and et al —
is implemented by [MarchLiu/hypatia](https://github.com/MarchLiu/hypatia). This
repository only adds an MCP transport layer (via [`rmcp`](https://crates.io/crates/rmcp))
on top of that library.
In addition, the available plugins for integration with popular agent hosts are just 
translations of [MarchLiu/hypatia](https://github.com/MarchLiu/hypatia) examples, with
adoption of tools provided by this repository MCP implementation.

## Install & run

> **Prerequisite:** this server binary can initialize a shelf;
> It is not handling complex setup and configuration requests.
> It only reads configuration that already exists, once, at startup. Use the
> [hypatia](https://github.com/MarchLiu/hypatia) engine's own CLI to manage
> shelves, install embedding models, and otherwise prepare your environment
> first

Clone the repository, then build and install the binary locally:

```bash
cargo install --path .
```

Initialize a custom shelf than the default:
```bash
hypatia-mcp-server init-shelf --shelf-name <name>
```

Start the server:

```bash
hypatia-mcp-server serve
```

By default it listens on `http://localhost:8000/mcp` (streamable HTTP transport).
Use `--port` and `--address` to change the bind address:

```bash
hypatia-mcp-server serve --port 1234 --address 0.0.0.0
```

The memory hooks, and any tool call that omits `shelf`, use the `default` shelf unless you pick
another with `--default-shelf` (`-s`). The shelf must exist first (`init-shelf --shelf-name
<name>`); the server states its default shelf in its MCP instructions so agents know about it:

```bash
hypatia-mcp-server serve --default-shelf work
```

## Available tools

These are the MCP tools currently exposed for agent use:  

| Tool | Description |
|---|---|
| `connect_shelf` | Connect to a hypatia memory shelf |
| `disconnect_shelf` | Disconnect a hypatia memory shelf from registry |
| `list_shelves` | List available hypatia memory shelves |
| `get_shelf_status` | Show a shelf's general status |
| `list_models` | List local embedding models available for hypatia memory |
| `list_archives` | List archives for a hypatia memory shelf |
| `create_knowledge` | Create a knowledge record for a hypatia memory shelf |
| `get_knowledge` | Get a knowledge record from a hypatia memory shelf |
| `update_knowledge` | Update a knowledge record in a hypatia memory shelf |
| `delete_knowledge` | Delete a knowledge record in a hypatia memory shelf |
| `create_statement` | Create a statement record in a hypatia memory shelf |
| `delete_statement` | Delete a statement record in a hypatia memory shelf |
| `query` | Query knowledges and statements from a hypatia memory shelf |
| `search` | Search for knowledges and statements in a hypatia memory shelf |
| `similar` | Find similar knowledges and statements in a hypatia memory shelf |

### Agent hook tools

A second set of tools is designed to be called from agent lifecycle hooks (e.g. Codex
`SessionStart`, `UserPromptSubmit`, and `Stop` hooks) rather than by the model itself.
Each takes the hook's JSON event payload as its arguments and returns a hook-style
response (`{"continue": true, ...}`). They work against the server's default shelf (`default`,
or the one given with `--default-shelf`) and use the
last component of the event's `cwd` as the project name (used as the knowledge scope).

| Tool | Hook event | Description |
|---|---|---|
| `agent_session_start` | `SessionStart` | Injects a "Hypatia Memory Context" into the session as `additionalContext`: the project, the session id, and the project/global knowledge tagged `rule` (follow) and `taboo` (avoid). |
| `user_prompt_submission` | `UserPromptSubmit` | Stores the user prompt as a `message` knowledge record, then returns `additionalContext` with trigger signals and up to 5 knowledge records recalled by searching on the prompt. |
| `agent_stop` | `Stop` | Stores the last assistant message as a `message` knowledge record (skipped if it already exists). |

**Trigger signals** returned by `user_prompt_submission`:

| Signal | Raised when |
|---|---|
| `TRIGGER:log` | Always; the message was already stored, no action needed |
| `TRIGGER:immediate` | The prompt contains `remember`, `recall`, or `forget` |
| `TRIGGER:extract` | The session has reached 5 or more turns |
| `TRIGGER:summary` | 16 or more `message` records for the project are not yet summarized |


**NOTE**: There are a few dummies implemented for `rmcp` PoC and they are neglectable.  

## JSE query language

The `query` tool's `jse` argument and the `similar` tool's `where` argument take
JSE (JSON Search Expression), the query language of the upstream
[hypatia engine](https://github.com/MarchLiu/hypatia#jse-query-language). JSE is passed
through to the engine unchanged, so the syntax is whatever the pinned engine version
(`v4.1.1`, see [Cargo.toml](Cargo.toml)) accepts. Pass it as a JSON value; a JSON-encoded
string is also accepted.

### Syntax

```json
["$knowledge", condition1, condition2, ...]
["$statement", condition1, condition2, ...]
{"$knowledge": [condition1, ...], "limit": 10, "offset": 0}
```

Conditions listed side by side are ANDed. With no conditions every entry is returned, up to
the default limit of 100, newest first. Use the object form to set `limit` and `offset`.

### Operators

| Operator | Purpose | Example |
|---|---|---|
| `$eq` / `$ne` | Equals / not equals | `["$eq", "name", "Rust"]` |
| `$gt` / `$lt` / `$gte` / `$lte` | Comparison | `["$gt", "created_at", "2025-01-01"]` |
| `$like` | SQL `LIKE` pattern | `["$like", "name", "Rust%"]` |
| `$contains` | Substring in a field; exact membership on `tags`, `scopes`, `figures` | `["$contains", "tags", "backend"]` |
| `$has` | Exact (indexed) membership in a JSON field | `["$has", "tags", "rule"]` |
| `$json-contains` | Structural containment of the content JSON | `["$json-contains", {"tags": ["rust"]}]` |
| `$content` | Match content JSON key-values | `["$content", {"format": "markdown"}]` |
| `$search` | Full-text search | `["$search", "database migration"]` |
| `$similar` | Vector similarity inside a query | `["$similar", "memory management"]` |
| `$and` / `$or` | Logical AND / OR; operands go directly, not in a nested array | `["$or", cond1, cond2]` |
| `$not` | Logical NOT | `["$not", cond]` |
| `$quote` | Prevent evaluation | `["$quote", ["$eq", "x", "y"]]` |
| `$triple` | Statement position match; `"$*"` is a wildcard | `["$triple", "Alice", "$*", "Bob"]` |
| `$k-hop` | K-hop graph traversal from a head | `["$k-hop", "Alice", "$*", 2]` |
| `$not-summaried` | Top level: knowledge with a tag that no `summary` statement points at | `["$not-summaried", "message", cond]` |

A field name is a table column (`name`, `created_at`, `head`, `relation`, `tail`, `triple`,
`tr_start`, `tr_end`) or a path into the content JSON (`data`, `tags`, `scopes`,
`meta.author`, `tags[0]`). A leading `$` is not part of the name. Invalid paths are rejected
with a validation error. Set `HYPATIA_DEBUG_SQL=1` in the server's environment to include the
generated SQL in error messages.

`similar`'s `where` takes a single condition (no `$knowledge`/`$statement` wrapper) and does
not accept `$search`, `$similar` or `$k-hop`.

### Examples

| Goal | Tool call |
|---|---|
| All knowledge | `query {"jse": ["$knowledge"]}` |
| Knowledge named `Rust` tagged `systems` | `query {"jse": ["$knowledge", ["$eq", "name", "Rust"], ["$has", "tags", "systems"]]}` |
| Project rules, including global ones | `query {"jse": ["$knowledge", ["$has", "tags", "rule"], ["$or", ["$has", "scopes", "my-app"], ["$has", "scopes", ""]]]}` |
| Everything Alice is the subject of | `query {"jse": ["$statement", ["$triple", "Alice", "$*", "$*"]]}` |
| The exact triple (primary-key lookup) | `query {"jse": ["$statement", ["$triple", "Alice", "knows", "Bob"]]}` |
| Entities reachable from Alice in 2 hops | `query {"jse": ["$statement", ["$k-hop", "Alice", "$*", 2]]}` |
| Full-text search, first 10 hits | `query {"jse": {"$knowledge": [["$search", "query optimization"]], "limit": 10}}` |
| Semantic search inside one scope | `similar {"query": "auth", "target": "knowledge", "where": ["$has", "scopes", "my-app"]}` |

## Agent plugins

This repository is also a plugin marketplace for two agent hosts. Each plugin bundles the MCP
server config, the lifecycle hooks above, and the `hypatia`, `hypatia-memory` and
`hypatia-dream` skills. Start the server first with `hypatia-mcp-server serve`.

| Host | Plugin | Install |
|---|---|---|
| Codex | [codex/hypatia-codex-mcp-plugin](codex/hypatia-codex-mcp-plugin/README.md) | `codex plugin marketplace add maxshine/hypatia-mcp-server`, then install `hypatia-codex-mcp-plugin` |
| Claude Code | [claude/hypatia-claude-mcp-plugin](claude/hypatia-claude-mcp-plugin/README.md) | `/plugin marketplace add maxshine/hypatia-mcp-server`, then `/plugin install hypatia-claude-mcp-plugin@hypatia-mcp-server` |

## Roadmap

### 2.0 — consolidated MCP tools

1.0 exposes one MCP tool per operation: about 20 memory tools plus the three hook tools.
An agent host loads every tool's name, description and JSON schema into the model's context,
so this many fine-grained tools costs context on every turn. It also makes tool selection
slower and more error-prone, and every skill and plugin has to track the whole list.

2.0 will group and consolidate the tools into a few, so they take less agent context and
perform better overall:

- [ ] Group the tools by domain into a few consolidated tools, each taking an
      `action` (or `op`) argument. For example:
      - `shelf`: list, connect, disconnect, status, models, archives, backfill
      - `knowledge`: create, get, update, delete
      - `statement`: create, delete
      - `retrieve`: query, search, similar
      - `vocabulary`: list and check tags and scopes
- [ ] Keep the hook tools separate from the model-facing tools, so hosts never offer
      `agent_session_start`, `user_prompt_submission` and `agent_stop` to the model.
- [ ] Use one response envelope for every tool (result or error, never a silent fallback),
      with short descriptions and compact schemas.
- [ ] Measure the before/after cost: tool-definition tokens per session, tool-selection
      accuracy, and round trips per typical memory task.
- [ ] Release the server and the Codex and Claude Code plugins together as 2.0.0, with the
      skills rewritten for the new tools and a migration note for 1.x tool names.

### Ongoing

- [ ] Companion repositories with integration plugins for popular agent hosts
      (Codex and Claude Code are available in this repository; others will be following).
- [ ] More MCP surface area: additional tools, parameterized prompts, and resources.
- [ ] Fill the test automation gap.
- [ ] CI/CD automation to publish ready-to-run binaries.
- [ ] Proper documentation site around the project.
- [ ] Active contributions to upstream Hypatia.

## License

See [LICENSE](LICENSE).
