# Content-type packages

See [ADR-content-type-packages](../../alpha-docs/ADR-content-type-packages.md) and [ADR-trust-planes-git](../../alpha-docs/ADR-trust-planes-git.md).

Alpha owns the **host** (validate schema, resolve declarative chat cards, open native or frame).  
Packages declare **schema + chat + open** — never custom chat-bubble JS/CSS.

```
content-types/
  host/                 # contracts + frame bridge
  first-party/          # Mail, Tasks, Projects, Sheets, Notes, AI threads, Git repos, Files
  examples/             # sample Drive leaves (e.g. product git pointers)
```

Entity JSON schemas remain in `../schemas/` (`alpha.task/4`, …). Each package `manifest.json` points at one.

**Git:** `alpha.git_repo/1` is a pointer to `git.{apex}` (or GitHub). Do not store packfiles on the cryptree.
