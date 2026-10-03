# hypatia-mcp-server

A personal study project for practicing Rust and the Model Context Protocol (MCP).
It wraps the agent memory system from [MarchLiu/hypatia](https://github.com/MarchLiu/hypatia)
in a streamable-HTTP MCP server, so any MCP-capable agent (Codex, Claude Code, etc.)
can connect to it as a durable memory backend.

> **Status:** work in progress / hobby project. APIs, tool names, and behavior may
> change without notice. Not intended for production use.

## Credit

All core memory functionality — knowledge storage, embeddings, shelves, archives —
is implemented by [MarchLiu/hypatia](https://github.com/MarchLiu/hypatia). This
repository only adds an MCP transport layer (via [`rmcp`](https://crates.io/crates/rmcp))
on top of that library.

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

**NOTE**: There are a few dummies implemented for `rmcp` PoC and they are neglectable.  

## Roadmap

- [ ] Companion repositories with integration plugins for popular agent hosts
      (Codex first, and others will be following).
- [ ] More MCP surface area: additional tools, parameterized prompts, and resources.
- [ ] Fill the test automation gap.
- [ ] CI/CD automation to publish ready-to-run binaries.
- [ ] Proper documentation site around the project.

## License

See [LICENSE](LICENSE).
