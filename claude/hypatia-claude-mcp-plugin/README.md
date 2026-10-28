# Hypatia Memory plugin for Claude Code

Claude Code plugin that integrates with the `hypatia-mcp-server` in this repository. It is a
port of the [Codex plugin](../../codex/hypatia-codex-mcp-plugin/README.md): the same MCP server,
hooks and skills, adapted to Claude Code's plugin layout and hook inputs.

| Path | Purpose |
|---|---|
| [.claude-plugin/plugin.json](.claude-plugin/plugin.json) | Plugin manifest |
| [.mcp.json](.mcp.json) | MCP server `hypatia_memory` at `http://localhost:8000/mcp` (the server's default) |
| [hooks/hooks.json](hooks/hooks.json) | `SessionStart`, `UserPromptSubmit` and `Stop` hooks calling the server's hook tools |
| [skills/hypatia](skills/hypatia/SKILL.md) | Natural-language knowledge CRUD, triples, JSE queries, search, tag/scope vocabulary and shelf management via the MCP tools |
| [skills/hypatia-memory](skills/hypatia-memory/SKILL.md) | Responds to the hooks' `TRIGGER:*` signals: summary cascade, work-unit extraction, remember/recall/forget |
| [skills/hypatia-dream](skills/hypatia-dream/SKILL.md) | Incremental graph consolidation at a work-period boundary (`report` / `apply` modes), with [evals](skills/hypatia-dream/evals/evals.json) |

The skills are adapted from the [upstream Hypatia skills](https://github.com/MarchLiu/hypatia/tree/main/skills),
with `hypatia` CLI commands replaced by the `hypatia_memory` MCP tools. In Claude Code the tools
are named `mcp__plugin_hypatia-claude-mcp-plugin_hypatia_memory__<tool>`.

## Install

1. Start the server: `hypatia-mcp-server serve`.
2. Add this repository as a plugin marketplace (it exposes
   [.claude-plugin/marketplace.json](../../.claude-plugin/marketplace.json)):
   `/plugin marketplace add maxshine/hypatia-mcp-server`
3. Install the plugin: `/plugin install hypatia-claude-mcp-plugin@hypatia-mcp-server`
4. Check that the `hypatia_memory` server is connected with `/mcp`.

For local development, load the plugin without installing it:
`claude --plugin-dir ./claude/hypatia-claude-mcp-plugin`.

## Differences from the Codex plugin

| Area | Codex | Claude Code |
|---|---|---|
| Manifest | `plugin.json` at the plugin root | `.claude-plugin/plugin.json` |
| MCP config | `mcp.json`, `"type": "streamable-http"` | `.mcp.json`, `"type": "http"` |
| Hook `server` | `hypatia_memory` | `plugin:hypatia-claude-mcp-plugin:hypatia_memory` (plugin-scoped name) |
| `turn_id` hook input | Codex `turn_id` | Claude Code has no `turn_id`; the hooks pass `prompt_id`, which is one UUID per user prompt and is shared by that prompt's `UserPromptSubmit` and `Stop` |
| Inputs the event does not provide | Passed as `${...}` placeholders | Passed as `""` (`turn_id` and `last_assistant_message` on `SessionStart`, `last_assistant_message` on `UserPromptSubmit`): the server only requires the keys to exist. `model` is omitted because Claude Code does not send it on these events |
| `SessionStart` | Runs at launch and injects rules/taboos | Claude Code skips `mcp_tool` hooks on `SessionStart` at launch (including `--continue`/`--resume`), because MCP servers are not connected yet. The hook runs only after `/clear` or a compaction. The `hypatia-memory` skill falls back to loading rules and taboos itself |

## TODO

- [ ] Rules and taboos are not injected at launch (see `SessionStart` above). Fixing this needs a
  `type: "command"` `SessionStart` hook, for example a `hypatia-mcp-server` subcommand that
  calls `agent_session_start` and prints its JSON result.
- [ ] The plugin name contains the word `claude`, which `claude plugin validate` reports as a
  warning ("reads as one of Anthropic's own"). It still installs and loads; rename it (and the
  hook `server` values) if you want a clean validation.
- [ ] The remaining items in the [Codex plugin TODO](../../codex/hypatia-codex-mcp-plugin/README.md#todo)
  (missing MCP tools, message entries being embedded, scope/tag tool error handling) apply here
  too.
