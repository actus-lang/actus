# Source-change checklist

Before editing:

- confirm the repository and branch;
- preserve existing worktree changes;
- locate the canonical facade and public boundary;
- identify affected ownership roles and cleanup paths;
- inspect serialized layout and constants;
- find accepted and rejected tests;
- check source and function size.

After editing:

- format changed source;
- run focused checks first;
- run the required repository checks;
- inspect the diff and generated artifacts;
- update docs, ADRs, and roadmap evidence;
- keep the commit narrow and descriptive.
