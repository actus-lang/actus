# Complete reference details

This page preserves the complete technical detail for this source section. The shorter handbook pages explain the subject first in everyday language.

## LSP and editor behavior

The compiler-backed LSP supports a workspace/document model with versioned
overlays and module-aware analysis. An agent changing language syntax must
consider all of the following, not only compiler acceptance:

- diagnostics and source spans;
- hover and semantic model data;
- completion and signature help;
- definition/navigation;
- rename and open-document overlays;
- semantic tokens;
- code lenses;
- formatting and idempotence;
- cancellation, stale versions, and invalid ranges;
- URI normalization across platforms;
- malformed JSON-RPC and missing documents without process termination.

Unsaved source must be analyzed consistently with the current in-memory
workspace. A new syntax is not complete if the LSP treats it as invalid,
reorders its docstrings, or reports stale module interfaces.
