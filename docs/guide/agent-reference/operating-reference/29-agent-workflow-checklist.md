# Complete reference details

This page preserves the complete technical detail for this source section. The shorter handbook pages explain the subject first in everyday language.

## 29. Agent workflow checklist

Before editing:

- read `AGENTS.md` and this guide;
- inspect `git status` and confirm the repository boundary;
- locate the canonical source, facade, tests, and docs for the feature;
- classify the requested behavior as implemented, bounded, designed, or new;
- check file/function size and module ownership.

While editing:

- keep the compiler pipeline one-directional;
- preserve ownership role spelling and explicit call-site markers;
- keep unsafe bridges private and typed facades public;
- preserve Actus `"""` documentation strings;
- add accepted and rejected tests;
- update LSP/formatter/diagnostic behavior for syntax changes;
- do not add a target-specific language special case;
- do not mark planned work complete without direct evidence.

Before handoff:

```sh
cargo fmt --all -- --check
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
scripts/check_source_limits.sh
git diff --check
```

For Actus source or standard-library changes, also run the relevant `actus`
check/build/test commands and native execution tests. For documentation-only
changes, run the documentation consistency checks and still inspect links and
claims manually.
