---
name: Epic
about: Tracking issue that decomposes into task issues
title: 'Phase N: <goal>' 
labels: 'epic'
assignees: ''
---

## Summary

[2-4 sentences. The goal of this body of work and what's different when it's complete.]

Full spec: [link to design doc or roadmap section]

## Task issues

### [Layer name] (e.g., Library modules, CLI commands)

- [ ] #N — `module/` — [brief description]
- [ ] #M — `other/` — [brief description]

## Dependency graph

```
#A ──┬── #C
     │
#B ──┴── #D
```

## Acceptance criteria

[System-level criteria, not per-task. What's true when the epic is done.]

- [ ] [Overall behaviour that proves the epic is complete]
- [ ] [Quality gate: no regressions, all tests pass, docs updated]
