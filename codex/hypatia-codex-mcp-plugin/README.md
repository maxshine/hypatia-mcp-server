# Hypatia Memory plugin for Codex

**Version 1.0.0**, for [`hypatia-mcp-server`](../../README.md#version) 1.0.0. The plugin and the
server share one version number; use the plugin release that matches your server.

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

## Customise the MCP server settings

The plugin does not start or configure `hypatia-mcp-server`; you run it yourself. The bundled
[mcp.json](mcp.json) assumes the server's defaults:

```json
"hypatia_memory": { "type": "streamable-http", "url": "http://localhost:8000/mcp" }
```

If your server listens somewhere else, Codex connects to the wrong place and the
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

### Option 2: point the plugin at your server

1. Start the server with the address you want and note the URL clients must use. The URL is
   `http://<host>:<port>/mcp`, where `<host>` is a name or IP that your machine can reach.
   `--address` only sets what the server binds to, so `0.0.0.0` is not a usable client host.

   ```bash
   hypatia-mcp-server serve --port 1234
   # client URL: http://localhost:1234/mcp
   ```

2. Edit `url` in the plugin's `mcp.json`, and change nothing else in the entry:

   ```json
   {
     "$schema": "https://agent-plugins.org/schemas/1.0.0/mcp.schema.json",
     "mcpServers": {
       "hypatia_memory": {
         "type": "streamable-http",
         "url": "http://localhost:1234/mcp"
       }
     }
   }
   ```

   Edit the copy of the plugin that Codex installed. Editing a checkout of this repository only
   helps if you install the plugin from that checkout (for example by adding your clone as the
   plugin marketplace). A plugin update replaces an installed copy, so repeat the edit after
   updating.

3. Restart Codex, then check that the server connects (for example with `codex mcp list` or the
   `/mcp` command) and that the `hypatia_memory` tools are listed.

### Also: tune the plugin's server in `config.toml` (not the URL)

Codex lets you adjust a plugin-provided MCP server from your own `~/.codex/config.toml` (or a
project-scoped `.codex/config.toml`), under `plugins.<plugin>.mcp_servers.<server>`. This
**cannot change the server's `url` or transport**: those always come from the plugin's
`mcp.json`, so it does not fix a wrong address. Use it to switch the server off or to set tool
policy, for example:

```toml
[plugins."hypatia-codex-mcp-plugin@hypatia-mcp-server".mcp_servers.hypatia_memory]
enabled = true
default_tools_approval_mode = "prompt"        # auto, prompt, writes or approve
disabled_tools = ["delete_knowledge"]         # tool deny list, applied after enabled_tools

[plugins."hypatia-codex-mcp-plugin@hypatia-mcp-server".mcp_servers.hypatia_memory.tools.search]
approval_mode = "approve"
```

- The plugin ID is `<plugin name>@<marketplace name>`: `hypatia-codex-mcp-plugin@hypatia-mcp-server`
  when installed from this repository's marketplace. Use the ID Codex shows for your install.
- The server key is `hypatia_memory`, the name in [mcp.json](mcp.json).
- Codex's general MCP server options include `enabled_tools`, `required` and
  `tools.<tool>.output_token_limit`. The docs only show `enabled`, the approval modes and the
  tool allow and deny lists for plugin servers, so test others before relying on them.
- Be careful with `enabled_tools` and `disabled_tools`. The hooks call `agent_session_start`,
  `user_prompt_submission` and `agent_stop`, and the skills use most of the other tools
  (`hypatia-dream` needs `delete_statement`). Hiding a tool can break them.
- This feature is documented at
  [codex-docs.com](https://www.codex-docs.com/en/docs/extend/mcp#plugin-provided-mcp-servers).
  Check it against your Codex version.

Notes:

- **Keep the server name `hypatia_memory`.** The hooks in [hooks/hooks.json](hooks/hooks.json)
  and the skills refer to the server by that name. If you rename it, the hooks stop working.
  Adding a second, differently named server in your own `config.toml` does not redirect the
  plugin's hooks.
- **Slow or remote servers.** Each hook has `"timeout": 10` (seconds) in
  [hooks/hooks.json](hooks/hooks.json). Raise it if the server is far away or slow to answer.
- **The server has no authentication.** Do not bind it to `0.0.0.0` or expose it beyond a
  network you trust. Prefer `localhost`, or an SSH tunnel to a remote machine.
- **Shelves live on the server's machine** (`~/.hypatia/<shelf>`), not on the machine running
  Codex. Initialise them where the server runs, with `hypatia-mcp-server init-shelf`.
- **If you hit connection errors**, confirm the URL answers (`curl -i http://localhost:8000/mcp`
  returns an HTTP response, not "connection refused") and that the server is running.

## TODO

### MCP tools missing compared with the `hypatia` CLI

The [skills](skills/) were adapted from the upstream CLI-based skills. These CLI features have no
MCP tool yet, so the skills work around them:

| Missing CLI feature | Current workaround in the skills | Where |
|---|---|---|
| `hypatia archive-store`, `hypatia archive-get` | Only `list_archives` is offered; the agent reports that storing or copying out archives is unsupported | [hypatia](skills/hypatia/SKILL.md) "Archive Files" |
| `hypatia export` | [hypatia](skills/hypatia/SKILL.md) states export is unavailable; [hypatia-dream](skills/hypatia-dream/SKILL.md) takes a statement-level JSON backup of every triple it deletes (`~/.hypatia/backups/hypatia-dream-<SHELF>-<ts>.json`, or inline in the report) | [hypatia](skills/hypatia/SKILL.md) "Shelf Management"; [hypatia-dream](skills/hypatia-dream/SKILL.md) "Apply Changes" step 2 and report; [eval 2](skills/hypatia-dream/evals/evals.json) |
| `hypatia connect <path>` (arbitrary path) | `connect_shelf` only opens `~/.hypatia/<shelf_name>` | [hypatia](skills/hypatia/SKILL.md) "Shelf Management" |
| `hypatia session-current --scope` | `query` with `$not-summaried` | [hypatia-memory](skills/hypatia-memory/SKILL.md) "Hierarchical Summary Cascade" step 1 |
| `hypatia backfill` (full pass) | `backfill_shelf` processes one batch per call; call it repeatedly until embedding debt is zero | [hypatia](skills/hypatia/SKILL.md) "Vector Similarity Search" |
| Current time | No MCP tool returns the time, so [hypatia-dream](skills/hypatia-dream/SKILL.md) runs `date -u` for its snapshot boundary | [hypatia-dream](skills/hypatia-dream/SKILL.md) "Preflight And Snapshot Boundary" step 3 |

Adding a shelf export tool would remove the biggest remaining workaround.

Resolved: `hypatia scope list` / `scope exists` and `hypatia tag list` / `tag exists` are now
served by `list_shelf_scopes`, `check_shelf_scope_existence`, `list_shelf_tags` and
`check_shelf_tag_existence`, and the skills use them directly.

### Scope/tag tool follow-ups

- [ ] On any error (for example the shelf is not connected) the four tools return an empty map
  or `"exists": false` instead of a tool error, which reads as "not in use". Returning the error
  would stop the agent from inventing a new spelling on a shelf it could not read.
- [x] The existence tools now echo the value under the same key on both paths (`tag_value` for
  tags, `scope_value` for scopes).
- [ ] `GenericNameValueArgs` in [hypatia_memory_tools.rs](../../src/services/hypatia_memory_tools.rs)
  is unused.

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
