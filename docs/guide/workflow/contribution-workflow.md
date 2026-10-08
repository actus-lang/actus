# Contribution workflow

Work from the latest `main` in a temporary focused branch. Keep one logical
change per commit and use a Conventional Commit message.

Before opening a pull request:

- inspect the complete diff;
- run formatting, checking, linting, tests, and relevant native commands;
- update the handbook, ADR, or roadmap when the contract changes;
- record exact failures and known boundaries;
- describe public API, ownership, binary, target, and compatibility impact.

Pull requests use the repository's configured merge policy. Do not merge while
required checks are red.
