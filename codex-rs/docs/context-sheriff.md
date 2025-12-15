# Context Sheriff (codex-context-sheriff)

This repository is a fork of the open source Codex CLI.

## Thesis

Many reports of “model degradation” are actually **user/agent drift** after context compaction or working-tree changes that are not obvious in the UI.

This fork focuses on making those boundaries observable without changing Codex core behavior.

## Flagship features (Tasks 001004)

- **Compaction summary is visible in the transcript** (manual + auto), so you can see what the next turn will “know”.
- **`/compact --preview`** lets you inspect the proposed summary before applying a manual compaction.
- **Working tree drift notice** warns (non-blocking, opt-out) when files changed outside the session.
- **Subtle fork branding** in the TUI header and CLI about/help output.

## What this fork intentionally does not change

- No token dashboards.
- No new blocking prompts.
- No changes to tool behavior, approvals, or auto-compaction mechanics.

## How to try it

- Install and usage follow upstream Codex CLI docs; start at `README.md`.
- For local development commands, use the workspace `just` recipes.
