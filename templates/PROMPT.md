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

### Step 4: Report & Signal

Before outputting a signal, you MUST provide a structured summary of the work completed. This helps human observers understand what happened in this iteration.

**Output this summary format:**

```
┌─────────────────────────────────────────────────────────────────┐
│ ITERATION SUMMARY                                               │
├─────────────────────────────────────────────────────────────────┤
│ Task: <task description from IMPLEMENTATION_PLAN.md>            │
│                                                                 │
│ Changes:                                                        │
│   • <file1> - <what changed>                                    │
│   • <file2> - <what changed>                                    │
│                                                                 │
│ Tests: <X passed, Y failed, or "N/A">                           │
│ Commit: <commit hash (short)> - <commit message>                │
│                                                                 │
│ Progress: <completed>/<total> tasks (<percentage>%)             │
│ Next: <brief description of next task, or "None - all complete">│
└─────────────────────────────────────────────────────────────────┘
```

Then output exactly one of these signals on its own line:

**Task completed, more tasks remain:**
```
[[RALPH:CONTINUE]]
```

**All tasks complete (every checkbox is `[x]`):**
```
[[RALPH:DONE]]
```

**Cannot proceed due to blocker:**
```
[[RALPH:BLOCKED:<reason>]]
```

---

## Rules

1. **One task per iteration** - Complete one checkbox, then signal
2. **Always test** - No task is done without running tests
3. **Always commit** - Each task = one atomic commit
4. **Update the plan** - Mark completion before signaling
5. **Always report** - Output the iteration summary before every signal
6. **Always signal** - End every iteration with the appropriate signal
7. **Don't gold-plate** - Do exactly what the task says, no more
8. **Follow SPEC.md** - Use the technology decisions and patterns specified

## Exit Signals (REQUIRED)

Every iteration MUST end with:
1. An **ITERATION SUMMARY** box (so observers know what happened)
2. Exactly one signal on its own line

| Signal | Meaning |
|--------|---------|
| `[[RALPH:CONTINUE]]` | Task completed, more tasks remain — orchestrator will start next iteration |
| `[[RALPH:DONE]]` | All tasks complete — orchestrator will exit successfully |
| `[[RALPH:BLOCKED:<reason>]]` | Cannot proceed — orchestrator will exit with error |

The orchestrator reads these signals to decide what to do next. Without a signal, the loop will pause and ask for manual intervention.

---

**Begin by reading SPEC.md and IMPLEMENTATION_PLAN.md, then execute the next incomplete task.**
