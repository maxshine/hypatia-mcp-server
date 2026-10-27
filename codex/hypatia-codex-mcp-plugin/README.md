# Hypatia Memory plugin for Codex

Scaffold for the Codex plugin that integrates with the `hypatia-mcp-server` in this repository.
It uses the portable Agent Plugins layout (root `plugin.json` and `mcp.json`).

| Path | Purpose |
|---|---|
| [plugin.json](plugin.json) | Plugin manifest; OpenAI presentation metadata lives under `extensions.com.openai` |
| [mcp.json](mcp.json) | MCP server `hypatia_memory` at `http://localhost:8000/mcp` (the server's default) |
| [hooks/hooks.json](hooks/hooks.json) | `SessionStart`, `UserPromptSubmit` and `Stop` hooks calling the server's hook tools (auto-discovered by Codex) |
| [skills/hypatia](skills/hypatia/SKILL.md) | Natural-language knowledge CRUD, triples, JSE queries, search and shelf management via the MCP tools |
| [skills/hypatia-memory](skills/hypatia-memory/SKILL.md) | Responds to the hooks' `TRIGGER:*` signals: summary cascade, work-unit extraction, remember/recall/forget |
| [skills/hypatia-dream](skills/hypatia-dream/SKILL.md) | Incremental graph consolidation at a work-period boundary (`report` / `apply` modes), with [evals](skills/hypatia-dream/evals/evals.json) |

The skills are adapted from the [upstream Hypatia skills](https://github.com/MarchLiu/hypatia/tree/main/skills),
with `hypatia` CLI commands replaced by the `hypatia_memory` MCP tools.

## Install

1. Start the server: `hypatia-mcp-server serve`.
2. Add this repository as a plugin marketplace (it exposes
   [.agents/plugins/marketplace.json](../../.agents/plugins/marketplace.json)):
   `codex plugin marketplace add maxshine/hypatia-mcp-server`
3. Install `hypatia-codex-mcp-plugin` from the plugin directory in Codex.
4. Review and trust the plugin's hooks. Codex skips plugin-bundled hooks until you do.

Hooks are on by default in Codex. Plugins that bundle lifecycle hooks are not eligible
for the public plugin directory, so distribution is through this repository's marketplace.

## TODO

### MCP tools missing compared with the `hypatia` CLI

The [skills](skills/) were adapted from the upstream CLI-based skills. These CLI features have no
MCP tool yet, so the skills work around them:

| Missing CLI feature | Current workaround in the skills | Where |
|---|---|---|
| `hypatia scope list` / `scope exists`, `hypatia tag list` / `tag exists` | Probe with `query` + `$has` and `"limit": 1`; sample related entries to find existing spellings | [hypatia](skills/hypatia/SKILL.md) "Checking scopes and tags already in use"; referenced from [hypatia-memory](skills/hypatia-memory/SKILL.md) "Session Startup" |
| `hypatia archive-store`, `hypatia archive-get` | Only `list_archives` is offered; the agent reports that storing or copying out archives is unsupported | [hypatia](skills/hypatia/SKILL.md) "Archive Files" |
| `hypatia export` | [hypatia](skills/hypatia/SKILL.md) states export is unavailable; [hypatia-dream](skills/hypatia-dream/SKILL.md) takes a statement-level JSON backup of every triple it deletes (`~/.hypatia/backups/hypatia-dream-<SHELF>-<ts>.json`, or inline in the report) | [hypatia](skills/hypatia/SKILL.md) "Shelf Management"; [hypatia-dream](skills/hypatia-dream/SKILL.md) "Apply Changes" step 2 and report; [eval 2](skills/hypatia-dream/evals/evals.json) |
| `hypatia connect <path>` (arbitrary path) | `connect_shelf` only opens `~/.hypatia/<shelf_name>` | [hypatia](skills/hypatia/SKILL.md) "Shelf Management" |
| `hypatia session-current --scope` | `query` with `$not-summaried` | [hypatia-memory](skills/hypatia-memory/SKILL.md) "Hierarchical Summary Cascade" step 1 |
| `hypatia backfill` (full pass) | `backfill_shelf` processes one batch per call; call it repeatedly until embedding debt is zero | [hypatia](skills/hypatia/SKILL.md) "Vector Similarity Search" |
| Current time | No MCP tool returns the time, so [hypatia-dream](skills/hypatia-dream/SKILL.md) runs `date -u` for its snapshot boundary | [hypatia-dream](skills/hypatia-dream/SKILL.md) "Preflight And Snapshot Boundary" step 3 |

Adding scope/tag listing and shelf export tools would remove the two biggest workarounds.

### Hook issues

- [x] [hooks/hooks.json](hooks/hooks.json) now sends every field
  [hypatia_memory_hooks.rs](../../src/services/hypatia_memory_hooks.rs) requires (`turn_id` and
  `last_assistant_message` for `SessionStart`, `last_assistant_message` for `UserPromptSubmit`).
  Codex does not define these values for those events, so they may arrive empty or unexpanded;
  the server only checks that the keys exist and ignores their values there. Relaxing the
  server's required-key lists would be the cleaner long-term fix.
- [x] User and assistant messages used to share the name `msg_<session>_<turn>_a`, so the `Stop`
  hook found the user's entry and never stored the assistant message. `format_message_name` now
  takes a `MessageRole`: user prompts are stored as `msg_<session>_<turn>_u` and assistant replies
  as `msg_<session>_<turn>_a`. Entries written before this fix are user prompts despite the `_a`
  suffix; check their `## Role` section.
- [ ] Message entries are meant to be stored without embedding, but the hooks pass
  `with_embed(false.then_some(false))`, which is `None`, so they are embedded.
