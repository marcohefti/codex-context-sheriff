## Slash Commands

### What are slash commands?

Slash commands are special commands you can type that start with `/`.

---

### Built-in slash commands

Control Codex’s behavior during an interactive session with slash commands.

| Command      | Purpose                                                     |
| ------------ | ----------------------------------------------------------- |
| `/model`     | choose what model and reasoning effort to use               |
| `/approvals` | choose what Codex can do without approval                   |
| `/review`    | review my current changes and find issues                   |
| `/new`       | start a new chat during a conversation                      |
| `/resume`    | resume an old chat                                          |
| `/init`      | create an AGENTS.md file with instructions for Codex        |
| `/compact`   | summarize conversation to prevent hitting the context limit |
| `/undo`      | ask Codex to undo a turn                                    |
| `/diff`      | show git diff (including untracked files)                   |
| `/mention`   | mention a file                                              |
| `/status`    | show current session configuration and token usage          |
| `/mcp`       | list configured MCP tools                                   |
| `/logout`    | log out of Codex                                            |
| `/quit`      | exit Codex                                                  |
| `/exit`      | exit Codex                                                  |
| `/feedback`  | send logs to maintainers                                    |

---

### `/compact`

Summarize the current conversation context into a bridge to free up context window.

- `/compact` rewrites the session history immediately.
- `/compact --preview` shows what the summary would be without rewriting history (side-effect-free). The preview includes a carry-over estimate and can be applied with `/compact --apply`.
- `/compact --apply` applies the latest preview verbatim (does not re-generate).
