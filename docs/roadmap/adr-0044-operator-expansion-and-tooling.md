# ADR-0044 Roadmap: Operator Expansion and Tooling Synchronization

This roadmap implements ADR-0044 through the complete Actus compiler pipeline.
Every gate requires positive and negative evidence before it can be marked
complete. The final gate synchronizes the compiler surface with LSP, the VS
Code extension, and Tree-sitter.

## Gate 0: Operator contract and compatibility baseline

- [ ] Confirm ADR-0044 precedence, associativity, operand-family, and result
      type rules.
- [ ] Inventory existing lexer, parser, AST, semantic, codegen, runtime, and
      formatter support for all requested operators.
- [ ] Record compatibility requirements for ADR-0042 array, Buffer, cast,
      pack, and slot-loan behavior.
- [ ] Define stable diagnostics for invalid operands, shift counts, and
      remainder-by-zero cases.
- [ ] Add a baseline regression matrix before implementation changes.

## Gate 1: Lexer, parser, and AST operator surface

- [x] Add deterministic token handling for `==`, `!=`, `%`, `&&`, `||`, `&`,
      `|`, `^`, `~`, `<<`, and `>>`.
- [x] Add AST variants for equality, remainder, logical, bitwise, and shift
      expressions.
- [x] Parse the complete precedence table without changing ADR-0043
      relational precedence.
- [x] Parse unary `!` and `~` without confusing them with existing syntax.
- [x] Preserve complete source spans and formatter round-tripping.
- [x] Add accepted, malformed, precedence, associativity, and ambiguity tests.

## Gate 2: Semantic typing and invalid-program rejection

- [ ] Validate `==` and `!=` for compatible values without implicit casts.
- [ ] Validate `%` for supported signed and unsigned integer families.
- [ ] Require `Bool` operands for `&&`, `||`, and `!`.
- [ ] Validate integer-only bitwise operators and preserve width and signedness.
- [ ] Validate unsigned shift counts and define out-of-range count behavior.
- [ ] Reject float bitwise operations, numeric truthiness, mixed equality
      families, and incompatible operand widths where required.
- [ ] Add deterministic positive and negative semantic tests for every family.

## Gate 3: Native arithmetic, bitwise, and equality lowering

- [ ] Lower integer equality and inequality with type-correct comparisons.
- [ ] Lower signed and unsigned remainder with explicit zero-divisor handling.
- [ ] Lower `&`, `|`, `^`, and `~` using native integer operations.
- [ ] Lower `<<` and `>>` with signedness-aware shift instructions and the
      approved shift-count contract.
- [ ] Preserve packed-field and indexed-array behavior when operator results
      are read from or written to those values.
- [ ] Add native execution and deterministic trap tests for arithmetic and
      bitwise operations.

## Gate 4: Short-circuit CFG and ownership correctness

- [ ] Lower `&&` and `||` as branch-based short-circuit CFG, never eager calls.
- [ ] Lower `!` as a Bool-preserving operation.
- [ ] Preserve expression values at CFG joins with deterministic block
      parameters or equivalent SSA construction.
- [ ] Preserve `erg`, `abs`, `dat`, and `ins` ownership transitions across
      skipped and evaluated right-hand operands.
- [ ] Preserve nested `case`, `break`, `continue`, return, and cleanup paths.
- [ ] Add side-effect, cleanup, loop, case, and native short-circuit tests.

## Gate 5: Runtime, diagnostics, and end-to-end quality

- [ ] Finalize runtime behavior for remainder-by-zero and invalid shift counts.
- [ ] Add stable diagnostics and renderer-independent diagnostic tests.
- [ ] Add compile-time constant evaluation tests for invalid operations where
      the failure is statically provable.
- [ ] Add native tests for all operator families and mixed invalid programs.
- [ ] Verify deterministic object output and repeated execution results.
- [ ] Run formatting, compilation, Clippy, full tests, source limits, and diff
      checks.
- [ ] Update ADR-0044 with command evidence and any approved deviations.

## Gate 6: Documentation and language-surface conformance

- [ ] Update the lexical map, language conventions, Alpha guarantees, and
      operator reference with the final implemented rules.
- [ ] Document short-circuit evaluation, numeric family restrictions, shift
      count behavior, and zero-divisor behavior.
- [ ] Add examples and accepted/rejected fixtures without embedding tests in
      implementation files.
- [ ] Verify the README, manifesto, roadmap, ADRs, and implementation agree.

## Gate 7: LSP, VS Code, and Tree-sitter synchronization (final gate)

- [ ] Add all ADR-0042 Gate 1–8 syntax and semantic surface to LSP support,
      including arrays, Buffer indexing, casts, packs, and slot loans.
- [ ] Add all ADR-0043 relational operators, nested-control diagnostics, and
      length-aware Buffer documentation to LSP support.
- [ ] Add ADR-0044 equality, remainder, logical, and bitwise operators to LSP
      tokenization, completion, hover, formatting, and diagnostics.
- [ ] Update the VS Code extension grammar, highlighting, completion, hover,
      diagnostics, and formatter integration.
- [ ] Update Tree-sitter grammar, precedence, generated parser artifacts, and
      corpus fixtures for the combined operator surface.
- [ ] Run cross-repository synchronization checks and reject drift between
      compiler, LSP, VS Code, and Tree-sitter definitions.
- [ ] Record end-to-end editor evidence and close ADR-0044 only after every
      prior gate and all three tooling integrations are green.
