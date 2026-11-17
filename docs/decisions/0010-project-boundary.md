# 0010. A project boundary stops store discovery

- **Status:** ✅ accepted
- **Date:** 2026-11-16
- **Deciders:** coding-lanes

## Context

Projects can be nested. A repository may sit inside a directory that is itself a
managed project, and the current project may have a `coding-lanes/` folder before
it has a `work/` folder.

Two discovery rules were possible. Finding the nearest `coding-lanes/work/`
could cross the current project's boundary and return an ancestor's tasks. A
project that has not created its work folder yet would then show another
project's work as if it were its own.

## Decision

**The nearest `coding-lanes/` folder is the project boundary. Discovery stops
there.**

- The start path is normalized before the ancestor walk.
- The first ancestor containing `coding-lanes/` owns the result. Its
  `work/` may be present or absent.
- Discovery never continues past that project to an ancestor project.
- Core owns the rule. Its lenient form returns the nearest project's work path
  even when the folder is absent, so a writer can create it. Its strict form
  reports that the project has no work folder, so a reader can say so.

The rejected alternative was to keep walking until a `coding-lanes/work/`
folder appeared. It made a half-made project usable by accident, but it also
made project ownership depend on which folders happened to exist.

## Consequences

### Positive

- A project never borrows another project's tasks.
- Core, raul, and the writers share one boundary rule.
- A missing work folder is a distinct state, not a missing project.

### Negative

- A project with `coding-lanes/` but no `work/` is not initialized by a reader.
  A writer creates the work folder when it first writes.

### Follow-up

- Windows-only path cases are covered by compile-checked tests; CI currently
  runs Ubuntu only.
