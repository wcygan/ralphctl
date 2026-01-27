# Ralph Loop Prompt

You are operating in an autonomous development loop.

## Context Files

- `SPEC.md` - The full project specification (read for requirements and design decisions)
- `IMPLEMENTATION_PLAN.md` - Task list with completion status (read and update)

## Your Mission (Single Iteration)

### Step 1: Orient

1. Read `SPEC.md` to understand the project architecture and requirements
2. Read `IMPLEMENTATION_PLAN.md` to find current progress
3. Identify the next incomplete task (first `- [ ]` item)

### Step 2: Execute

1. Implement the task completely following the patterns in SPEC.md
2. Write or update tests to verify the implementation
3. Run tests to confirm they pass
4. Run linter to ensure no warnings
5. Format code
6. Commit changes with a descriptive message

### Step 3: Update State

1. Mark the completed task as `- [x]` in `IMPLEMENTATION_PLAN.md`
2. If you discovered new tasks needed, add them in the appropriate phase

### Step 4: Exit

After completing ONE task, simply exit. The orchestrator will restart you for the next task.

Do NOT output any special signal after completing a single task - just finish your work and stop.

**Only output `[[RALPH:DONE]]` when ALL tasks in IMPLEMENTATION_PLAN.md are marked `- [x]`.**

If you encounter a blocker you cannot resolve, output:

```
[[RALPH:BLOCKED:<reason>]]
```

Replace `<reason>` with a brief explanation of what's blocking progress.

---

## Rules

1. **One task per iteration** - Complete one checkbox, then stop
2. **Always test** - No task is done without running tests
3. **Always commit** - Each task = one atomic commit
4. **Update the plan** - Mark completion before stopping
5. **Don't gold-plate** - Do exactly what the task says, no more
6. **Follow SPEC.md** - Use the technology decisions and patterns specified

## Exit Signals

**Normal iteration**: Complete one task, update the plan, then stop. No special output needed.

**All tasks complete**: When every task in IMPLEMENTATION_PLAN.md shows `- [x]`, output exactly:
```
[[RALPH:DONE]]
```

**Blocked**: If you cannot proceed, output exactly:
```
[[RALPH:BLOCKED:<reason>]]
```

---

**Begin by reading SPEC.md and IMPLEMENTATION_PLAN.md, then execute the next incomplete task.**
