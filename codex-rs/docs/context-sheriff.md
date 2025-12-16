# Context Sheriff (codex-context-sheriff)

This repository is a fork of the open source Codex CLI.

## Thesis

Many reports of “model degradation” are actually **user/agent drift** after context compaction or working-tree changes that are not obvious in the UI.

This fork focuses on making those boundaries observable without changing Codex core behavior.

If you want the longer-form writeup that motivated these changes, see: https://blog.heftiweb.ch/p/context-amnesia

## What this fork adds

### Compaction summary is visible

Compaction can discard the agent’s own “receipts” (what it said, what tools it ran, and what it verified). The terminal scrollback still shows everything, but the model continues from a rewritten bridge.

This fork renders the injected summary as a visible transcript entry so you can see what the next turn will actually “know” and immediately correct drift.

- Issue: https://github.com/marcohefti/codex-context-sheriff/issues/1
- PR: https://github.com/marcohefti/codex-context-sheriff/pull/4

### Manual compaction preview/apply

Manual compaction is high-impact: it rewrites the conversation into a bridge. Without a preview, applying it can feel like a leap of faith.

This fork adds a side-effect-free preview mode plus an apply step so you can validate that the summary captures the right progress/constraints before it becomes the new source of truth.

- Issue: https://github.com/marcohefti/codex-context-sheriff/issues/2
- PR: https://github.com/marcohefti/codex-context-sheriff/pull/5

### Working tree change warning

If files changed outside the session between turns (manual edits, other tools, parallel sessions), Codex can end up fighting an unexpected diff and users experience it as random rollbacks/overwrites.

This fork emits a non-blocking warning listing a few changed paths, so you can explicitly re-ground the agent before continuing. It can be disabled via config.

- Issue: https://github.com/marcohefti/codex-context-sheriff/issues/3
- PR: https://github.com/marcohefti/codex-context-sheriff/pull/6

This fork also includes subtle branding in the TUI header and CLI about/help output.

## What this fork intentionally does not change

- No token dashboards.
- No new blocking prompts.
- No changes to tool behavior, approvals, or auto-compaction mechanics.

## How to try it

- Install and usage follow upstream Codex CLI docs; start at `README.md`.
- For local development commands, use the workspace `just` recipes.
