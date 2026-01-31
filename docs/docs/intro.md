---
sidebar_position: 0
title: Introduction
slug: /intro
---

# ralphctl

ralphctl is a CLI for managing Ralph Loop workflows -- autonomous development sessions driven by Claude. It orchestrates `claude` subprocess calls to execute iterative development tasks defined in markdown files.

## Workflow

```
ralphctl init → ralphctl interview → ralphctl run → ralphctl archive
```

1. **`init`** scaffolds the project files (SPEC.md, IMPLEMENTATION_PLAN.md, PROMPT.md) from templates.
2. **`interview`** launches an AI-guided session to flesh out your spec and implementation plan.
3. **`run`** executes the autonomous loop: each iteration reads the current state, implements the next task, and signals whether to continue.
4. **`archive`** saves completed work to a timestamped directory and resets the workspace.

## Getting Started

Install ralphctl and scaffold a new project:

```bash
ralphctl init
ralphctl interview
ralphctl run
```

Check progress at any time:

```bash
ralphctl status
```

## Learn More

- [The Philosophy](./philosophy/the-philosophy.md) -- why the Ralph Loop works
- [How ralphctl Implements It](./philosophy/how-ralphctl-implements-it.md) -- design decisions mapped to principles
- [Future Improvements](./philosophy/future-improvements.md) -- where we're headed
