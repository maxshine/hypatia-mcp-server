# Hypatia Memory plugin for Claude Code

**Version 1.0.0**, for [`hypatia-mcp-server`](../../README.md#version) 1.0.0. The plugin and the
server share one version number; use the plugin release that matches your server. Claude Code
pins an installed plugin to the `version` in [plugin.json](.claude-plugin/plugin.json), so users
get updates only when that number changes.

Claude Code plugin that integrates with the `hypatia-mcp-server` in this repository. It is a
port of the [Codex plugin](../../codex/hypatia-codex-mcp-plugin/README.md): the same MCP server,
hooks and skills, adapted to Claude Code's plugin layout and hook inputs.

| Path | Purpose |
|---|---|
| [.claude-plugin/plugin.json](.claude-plugin/plugin.json) | Plugin manifest, including the `server_url` option (see [Customise the MCP server settings](#customise-the-mcp-server-settings)) |
| [.mcp.json](.mcp.json) | MCP server `hypatia_memory`; its URL is the `server_url` option, which defaults to `http://localhost:8000/mcp` (the server's default) |
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

## Customise the MCP server settings

The plugin does not start or configure `hypatia-mcp-server`; you run it yourself. The server's
URL is the plugin option `server_url` (Claude Code v2.1.269 or later), which defaults to the
server's own default, `http://localhost:8000/mcp`. [.mcp.json](.mcp.json) reads it:

```json
"hypatia_memory": { "type": "http", "url": "${user_config.server_url}" }
```

If your server listens somewhere else, Claude Code connects to the wrong place and the
`hypatia_memory` tools and hooks fail. This happens when you:

- start the server with `--port` or `--address` (for example `serve --port 1234`);
- run it on another machine, in a container or VM, or on the Windows side of WSL;
- already have another service on port 8000.

Pick one of the two fixes.

### Option 1: run the server where the plugin expects it

No plugin change is needed. Free port 8000 and start the server with its defaults:

```bash
hypatia-mcp-server serve
```

### Option 2: set the plugin's `server_url`

> **Requires Claude Code v2.1.269 or later.** Older versions may not show or apply the
> plugin's `server_url` option. Upgrade Claude Code, or use Option 1.

1. Start the server with the address you want and note the URL clients must use. The URL is
   `http://<host>:<port>/mcp`, where `<host>` is a name or IP that your machine can reach.
   `--address` only sets what the server binds to, so `0.0.0.0` is not a usable client host.

   ```bash
   hypatia-mcp-server serve --port 1234
   # client URL: http://localhost:1234/mcp
   ```

2. Give the plugin that URL. Any one of these works:

   - In Claude Code, run `/plugin configure hypatia-claude-mcp-plugin@hypatia-mcp-server`
     and enter the URL. The same dialog opens when you install or enable the plugin, and
     there is a row for it in `/config`.
   - From a shell, when installing:

     ```bash
     claude plugin install hypatia-claude-mcp-plugin@hypatia-mcp-server \
       --config server_url=http://localhost:1234/mcp
     ```

     For an installed plugin, pipe a JSON object instead:

     ```bash
     echo '{"server_url": "http://localhost:1234/mcp"}' | \
       claude plugin configure hypatia-claude-mcp-plugin@hypatia-mcp-server --values-stdin
     ```

   - Edit your user `~/.claude/settings.json`. This is where Claude Code stores the value the
     other two ways save, so you can also write it by hand:

     ```json
     {
       "pluginConfigs": {
         "hypatia-claude-mcp-plugin@hypatia-mcp-server": {
           "options": {
             "server_url": "http://localhost:1234/mcp"
           }
         }
       }
     }
     ```

   Only user and managed settings are read for `pluginConfigs`. Entries in a project's
   `.claude/settings.json` or `.claude/settings.local.json` are ignored, so a cloned repository
   cannot redirect the plugin. The value survives plugin updates.

3. Reload the plugin (`/reload-plugins`) or restart Claude Code, then run `/mcp` and check that
   `hypatia_memory` is connected.

`settings.json` can only set options a plugin declares. It cannot change an arbitrary field of
the plugin's MCP entry, and it is not a general way to override a plugin's bundled MCP
server. This plugin declares `server_url` and nothing else. To change anything else
(the transport, headers, or the server name), edit the plugin's `.mcp.json` in your clone of this
repository and load it with `claude --plugin-dir ./claude/hypatia-claude-mcp-plugin`.
An installed copy is replaced when the plugin updates.

Notes:

- **Keep the server name `hypatia_memory`.** The hooks in [hooks/hooks.json](hooks/hooks.json)
  call it as `plugin:hypatia-claude-mcp-plugin:hypatia_memory`, and the skills use its tools.
  If you rename it, the hooks stop working. Adding your own server with `claude mcp add` does
  not redirect the plugin's hooks and would list a second set of tools.
- **Slow or remote servers.** Each hook has `"timeout": 10` (seconds) in
  [hooks/hooks.json](hooks/hooks.json). Raise it if the server is far away or slow to answer.
- **The server has no authentication.** Do not bind it to `0.0.0.0` or expose it beyond a
  network you trust. Prefer `localhost`, or an SSH tunnel to a remote machine.
- **Shelves live on the server's machine** (`~/.hypatia/<shelf>`), not on the machine running
  Claude Code. Initialise them where the server runs, with `hypatia-mcp-server init-shelf`.
- **If you hit connection errors**, confirm the URL answers (`curl -i http://localhost:8000/mcp`
  returns an HTTP response, not "connection refused") and that the server is running.
  In `claude -p` runs, a failed plugin server is skipped for 15 minutes before it is tried again.

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
