# Hypatia Memory plugin for Codex

Scaffold for the Codex plugin that integrates with the `hypatia-mcp-server` in this repository.
It uses the portable Agent Plugins layout (root `plugin.json` and `mcp.json`).

| Path | Purpose |
|---|---|
| [plugin.json](plugin.json) | Plugin manifest; OpenAI presentation metadata lives under `extensions.com.openai` |
| [mcp.json](mcp.json) | MCP server `hypatia_memory` at `http://localhost:8000/mcp` (the server's default) |
| [hooks/hooks.json](hooks/hooks.json) | `SessionStart`, `UserPromptSubmit` and `Stop` hooks calling the server's hook tools (auto-discovered by Codex) |
| [skills/](skills/) | Skills (placeholder: `hypatia-memory`) |

## Install

1. Start the server: `hypatia-mcp-server serve`.
2. Add this repository as a plugin marketplace (it exposes
   [.agents/plugins/marketplace.json](../../.agents/plugins/marketplace.json)):
   `codex plugin marketplace add maxshine/hypatia-mcp-server`
3. Install `hypatia-codex-mcp-plugin` from the plugin directory in Codex.
4. Review and trust the plugin's hooks. Codex skips plugin-bundled hooks until you do.

Hooks are on by default in Codex. Plugins that bundle lifecycle hooks are not eligible
for the public plugin directory, so distribution is through this repository's marketplace.
