# Hypatia Memory plugin for Codex

**Version 1.0.0**, for [`hypatia-mcp-server`](../../README.md#version) 1.0.0. The plugin and the
server share one version number; use the plugin release that matches your server.

Scaffold for the Codex plugin that integrates with the `hypatia-mcp-server` in this repository.
It uses the portable Agent Plugins layout (root `plugin.json` and `mcp.json`).

| Path | Purpose |
|---|---|
| [plugin.json](plugin.json) | Plugin manifest; OpenAI presentation metadata lives under `extensions.com.openai` |
| [mcp.json](mcp.json) | MCP server `hypatia_memory` at `http://localhost:8000/mcp` (the server's default) |
| [hooks/hooks.json](hooks/hooks.json) | `SessionStart`, `UserPromptSubmit` and `Stop` hooks calling the server's hook tools (not discovered by Codex yet; see [Enable the hooks](#enable-the-hooks-codex-bug-workaround)) |
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
4. Enable the hooks with the [workaround below](#enable-the-hooks-codex-bug-workaround).
   Codex currently does not load this plugin's bundled hooks, so `/hooks` shows none after install.

Hooks are on by default in Codex. Plugins that bundle lifecycle hooks are not eligible
for the public plugin directory, so distribution is through this repository's marketplace.

## Enable the hooks (Codex bug workaround)

**Symptom:** the MCP tools and skills work after install, but `/hooks` in Codex CLI lists none of
the plugin's hooks, and nothing is recorded or injected at session start, prompt submit or stop.

**Cause:** a Codex bug. Codex only loads bundled hooks from plugins with a legacy
`.codex-plugin/plugin.json` manifest. For plugins with a portable Agent Plugins root `plugin.json`,
like this one, the hook-only loader and the plugin-detail path return no hooks, even though the
manifest declares `extensions.com.openai.hooks` (`./hooks/hooks.json`). Reports on Codex 0.166.0
and 0.167.0 confirm this for `hooks/list` and `plugin/read` after a normal local marketplace
install. Tracked in [openai/codex#59516](https://github.com/openai/codex/issues/59516), which
is open and has no maintainer response or fix yet.

**Check first:** the bug may already be fixed in your Codex version, in which case you don't need
the workaround.

1. Check your version with `codex --version`. The bug is confirmed on 0.166.0 and 0.167.0.
2. Check whether [openai/codex#59516](https://github.com/openai/codex/issues/59516) has been
   closed, and which release includes the fix.
3. Install the plugin, restart Codex and run `/hooks`. If the plugin's `SessionStart`,
   `UserPromptSubmit` and `Stop` hooks are listed, trust them there and skip the workaround.
   Apply it only if they are missing.

**Workaround:** copy the hook definitions from [hooks/hooks.json](hooks/hooks.json) into the
user-level hooks file `$CODEX_HOME/hooks.json` (default `~/.codex/hooks.json`). Codex reads that
file and the hooks then work as they do from a plugin.

1. If `~/.codex/hooks.json` does not exist, copy the file:

   ```bash
   mkdir -p ~/.codex
   cp codex/hypatia-codex-mcp-plugin/hooks/hooks.json ~/.codex/hooks.json
   ```

   If it already exists, do not overwrite it. Merge the `SessionStart`, `UserPromptSubmit` and
   `Stop` entries from this plugin's `hooks/hooks.json` into the existing `hooks` object.

2. Keep the `hypatia_memory` MCP server enabled: the hooks call its tools by that name, and it
   still comes from the installed plugin's [mcp.json](mcp.json) (see the notes under
   [Customise the MCP server settings](#customise-the-mcp-server-settings)).
3. Restart Codex, open `/hooks`, and review and trust the three hooks. Codex does not run hooks
   until you trust them.

Notes:

- The `hooks.json` files are plain copies, so they do not follow plugin updates. Re-copy after
  you update the plugin if its hooks change.
- Once Codex fixes the issue, remove the copied entries from `~/.codex/hooks.json`. Otherwise
  each event runs both the user-level and the plugin hook, and messages are recorded twice.

References:

- [openai/codex#59516](https://github.com/openai/codex/issues/59516): "Codex CLI 0.166 does not
  honor plugins bundled hooks declared in AgentPlugin manifest form".
- Source locations identified in the issue, in `codex-rs/core-plugins/src`: `loader.rs` (hook-only
  loader skips AgentPlugin manifests), `manager.rs` (plugin details skip them too) and
  `agent_plugin_manifest.rs` (the `hooks` extension is parsed but not used).

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
