# Phase 52: Source Layout Audit

## Status: Proposed

## Scope and invariants

Phase 52 reorganizes `src/` without changing compiler behavior. Directory
roots are facades, implementation files are named for their responsibility,
and every domain family is located once. `X.rs` plus `X/` pairs, generic file
names, parent-name duplicates, and one-file subdirectories are removed.

The public module paths, ABI, diagnostics, snapshots, `actus --version`, and
CLI output remain unchanged. Private source tests move with their module;
integration tests remain under `tests/`.

## Evidence gates

Every PR uses two structural commits where applicable:

1. a `git mv`-only commit whose `git diff -M` reports 100% renames;
2. a module-path/facade commit containing only `mod`, `use`, `pub use`, path,
   and test-module changes.

Before and after each PR, compare `cargo test --all-features -- --list`,
snapshots, `actus --version`, CLI output, `cargo fmt --all -- --check`, and
`git diff --check`. Function bodies are compared byte-for-byte for moves.

## PR sequence

1. Parser, lexer, and AST
2. Target and configuration
3. Semantic
4. Shared lowering, body, and backend contract
5. Cranelift
6. LLVM
7. Runtime, modules, and registry
8. CLI and LSP
9. Formatter, diagnostics, documentation, and conformance
10. Remove the final architecture allowlist

Each module PR performs its complete map, facade conversion, domain grouping,
and name corrections once. The architecture gate is introduced in PR 0 and
its exact allowlist shrinks after every PR.

## 1. Parser, lexer, and AST

### Final tree

```text
parser/
  mod.rs
  case.rs conditional.rs contracts.rs cursor.rs declarations.rs enums.rs
  expressions.rs generics.rs grammar.rs packs.rs roles.rs serialization.rs
  spans.rs state.rs statements.rs structs.rs
  metadata/{mod,bounded,capability,declaration,deterministic,
    dispatch,duplicates,effect,fault,limitless,must_use,realtime,repr,
    stack_limit,syntax,target,test}.rs
lexer/
  mod.rs scan.rs token.rs scanner/{mod,state,cursor,identifiers,literals,tokens}.rs
ast/
  mod.rs abi.rs expr.rs intrinsic.rs pattern.rs registry.rs stmt.rs types.rs
  decl/{mod,builtins,callable,data,top_level,types}.rs
```

| Old | New | Type |
| --- | --- | --- |
| `parser/parser.rs` | `parser/state.rs` | rename |
| `parser/mod.rs` `parse` | `parser/grammar.rs` | extract |
| `parser/metadata/*` | `parser/metadata/*` | unchanged |
| `lexer/mod.rs` `scan` | `lexer/scan.rs` | extract |
| `lexer/scanner/mod.rs` `LexErrorKind`, `Scanner` | `lexer/scanner/state.rs` | extract |
| `lexer/scanner/*` | `lexer/scanner/*` | unchanged |
| `ast/decl/*` | `ast/decl/*` | unchanged |
| `ast/{abi,expr,intrinsic,pattern,registry,stmt,types}.rs` | same paths | unchanged |

`Parser` is the primary type in `parser/state.rs`; the parser and lexer roots
now expose the unchanged public paths through `parser/mod.rs` and
`lexer/mod.rs`.

The remaining `#[path]` declarations are explicit temporary exceptions for
later PRs. `src/codegen/body/case.rs` and
`src/codegen/body/case_value.rs` are removed in PR 4; their case lowering
children become ordinary module children. `src/runtime/fs/walk.rs` is removed
in PR 7 when its tests move to `runtime/fs/tests.rs`.
`src/conformance/source_limits.rs` extracts `diagnostics.rs`, `policy.rs`, and
`scan.rs`, while `src/ffi/abi.rs` extracts `abi_errors.rs` and `aggregate.rs`;
both are PR 9 work. `src/diagnostics/renderer/codes/mod.rs` is removed in PR 9
when `pack_codes.rs` becomes the codes facade's regular child.

### PR 1 evidence

- [x] Parser, lexer, and AST layout work is implemented in commits
  [`5866cdc1`](https://github.com/actus-lang/actus/commit/5866cdc1) and
  [`17cdfd44`](https://github.com/actus-lang/actus/commit/17cdfd44); AST files
  remain unchanged as required by this map.
- [x] Architecture allowlist entries for the parser and lexer facades were
  removed in [`ebdd03af`](https://github.com/actus-lang/actus/commit/ebdd03af);
  the no-new-`#[path]` guard and its six exact temporary exceptions were added
  in [`aaebe75b`](https://github.com/actus-lang/actus/commit/aaebe75b).
- [x] `cargo test --all-features -- --list` reports 1,988 tests before and
  after; after excluding Cargo timing lines, the test-name diff is empty.
- [x] `target/debug/actus --version` and
  `target/debug/actus check examples/hello.act --strict` are byte-identical
  before and after the layout change.
- [x] Snapshot files are unchanged; formatting and the architecture test pass.
- [x] The body audit used `git diff 4ff355ca -- src/parser src/lexer` with
  `grep '^[+-]'` filtering. After excluding pure move/extract hunks, the
  remaining semantic changes are limited to `pub(super)` visibility and
  module/import lines; the filtered body-change output is empty.

## 2. Target and configuration

### Final tree

```text
target/{mod,model,contracts,resolution,arm,counter,provider}.rs
configuration/{mod,accessors,checksum,dependencies,loading,lockfile,manifest,
  resolution,strict,types,version}.rs
```

| Old | New | Type |
| --- | --- | --- |
| `target.rs` model items | `target/model.rs` | extract |
| `target.rs` linker/ABI contracts | `target/contracts.rs` | extract |
| `target.rs` target parsing/resolution | `target/resolution.rs` | extract |
| `target.rs` | `target/mod.rs` | pair→facade |
| `target/{arm,counter,provider}.rs` | same paths | unchanged |
| `configuration/*.rs` | same paths | unchanged |
| `configuration/mod.rs` | same path, facade-only | extract |

The target split follows the accepted grouping `target → model/contracts/
resolution`; `arm`, `counter`, and `provider` remain separate domains.

### PR 2 evidence

- [x] `target.rs` was moved as a 100% rename to `target/model.rs` in
  [`025d897f`](https://github.com/actus-lang/actus/commit/025d897f).
- [x] The target root was split into the `model`, `contracts`, and
  `resolution` modules in [`7c00c1d3`](https://github.com/actus-lang/actus/commit/7c00c1d3);
  the public `actus::target` exports remain in `target/mod.rs`.
- [x] The target architecture allowlist entry was removed in
  [`11a174f4`](https://github.com/actus-lang/actus/commit/11a174f4). The
  temporary `target/mod.rs` probe was rejected by the architecture test with
  `non-facade item in new module facade src/target/mod.rs: ["fn probe() {}"]`.
- [x] `cargo fmt --all -- --check` and `cargo check --all-features` pass after
  the split.
- [x] The complete PR validation passed: `cargo test --all-features --
  --list` reports 1,988 entries before and after; the only four name changes
  are `target::tests::*` to `target::resolution::tests::*`. Snapshot diff is
  empty, and formatting, clippy, and all-target tests pass.
- [x] The filtered source diff from base contains only target extraction and
  module/import lines plus the internal `pub(super)` visibility required by
  sibling target modules; no function body behavior changed. A comparison of
  deleted and added non-`use` lines, excluding visibility modifiers, produced
  an empty difference list:

  ```text
  remaining differences:
  ```

## 3. Semantic

### Final tree

```text
semantic/
  mod.rs target_filter.rs stack_limit.rs
  generics/{mod,types,cache,calls,errors,substitution}.rs
  enums/{mod,model,registry,constructors/{mod,type_parsing,validation,variants}}.rs
  patterns/{mod,bindings,coverage,validation,moves,diagnostics}.rs
  calls/{mod,effects,ownership,parsing,paths,signatures,arguments,shapes}.rs
  analyzer/{mod,analysis,expressions/{mod,analysis,conditionals,context,rules},
    statements/{mod,control},bounded/{mod,recursion,graph},constants,
    declarations,for_loops,lifecycle,operator_validation,state,validation,verb}.rs
```

| Old | New | Type |
| --- | --- | --- |
| `semantic/mod.rs` target filtering | `semantic/target_filter.rs` | extract |
| `semantic/mod.rs` stack validation | `semantic/stack_limit.rs` | extract |
| `generics.rs` | `generics/types.rs` | group |
| `generic_cache.rs` | `generics/cache.rs` | group |
| `generic_calls.rs` | `generics/calls.rs` | group |
| `generic_errors.rs` | `generics/errors.rs` | group |
| `type_substitution.rs` | `generics/substitution.rs` | group |
| `enums.rs` | `enums/model.rs` | group |
| `enum_registry.rs` | `enums/registry.rs` | group |
| `enum_constructors/*` | `enums/constructors/*` | group |
| `pattern_moves.rs` | `patterns/moves.rs` | group |
| `pattern_support.rs` | `patterns/diagnostics.rs` | rename |
| `call_arguments.rs` | `calls/arguments.rs` | group |
| `argument_shapes.rs` | `calls/shapes.rs` | group |
| `enum_constructors/support.rs` | `enums/constructors/variants.rs` | rename |
| `analyzer/mod.rs` `analyze` | `analyzer/analysis.rs` | extract |
| `analyzer/expression_*.rs` | `analyzer/expressions/*.rs` | group |
| `analyzer/statement_*.rs` | `analyzer/statements/*.rs` | group |
| `analyzer/{bounded,bounded_graph}.rs` | `analyzer/bounded/{recursion,graph}.rs` | group |
| remaining semantic files | same paths | unchanged |

Imports prove the groups: generic calls use the calls and argument-shape
modules (`src/semantic/generics/calls.rs:1-8`), enum lowering calls
`enum_constructors::generic_receiver_type` (`src/semantic/enums/model.rs:33,156`),
pattern validation uses the diagnostics module (`src/semantic/patterns/validation.rs:7`),
and analyzer state stores both call signatures and generic cache
(`src/semantic/analyzer/state.rs:26,64`).

### PR 3 evidence

The semantic refactor preserves the base visibility boundary. The test-list
comparison was run from an archive of `3527c493`, in a temporary directory
separate from the working tree. The archived checkout used the already
installed bundled LLVM prefix only to satisfy `llvm-sys`; no source or build
output was taken from the working tree.

```text
$ repo=$PWD
$ tmp=$(mktemp -d)
$ git archive 3527c493 | tar -x -C "$tmp"
$ (cd "$tmp" && LLVM_SYS_231_PREFIX="$repo/.actus-toolchain/llvm-23.1.2" cargo test --all-features -- --list)
$ cargo test --all-features -- --list
base test names: 1988
working-tree test names: 1988
```

The complete name-only diff was:

```diff
-semantic::generic_cache::tests::deduplicates_by_structural_identity_within_a_toolchain
-semantic::generic_cache::tests::different_toolchain_caches_are_independent
+semantic::generics::cache::tests::deduplicates_by_structural_identity_within_a_toolchain
+semantic::generics::cache::tests::different_toolchain_caches_are_independent
+semantic::generics::substitution::tests::exposes_const_arguments_for_expression_specialization
+semantic::generics::substitution::tests::preserves_reference_role_when_substituting_a_generic_root
+semantic::generics::substitution::tests::substitutes_nested_type_applications
-semantic::type_substitution::tests::exposes_const_arguments_for_expression_specialization
-semantic::type_substitution::tests::preserves_reference_role_when_substituting_a_generic_root
-semantic::type_substitution::tests::substitutes_nested_type_applications
```

Thus the count is unchanged and the only changes are the two requested module
path moves. The checker command and complete output are:

```text
$ python3 scripts/check_layout_bodies.py 1d8e3a03^ src/semantic
ambiguous: src/semantic/enums/constructors/variants.rs:64: fn argument_span
differences:
visibility changes:
visibility expansions:
```

The ambiguity is intentional: `argument_span` is the name of two different
functions, so the checker reports both-name ambiguity rather than treating
them as one function.

The base and working trees both contain 168 `pub(crate)` declarations:

```text
$ git grep -h 'pub(crate)' 1d8e3a03^ -- src/semantic | wc -l
168
$ rg -n 'pub\(crate\)' src/semantic | wc -l
168
```

The checker was also run with a deliberately widened item in
`src/semantic/analyzer/verb.rs`, a widened `semantic/mod.rs` module, and a
widened untracked facade; each run failed with `visibility expansions`, and
all three changes were reverted. `constructors/support.rs` is now named
`constructors/variants.rs` because it resolves enum variants and argument
spans.

The pre-correction analyzer visibility proof widened the requested 18 items
and exited 1; after restoring their `crate::semantic::analyzer` boundary, the
clean checker output above has zero expansions. `cargo test --all-targets
--all-features -- --test-threads=1` and `cargo clippy --all-targets
--all-features -- -D warnings` both passed.

## 4. Shared lowering, body, and contract

### Final tree

```text
codegen/mod.rs                                      # facade only
codegen/abi/aggregate.rs                            # LARGE_AGGREGATE_THRESHOLD
codegen/body/{mod,lowering,contract,control_flow,if_flow,provider,
  statements,try_value,value}.rs
codegen/body/case/{mod,branches,carried,context,returns,nested}.rs
codegen/body/case/value/{mod,values,owner_flow}.rs
codegen/body/loops/{mod,lowering,loop_statement,carried,range_setup,state}.rs
codegen/shared_lowering/{mod,aggregate_assignment,constructors,declarations,
  enum_payload,errors,functions,orchestration,performance,returns,runtime,
  runtime_externals,scalar_types,tests}.rs
codegen/shared_lowering/expression/{mod,lowering,arena,arithmetic,array_ops,
  buffer,case,cast,cleanup,compare,dispatch,drop,layout,literals,packed,
  parameter_state,provider,state}.rs
codegen/shared_lowering/expression/aggregate/{mod,values,arrays,compound,
  fields,metadata,structs}.rs
codegen/shared_lowering/expression/calls/{mod,lowering,arguments,constructors,
  loan,print,resolution,runtime}.rs
codegen/shared_lowering/layout/{mod,registry,cleanup,generic_struct,options,
  provider}.rs
codegen/contract/{mod,types,cleanup,dominance,errors,flow,instruction,
  semantic_cleanup,validation}.rs
codegen/result_constructors/{mod,normalize,spans}.rs
codegen/symbols/{mod,model,bindings}.rs
```

| Old | New | Type |
| --- | --- | --- |
| `body.rs` | `body/mod.rs` | pair→facade |
| `body.rs` `BodyLowering` | `body/lowering.rs` | extract |
| `body_contract.rs` | `body/contract.rs` | move |
| `body/case.rs` | `body/case/mod.rs` | pair→facade |
| `body/case.rs` case lowering items | `body/case/branches.rs` | extract |
| `body/case_return.rs` | `body/case/returns.rs` | rename |
| `body/nested_case.rs` | `body/case/nested.rs` | rename |
| `body/case_value.rs` | `body/case/value/mod.rs` | pair→facade |
| `body/case_value.rs` case value items | `body/case/value/values.rs` | extract |
| `body/case_value/owner_flow.rs` | `body/case/value/owner_flow.rs` | move; remove `#[path]` |
| `body/loops.rs` | `body/loops/mod.rs` | pair→facade |
| `body/loops.rs` loop lowering items | `body/loops/lowering.rs` | extract |
| `body/loop_flow.rs` | `body/loops/loop_statement.rs` | rename |
| `body/loop_support.rs` | `body/loops/carried.rs` | rename |
| `shared_lowering/expression.rs` | `expression/mod.rs` | pair→facade |
| `shared_lowering/expression.rs` expression lowering items | `expression/lowering.rs` | extract |
| `expression/aggregate.rs` | `expression/aggregate/mod.rs` | pair→facade |
| `expression/aggregate.rs` aggregate lowering items | `expression/aggregate/values.rs` | extract |
| `expression/calls.rs` | `expression/calls/mod.rs` | pair→facade |
| `expression/calls.rs` call lowering items | `expression/calls/lowering.rs` | extract |
| `shared_lowering/layout.rs` | `layout/mod.rs` | pair→facade |
| `shared_lowering/layout.rs` `LayoutRegistry` and layout helpers | `layout/registry.rs` | extract |
| `shared_lowering/support.rs` | `scalar_types.rs` | extract scalar type functions |
| `shared_lowering/support.rs` | `errors.rs` | extract expression span/kind and unsupported-operation errors in COMMIT 4 |
| `contract/mod.rs` backend types | `contract/types.rs` | extract |
| `result_constructors/mod.rs` | `result_constructors/normalize.rs` | extract |
| `symbols/mod.rs` | `symbols/model.rs` | extract |
| `codegen/mod.rs` `LARGE_AGGREGATE_THRESHOLD` | `abi/aggregate.rs` | extract in COMMIT 4 |
| existing contract children | same paths | unchanged |

`body.rs:10-20` imports the case and loop families; aggregate and call
modules are imported through the expression facades. `LayoutRegistry` is the
primary type in `shared_lowering/layout.rs`. `constants.rs` already contains
the inline-constants pass, so the aggregate threshold belongs with the ABI
aggregate contract. `loop_flow.rs` performs loop lowering and `loop_support.rs`
contains loop-carried analysis; `integer_values.rs` would contain only two
functions, so those functions remain in `scalar_types.rs`.

### PR 4 evidence

PR 4 split the shared codegen facades and implementations. The `expression.rs`
implementation was additionally split into `expression/lowering.rs` and
`expression/model.rs`; the struct and accessor items moved to `model.rs` to
keep both files below the 500-line source limit. The `include!` paths in
`layout/` and `expression/` were updated. Converting those included files into
ordinary modules is deferred.

The checker command and complete output were:

```text
compared items: 2127 matched
path-only changes (prefix rewrites, targets verified):
  (bare) -> super  x1
  nested_case -> super::case::nested  x1
  super -> super::super  x8
  super -> super::super::super  x2
  super -> super::arena  x1
  super -> super::super::arena  x1
  super -> super::case  x1
  super -> super::super  x1
  super -> super::constants  x1
  super -> super::super::constants  x1
  super -> super::constructors  x2
  super -> super::super::constructors  x2
  super -> super::layout  x1
  super -> super::super::layout  x1
  super -> super::loops  x2
  super -> super::model  x3
  super -> super::super::model  x3
  super -> super::serialization  x4
  super -> super::super::serialization  x4
  super::super -> (bare)  x1
  super::super -> super::super::super  x25
  super::super::abi -> super::super::super::abi  x10
  super::super::constructors -> super::super::super::constructors  x1
  super::super::super::body_contract -> super::super::contract  x2
  super::super::super::support -> super::super::super::errors  x22
  super::super::super::support -> super::super::super::scalar_types  x5
  super::super::support -> (bare)  x4
  super::support -> super::super::errors  x2
  super::support -> super::super::scalar_types  x1
result: OK
```

`cargo test --all-features -- --list` produced 1,988 entries on the working
tree. The same command against the `ca0f2994` archive could not complete
because the local linker terminated with `Bus error`, so no base name diff is
claimed here. `cargo clippy --all-targets --all-features -- -D warnings`
passed. The all-target test run reached 172 passed tests and then stopped on
11 temporary-file tests with `Disk quota exceeded`.

## 5. Cranelift

### Final tree and map

```text
cranelift/
  mod.rs abi.rs aggregate.rs arenas.rs arrays.rs buffer_index.rs cleanup.rs
  control_flow.rs dependencies.rs enum_layout.rs enums/ generic/ ir_audit.rs
  isa.rs literals.rs native.rs object.rs runtime.rs structs/ types.rs
  numeric/{mod,float.rs,integer.rs}
  dispatch/{mod,dynamic.rs,vtable.rs}
  emission/{mod,output.rs,verbs.rs,declarations.rs}
  calls/{mod,arguments.rs,context.rs,lower.rs,method.rs,returns.rs}
  case/{mod,lowering.rs,branches.rs,context.rs,matching.rs,payload.rs,types.rs}
  contract/{mod,emission.rs,aggregate.rs,arena.rs,arrays.rs,cleanup.rs,
    control_flow.rs,declarations.rs,dispatch.rs,enums.rs,functions.rs,packed.rs,
    returns.rs,scalar.rs}
  expressions/{mod,lowering.rs,buffer.rs,cast.rs,construct.rs,if_lowering.rs,
    initializer_types.rs,literals.rs,operations.rs,short_circuit.rs,try_lowering.rs}
  function_definition/{mod,definition.rs,flow.rs,parameters.rs}
  layout/{mod,registry.rs,arrays.rs,packs.rs,structs.rs}
  lowering/{mod,flow.rs,body.rs,loops.rs,scopes.rs,
    statements/{mod,lowering.rs,compound.rs,control_flow.rs,dispatch.rs,owners.rs,
      structured_control.rs,try_lowering.rs}}
  performance/{mod,registry.rs,emit.rs}
```

| Old | New | Type |
| --- | --- | --- |
| `float_ops.rs` | `numeric/float.rs` | move |
| `integer_ops.rs` | `numeric/integer.rs` | move |
| `dynamic_call.rs` | `dispatch/dynamic.rs` | move |
| `vtable.rs` | `dispatch/vtable.rs` | move |
| `emission.rs` | `emission/output.rs` | move |
| `emitter_declarations.rs` | `emission/verbs.rs` | move |
| `declarations.rs` | `emission/declarations.rs` | move |
| `calls.rs` | `calls/mod.rs` | pair→facade |
| `case/mod.rs` | `case/lowering.rs` | move |
| `contract/mod.rs` | `contract/emission.rs` | move |
| `expressions/mod.rs` | `expressions/lowering.rs` | move |
| `function_definition/mod.rs` | `function_definition/definition.rs` | move |
| `layout/mod.rs` | `layout/registry.rs` | move |
| `lowering/mod.rs` | `lowering/flow.rs` | move |
| `lowering/statements/mod.rs` | `lowering/statements/lowering.rs` | move |
| `performance/mod.rs` | `performance/registry.rs` | move |
| `calls/{arguments,context,lower,method,returns}.rs` | same paths | unchanged |
| `case/{branches,context,matching,payload,types}.rs` | same paths | unchanged |
| `contract/{aggregate,arena,arrays,cleanup,control_flow,declarations,dispatch,enums,functions,packed,returns,scalar}.rs` | same paths | unchanged |
| `expressions/{buffer,cast,construct,if_lowering,initializer_types,literals,operations,short_circuit,try_lowering}.rs` | same paths | unchanged |
| `function_definition/{flow,parameters}.rs` | same paths | unchanged |
| `layout/{arrays,packs,structs}.rs` | same paths | unchanged |
| `lowering/{body,loops,scopes}.rs` | same paths | unchanged |
| `lowering/statements/{compound,control_flow,dispatch,owners,structured_control,try_lowering}.rs` | same paths | unchanged |
| `performance/emit.rs` | same path | unchanged |
| remaining `cranelift/*.rs` and subtrees (`enums`, `generic`, `structs`) | same paths | unchanged |

The `emitter_declarations.rs` implementation declares and defines generated
verbs rather than external declarations, so it is named `emission/verbs.rs`.
The former `lowering/mod.rs` owns `Flow` and `LoopTargets`, so it is named
`lowering/flow.rs`. The remaining Cranelift files and subtrees stay in their
current domains; similarly named backend-specific `enum_layout` files are not
merged.

### PR 5 evidence

- [x] The move-only commit contains 16 `R100` renames (`bf440755`). The
  facade and implementation split is recorded in `e35c192d`, and the ten
  Cranelift architecture-allowlist entries were removed in `04419a4f`.
- [x] `python3 scripts/check_layout_bodies.py 09eeca0f src/codegen`:

  ```text
  compared items: 2127 matched
  path-only changes (prefix rewrites, targets verified):
    branches -> super::branches  x1
    super -> super::super::numeric  x5
    super::arrays -> super::super::arrays  x1
    super::buffer_index -> super::super::buffer_index  x1
    super::contract -> super::super::contract  x1
    super::declarations -> super::super::emission  x1
    super::dependencies -> super::super::dependencies  x1
    super::generic -> super::super::generic  x1
    super::ir_audit -> super::super::ir_audit  x1
    super::literals -> super::super::literals  x1
    super::loops -> super::super::loops  x1
    super::lowering -> super::super::lowering  x1
    super::performance -> super::super::performance  x6
    super::super::case -> super::super::super::case  x1
    super::super::expressions -> super::super::super::expressions  x1
    super::super::float_ops -> super::super::numeric  x1
    super::vtable -> super::dispatch  x1
    super::vtable -> super::super::dispatch  x4
  result: OK
  ```

- [x] The `include!` path changes are confined to the `layout/` and
  `expression/` trees. Converting those `include!` inputs into Rust modules
  is deferred to the relevant later layout PR.
- [ ] The required base-archive test-list comparison was attempted with
  `git archive 09eeca0f | tar -x -C <temp>` and
  `CARGO_BUILD_JOBS=1 CXXFLAGS=-w LLVM_SYS_231_PREFIX=<verified-prefix>
  cargo test --all-features -- --list`. The current branch listed 1,988
  tests; the base archive could not finish linking because the runner
  returned `Disk quota exceeded (os error 122)`, so no base name diff is
  claimed here.
- [x] `cargo fmt --all -- --check`, `CXXFLAGS=-w cargo check --lib
  --all-features`, `CXXFLAGS=-w cargo clippy --all-targets --all-features
  -- -D warnings`, `cargo test --all-targets --all-features`,
  `scripts/check_source_limits.sh`, `scripts/check_architecture.sh`,
  `scripts/check_documentation.sh`, and `scripts/check_secrets.sh` passed.

## 6. LLVM

### Final tree and exact map

```text
llvm/
  mod.rs arena.rs branches.rs context.rs control_flow.rs
  declarations.rs emission.rs enums.rs failures.rs guards.rs identity.rs
  instruction.rs literals.rs memory.rs module.rs object.rs returns.rs values.rs
  verification.rs calls.rs
  scalar/{mod,values,loan,float}.rs
  scalar/integer/{mod,cast,ops}.rs
  aggregate/{mod,storage,arrays,packed/fields}.rs
  cleanup/{mod,emission,bridge/{mod,address}}.rs
  shim/{mod,options}.rs
```

| Old | New | Type |
| --- | --- | --- |
| `llvm/scalar.rs` | `llvm/scalar/values.rs` | group + extract |
| `llvm/scalar_loan.rs` | `llvm/scalar/loan.rs` | rename |
| `llvm/float.rs` | `llvm/scalar/float.rs` | group |
| `llvm/integer/mod.rs` | `llvm/scalar/integer/mod.rs` | move |
| `llvm/integer/cast.rs` | `llvm/scalar/integer/cast.rs` | move |
| `llvm/integer/ops.rs` | `llvm/scalar/integer/ops.rs` | move |
| `llvm/aggregates.rs` | `llvm/aggregate/storage.rs` | group |
| `llvm/arrays.rs` | `llvm/aggregate/arrays.rs` | group |
| `llvm/packed/mod.rs` | `llvm/aggregate/packed/fields.rs` | extract |
| `llvm/calls.rs` | `llvm/calls.rs` | unchanged |
| `llvm/returns.rs` | `llvm/returns.rs` | unchanged |
| `llvm/cleanup.rs` | `llvm/cleanup/mod.rs` | pair→facade |
| `llvm/cleanup/bridge.rs` | `llvm/cleanup/bridge/mod.rs` | pair→facade |
| `llvm/cleanup/bridge/address.rs` | same path | unchanged |
| `llvm/shim.rs` | `llvm/shim/mod.rs` | pair→facade |
| `llvm/shim/target_options.cpp` | same path | unchanged |
| `llvm/{arena,branches,context,control_flow,declarations,emission,enums,failures,guards,identity,instruction,literals,memory,module,object,values,verification}.rs` | same paths | unchanged |

`aggregates.rs`, `arrays.rs`, and `packed/` are grouped because they are all
aggregate representation/lowering and their use sites cross-reference the
same LLVM aggregate operations. `calls.rs` remains unchanged: it is a single
call implementation file and moving it into a one-file `call/` directory is
explicitly disallowed. `returns.rs` also remains `returns.rs`; the keyword
correction applies only to the proposed `body/case/returns.rs` destination.

## 7. Runtime, modules, and registry

### Final tree and map

```text
runtime/region/{mod,descriptor,access,profile,provider,tests}.rs
runtime/region/bridge/{mod,access,pinning,range,volatile,tests}.rs
runtime/fs/{walk.rs,tests.rs}
modules/aggregation/objects/{mod,plan,types,units}.rs
modules/aggregation/plan/{mod,compilation,validation}.rs
modules/aggregation/visibility/{mod,signature}.rs
```

| Old | New | Type |
| --- | --- | --- |
| `runtime/region.rs` | `region/mod.rs` | pair→facade |
| region descriptor/access items | `region/{descriptor,access}.rs` | extract |
| `region_profile.rs` | `region/profile.rs` | group |
| `region_provider.rs` | `region/provider.rs` | group |
| `region_bridge.rs` | `region/bridge/access.rs` | group + extract |
| `region_pinning_bridge.rs` | `region/bridge/pinning.rs` | group |
| `region_range_bridge.rs` | `region/bridge/range.rs` | group |
| `region_volatile_bridge.rs` | `region/bridge/volatile.rs` | group |
| `region_bridge_tests.rs` | `region/bridge/tests.rs` | rename |
| `region/tests.rs` | same path | unchanged |
| `runtime/fs/walk_tests.rs` | `runtime/fs/tests.rs` | rename |
| `aggregation/object_*.rs` | `aggregation/objects/*.rs` | group |
| `aggregation/plan.rs` | `aggregation/plan/compilation.rs` | group + extract |
| `aggregation/plan_validation.rs` | `aggregation/plan/validation.rs` | group |
| `aggregation/{visibility,signature_visibility}.rs` | `aggregation/visibility/*.rs` | group |
| `registry/{cache,index,trust}.rs` | same paths | unchanged |

`runtime/mod.rs` currently re-exports all region bridge APIs
(`src/runtime/mod.rs:110-120`), proving that they belong to the region domain.

## 8. CLI and LSP

| Old | New | Type |
|---|---|---|
| `cli/build/{emission,llvm_emission,cranelift_shared,shared_emission}.rs` | `cli/build/emission/{mod,llvm,cranelift,shared}.rs` | group |
| `lsp/{protocol,protocol_contract,transport,request_dispatch,server,progress,cancellation}.rs` | `lsp/protocol/{messages,session,transport,dispatch,server,progress,cancellation}.rs` | group |
| `lsp/{query_bounds,query_cache,query_handlers,semantic_model,semantic_query,semantic_tokens,source_queries}.rs` | `lsp/queries/{bounds,cache,handlers,model,semantic,semantic_tokens,source}.rs` | group |
| editor feature files | `lsp/editor/*` | group |
| `lsp/{documents,workspace,uri,position,module_scope}.rs` | `lsp/workspace/{documents,state,uri,position,module_scope}.rs` | group/rename |
| remaining CLI files | same paths | unchanged |

`workspace.rs` does not become `workspace/workspace.rs`; its destination is
`workspace/state.rs`, because it owns workspace state. LSP imports confirm the
protocol/query/editor dependency direction (`src/lsp/server.rs:6-16`,
`src/lsp/query_handlers.rs:5-20`).

## 9. Formatter, diagnostics, documentation, conformance

### Final tree

```text
cli/
  mod.rs build/{mod,emission/{mod,llvm,cranelift,shared},loading,options,
    artifacts,module_type_context,object_manifest}.rs test_runner/{mod,runner,
    collection,execution,output,process}.rs
lsp/
  mod.rs protocol/{mod,messages,session,transport,dispatch,server,progress,
    cancellation}.rs queries/{mod,bounds,cache,handlers,model,semantic,
    semantic_tokens,source}.rs editor/{completion,definition,navigation,rename,
    code_lens,signature_help,formatting,hover}.rs workspace/{documents,state,
    uri,position,module_scope}.rs
formatter/declarations/{mod,render,widths}.rs
diagnostics/renderer/{mod,render,json,module,codes/{mod,packing,extended,mapping,messages}}.rs
documentation/{mod,contract,language,model,rules}.rs
conformance/{mod,diagnostics,exceptions,policy,scan,source_limits,tree_policy}.rs
ffi/{mod,abi,abi_errors,aggregate}.rs
```

`formatter/declarations.rs` and `diagnostics/renderer/mod.rs` receive separate
facade and extraction rows; `diagnostics/strict_codes.rs` remains a single
file at its current path.

| Old | New | Type |
| --- | --- | --- |
| `formatter/declarations.rs` | `formatter/declarations/render.rs` | group + extract |
| `formatter/declaration_widths.rs` | `formatter/declarations/widths.rs` | group |
| `diagnostics/renderer/pack_codes.rs` | `renderer/codes/packing.rs` | rename |
| `diagnostics/renderer/codes/*` | same codes subtree | unchanged |
| `diagnostics/strict_codes.rs` | same path | unchanged |
| documentation files | same paths | unchanged |
| conformance files | same paths | unchanged |
| `conformance/source_limits.rs` | `conformance/{diagnostics,policy,scan}.rs` | extract (`#[path]` removal) |
| `ffi/abi.rs` | `ffi/{abi_errors,aggregate}.rs` | extract (`#[path]` removal) |
| `diagnostics/renderer/codes/mod.rs` | `diagnostics/renderer/codes/packing.rs` | extract (`#[path]` removal) |
| `runtime/fs/walk.rs` | `runtime/fs/walk.rs` | unchanged (`#[path]` removal; child moves with `walk_tests.rs`) |

The two body exceptions are tracked with the codegen map: `body/case.rs` and
`body/case_value.rs` each lose their `#[path]` child declaration in PR 4.

`diagnostics/strict_codes.rs` stays in place because it would otherwise create
a one-file directory. Likewise `llvm/calls.rs` stays at the LLVM root.

## Rejected groups

- `enum_layout.rs` files across AST/Cranelift/LLVM are separate backend layers.
- `generic/layout.rs` is generic layout, not Cranelift machine layout.
- `build_graph.rs` and `packaging.rs` each have no second sibling in the same
  domain and therefore remain standalone files.
- LSP workspace state is not merged with editor features.
- `diagnostics/strict_codes.rs` is not moved into a one-file `codes/` folder.

## Pair conversion details

The `pair→facade` label is used only where the directory already exists. In
each such case the implementation row is separate and names the responsibility
file that receives the existing items:

| Old | Facade | Extracted items | Destination |
| --- | --- | --- | --- |
| `body/case.rs` | `body/case/mod.rs` | case lowering functions | `body/case/branches.rs` |
| `body.rs` | `body/mod.rs` | `BodyLowering` facade exports | `body/lowering.rs` |
| `body/loops.rs` | `body/loops/mod.rs` | loop lowering functions | `body/loops/lowering.rs` |
| `body/case_value.rs` | `body/case/value/mod.rs` | case value lowering functions | `body/case/value/values.rs` |
| `shared_lowering/expression.rs` | `expression/mod.rs` | expression lowering methods | `expression/lowering.rs` |
| `expression/aggregate.rs` | `aggregate/mod.rs` | aggregate lowering functions | `aggregate/values.rs` |
| `expression/calls.rs` | `calls/mod.rs` | call lowering functions | `calls/lowering.rs` |
| `shared_lowering/layout.rs` | `layout/mod.rs` | `LayoutRegistry` and layout helpers | `layout/registry.rs` |
| `llvm/cleanup.rs` | `cleanup/mod.rs` | `emit_cleanup` | `cleanup/emission.rs` |
| `llvm/cleanup/bridge.rs` | `bridge/mod.rs` | bridge argument and symbol functions | `bridge/emission.rs` |
| `llvm/shim.rs` | `shim/mod.rs` | `TargetMachineOptions` and wrapper | `shim/options.rs` |
| `aggregation/visibility.rs` | `visibility/mod.rs` | visibility validation functions | `visibility/checks.rs` |
| `runtime/region.rs` | `region/mod.rs` | region descriptors and access facade | `region/descriptor.rs` |
| `target.rs` | `target/mod.rs` | target model items | `target/model.rs` |
| `target.rs` | `target/mod.rs` | linker and ABI contracts | `target/contracts.rs` |
| `target.rs` | `target/mod.rs` | target parsing and resolution | `target/resolution.rs` |

`body/case/returns.rs` is plural because `return` is a Rust keyword;
`llvm/returns.rs` remains the existing non-keyword filename. The same
facade-plus-extract rule applies to every other true pair in the mapping.

### Group and extract details without an existing pair

| Current file | Primary items | Destination | Type |
| --- | --- | --- | --- |
| `cranelift/emission.rs` | emission entry points | `cranelift/emission/output.rs` | group + extract |
| `llvm/scalar.rs` | scalar selection, guards, constants | `llvm/scalar/values.rs` | group + extract |
| `formatter/declarations.rs` | declaration formatting functions | `formatter/declarations/render.rs` | group + extract |
| `runtime/region_bridge.rs` | region bridge operations | `region/bridge/access.rs` | group + extract |
| `body/loop_flow.rs` | loop exit and flow transitions | `body/loops/loop_statement.rs` | group + extract |
| `modules/aggregation/plan.rs` | `ModuleCompilationPlan` and plan builders | `modules/aggregation/plan/compilation.rs` | group + extract |
| `modules/aggregation/plan_validation.rs` | plan validation functions | `modules/aggregation/plan/validation.rs` | group + extract |
| `shared_lowering/support.rs` | scalar type functions | `scalar_types.rs` | group + extract |
| `shared_lowering/support.rs` | expression span/kind and unsupported-operation errors | `errors.rs` | group + extract in COMMIT 4 |

## Exact LLVM map

| Current file | Final path | Type | Reason |
| --- | --- | --- | --- |
| `llvm/aggregates.rs` | `llvm/aggregate/storage.rs` | group | aggregate storage/allocation functions |
| `llvm/arena.rs` | `llvm/arena.rs` | unchanged | arena ownership domain |
| `llvm/arrays.rs` | `llvm/aggregate/arrays.rs` | group | aggregate representation/lowering family |
| `llvm/branches.rs` | `llvm/branches.rs` | unchanged | branch emission domain |
| `llvm/calls.rs` | `llvm/calls.rs` | unchanged | one file; no one-file `call/` directory |
| `llvm/cleanup.rs` | `llvm/cleanup/mod.rs` | pair→facade | existing cleanup directory |
| `llvm/cleanup/bridge.rs` | `llvm/cleanup/bridge/mod.rs` | pair→facade | existing bridge directory |
| `llvm/cleanup/bridge/address.rs` | same path | unchanged | address bridge domain |
| `llvm/context.rs` | `llvm/context.rs` | unchanged | LLVM context ownership |
| `llvm/control_flow.rs` | `llvm/control_flow.rs` | unchanged | control-flow emission |
| `llvm/declarations.rs` | `llvm/declarations.rs` | unchanged | declaration emission |
| `llvm/emission.rs` | `llvm/emission.rs` | unchanged | no sibling directory |
| `llvm/enums.rs` | `llvm/enums.rs` | unchanged | enum emission |
| `llvm/failures.rs` | `llvm/failures.rs` | unchanged | failure values |
| `llvm/float.rs` | `llvm/scalar/float.rs` | group | scalar operations |
| `llvm/guards.rs` | `llvm/guards.rs` | unchanged | guard emission |
| `llvm/identity.rs` | `llvm/identity.rs` | unchanged | toolchain identity |
| `llvm/instruction.rs` | `llvm/instruction.rs` | unchanged | instruction abstraction |
| `llvm/integer/mod.rs` | `llvm/scalar/integer/mod.rs` | group | integer scalar family |
| `llvm/integer/cast.rs` | `llvm/scalar/integer/cast.rs` | group | integer scalar family |
| `llvm/integer/ops.rs` | `llvm/scalar/integer/ops.rs` | group | integer scalar family |
| `llvm/literals.rs` | `llvm/literals.rs` | unchanged | literal emission |
| `llvm/memory.rs` | `llvm/memory.rs` | unchanged | memory operations |
| `llvm/module.rs` | `llvm/module.rs` | unchanged | module construction |
| `llvm/object.rs` | `llvm/object.rs` | unchanged | object emission |
| `llvm/packed/mod.rs` | `llvm/aggregate/packed/fields.rs` | extract | packed field load/store implementation |
| `llvm/returns.rs` | `llvm/returns.rs` | unchanged | single return implementation |
| `llvm/scalar.rs` | `llvm/scalar/values.rs` | extract | no current scalar/ directory; scalar family is grouped by content |
| `llvm/scalar_loan.rs` | `llvm/scalar/loan.rs` | group | scalar loan operations |
| `llvm/shim.rs` | `llvm/shim/mod.rs` | pair→facade | existing C++ shim directory |
| `llvm/shim/target_options.cpp` | same path | unchanged | C++ shim implementation |
| `llvm/values.rs` | `llvm/values.rs` | unchanged | LLVM value wrapper |
| `llvm/verification.rs` | `llvm/verification.rs` | unchanged | IR verification |

## Current allowlist and planned PR

The architecture test has six exact-path allowlists. Their current sizes are:

| List | Count | Current entries move in |
| --- | ---: | --- |
| non-facade `mod.rs` | 32 | PRs 1, 3, 4, 5, 6, 7, 8, 9 |
| X.rs + X/ pairs | 15 | PRs 2, 4, 5, 6, 7 |
| one-file module directories | 7 | PRs 4, 5, 7 |
| parent-name duplicates | 0 | none |
| generic/forbidden filenames | 2 | PRs 3 and 4 |
| `#[path]` module redirections | 6 | PRs 4, 7, and 9 |

### Allowlist entry → PR

| Entry | PR |
| --- | ---: |
| `src/cli/mod.rs` | 8 |
| `src/cli/test_runner/mod.rs` | 8 |
| `src/codegen/mod.rs` | 4 |
| `src/codegen/contract/mod.rs` | 4 |
| `src/codegen/cranelift/case/mod.rs` | 5 |
| `src/codegen/cranelift/contract/mod.rs` | 5 |
| `src/codegen/cranelift/expressions/mod.rs` | 5 |
| `src/codegen/cranelift/function_definition/mod.rs` | 5 |
| `src/codegen/cranelift/layout/mod.rs` | 5 |
| `src/codegen/cranelift/lowering/statements/mod.rs` | 5 |
| `src/codegen/cranelift/lowering/mod.rs` | 5 |
| `src/codegen/cranelift/performance/mod.rs` | 5 |
| `src/codegen/llvm/packed/mod.rs` | 6 |
| `src/codegen/result_constructors/mod.rs` | 4 |
| `src/codegen/symbols/mod.rs` | 4 |
| `src/diagnostics/renderer/mod.rs` | 9 |
| `src/formatter/mod.rs` | 9 |
| `src/lexer/mod.rs` | 1 |
| `src/lexer/scanner/mod.rs` | 1 |
| `src/lsp/hover/mod.rs` | 8 |
| `src/parser/mod.rs` | 1 |
| `src/registry/mod.rs` | 7 |
| `src/runtime/mod.rs` | 7 |
| `src/runtime/fs/mod.rs` | 7 |
| `src/runtime/path/mod.rs` | 7 |
| `src/modules/aggregation/visibility.rs` | 7 |
| `src/codegen/body.rs` | 4 |
| `src/codegen/body/case.rs` | 4 |
| `src/codegen/body/case_value.rs` | 4 |
| `src/codegen/body/loops.rs` | 4 |
| `src/codegen/cranelift/calls.rs` | 5 |
| `src/codegen/llvm/cleanup.rs` | 6 |
| `src/codegen/llvm/cleanup/bridge.rs` | 6 |
| `src/codegen/llvm/shim.rs` | 6 |
| `src/codegen/shared_lowering/expression.rs` | 4 |
| `src/codegen/shared_lowering/expression/aggregate.rs` | 4 |
| `src/codegen/shared_lowering/expression/calls.rs` | 4 |
| `src/codegen/shared_lowering/layout.rs` | 4 |
| `src/runtime/region.rs` | 7 |
| `src/target.rs` | 2 |
| `src/codegen/cranelift/performance` | 5 |
| `src/codegen/llvm/packed` | 6 |
| `src/codegen/result_constructors` | 4 |
| `src/codegen/symbols` | 4 |
| `src/codegen/shared_lowering/support.rs` | 4 |
| `src/codegen/body/case.rs` | 4 |
| `src/codegen/body/case_value.rs` | 4 |
| `src/runtime/fs/walk.rs` | 7 |
| `src/conformance/source_limits.rs` | 9 |
| `src/diagnostics/renderer/codes/mod.rs` | 9 |
| `src/ffi/abi.rs` | 9 |

The full exact paths are the constants in
`tests/architecture_conformance.rs`; this table assigns every entry to the
PR that removes it.

### Destination uniqueness audit

The following command extracts old→new pairs from every map table, expands
brace lists, normalizes both sides to `src/` paths using the section context,
and reports only destinations fed by two different old files. Repeating one
old file in several tables is allowed; one old file splitting into several
destinations is also allowed. Its output is intentionally empty:

```sh
python3 - docs/roadmap/phase-52-source-layout.md <<'PY' | sort | uniq -d
import re, sys
from collections import defaultdict
from pathlib import Path

def expand(value):
    match = re.search(r"\{([^{}]+)\}", value)
    if not match:
        return [value]
    prefix, suffix = value[:match.start()], value[match.end():]
    return [item for part in match.group(1).split(',')
            for item in expand(prefix + part + suffix)]

def normalize(path, section):
    path = path.strip()
    if path.startswith('src/'):
        return path
    if path == 'body.rs':
        return 'src/codegen/shared_lowering/body.rs'
    if path.startswith(('body/', 'shared_lowering/', 'contract/')):
        return 'src/codegen/' + path
    if path == 'body.rs' and section == 'shared':
        return 'src/codegen/shared_lowering/' + path
    if path.startswith(('cli/', 'lsp/')):
        return 'src/' + path
    if path.startswith('cranelift/'):
        return 'src/codegen/' + path
    if path.startswith('llvm/'):
        return 'src/codegen/' + path
    if path.startswith('aggregation/'):
        return 'src/modules/' + path
    if path.startswith('runtime/'):
        return 'src/' + path
    if path.startswith('region/'):
        return ('src/runtime/' if section == 'runtime' else 'src/') + path
    roots = {
        'parser': 'src/parser/', 'target': 'src/target/',
        'semantic': 'src/semantic/', 'shared': 'src/codegen/shared_lowering/',
        'cranelift': 'src/codegen/cranelift/', 'llvm': 'src/codegen/llvm/',
        'runtime': 'src/runtime/', 'cli': 'src/cli/',
        'formatter': 'src/', 'diagnostics': 'src/', 'lsp': 'src/',
    }
    if section == 'shared' and path.startswith('expression/'):
        return 'src/codegen/shared_lowering/' + path
    if section == 'shared' and path.startswith('layout/'):
        return 'src/codegen/shared_lowering/' + path
    root = roots.get(section, 'src/')
    return root + path

table = None
section = None
pairs = defaultdict(set)
for line in Path(sys.argv[1]).read_text().splitlines():
    if line.startswith('## '):
        title = line.lower()
        section = ('shared' if 'shared' in title else
                   'cranelift' if 'cranelift' in title else
                   'llvm' if 'llvm' in title else
                   'runtime' if 'runtime' in title or 'modules' in title else
                   'cli' if 'cli' in title or 'lsp' in title else
                   'formatter' if 'formatter' in title or 'diagnostic' in title else
                   'parser' if 'parser' in title else
                   'target' if 'target' in title else
                   'semantic' if 'semantic' in title else None)
    if not line.startswith('|'):
        table = None
        continue
    if not line.startswith('|'):
        table = None
        continue
    cells = [cell.strip() for cell in line.strip('|').split('|')]
    if 'Old' in cells or 'Current file' in cells:
        old = cells.index('Old') if 'Old' in cells else cells.index('Current file')
        new = [i for i, cell in enumerate(cells)
               if cell in {'New', 'Destination', 'Final path'}]
        table = (old, new)
        continue
    if table is None or set(''.join(cells)) <= {'-', ':'}:
        continue
    old_index, new_indexes = table
    old_tokens = [token for token in re.findall(r'`([^`]+)`', cells[old_index])
                  if (token.endswith('.rs') or token.startswith('src/') or
                      '{' in token or '*' in token)]
    new_tokens = [token for index in new_indexes
                  for token in re.findall(r'`([^`]+)`', cells[index])
                  if '*' not in token]
    olds = [normalize(path, section) for token in old_tokens for path in expand(token)]
    news = [normalize(path, section) for token in new_tokens for path in expand(token)]
    if len(olds) == len(news) and len(olds) > 1:
        for old, new in zip(olds, news):
            pairs[new].add(old)
    else:
        for new in news:
            for old in olds:
                pairs[new].add(old)
for new, olds in pairs.items():
    if len(olds) >= 2:
        for _ in olds:
            print(new)
PY
```

```text
(no output)
```

### Non-facade `mod.rs` extraction map

The following map uses the base-tree paths as its source column. Facade
declarations and re-exports remain in the corresponding `mod.rs` files and
are listed separately below; they are not extraction rows.

| Base file | Extracted contents | Destination | PR |
| --- | --- | --- | ---: |
| `src/cli/mod.rs` | `run`, dispatch, command parsing | `cli/dispatch.rs` | 8 |
| `src/cli/test_runner/mod.rs` | `TestOptions`, `test_command` | `cli/test_runner/runner.rs` | 8 |
| `src/codegen/mod.rs` | `LARGE_AGGREGATE_THRESHOLD` | `codegen/abi/aggregate.rs` | 4 |
| `src/codegen/contract/mod.rs` | `Backend*` types | `codegen/contract/types.rs` | 4 |
| `src/codegen/cranelift/case/mod.rs` | case lowering functions | `cranelift/case/lowering.rs` | 5 |
| `src/codegen/cranelift/contract/mod.rs` | `emit_contract_module_object` | `cranelift/contract/emission.rs` | 5 |
| `src/codegen/cranelift/expressions/mod.rs` | expression lowering functions | `cranelift/expressions/lowering.rs` | 5 |
| `src/codegen/cranelift/function_definition/mod.rs` | `define_function` | `cranelift/function_definition/definition.rs` | 5 |
| `src/codegen/cranelift/layout/mod.rs` | `LayoutRegistry`, layout types | `cranelift/layout/registry.rs` | 5 |
| `src/codegen/cranelift/lowering/mod.rs` | `Flow`, `LoopTargets` | `cranelift/lowering/flow.rs` | 5 |
| `src/codegen/cranelift/lowering/statements/mod.rs` | `lower_statements` | `cranelift/lowering/statements/lowering.rs` | 5 |
| `src/codegen/cranelift/performance/mod.rs` | performance registry | `cranelift/performance/registry.rs` | 5 |
| `src/codegen/cranelift/emission.rs` | emission entry points | `cranelift/emission/output.rs` | 5 |
| `src/codegen/cranelift/emitter_declarations.rs` | generated verb declarations and definitions | `cranelift/emission/verbs.rs` | 5 |
| `src/codegen/cranelift/declarations.rs` | Cranelift declaration emission | `cranelift/emission/declarations.rs` | 5 |
| `src/codegen/llvm/packed/mod.rs` | packed field load/store | `llvm/aggregate/packed/fields.rs` | 6 |
| `src/codegen/result_constructors/mod.rs` | `normalize_program` | `codegen/result_constructors/normalize.rs` | 4 |
| `src/codegen/symbols/mod.rs` | `SymbolKind`, `SymbolIdentity`, registry | `codegen/symbols/model.rs` | 4 |
| `src/diagnostics/renderer/mod.rs` | renderer and diagnostic functions | `diagnostics/renderer/render.rs` | 9 |
| `src/formatter/mod.rs` | `Formatter`, program formatting | `formatter/program.rs` | 9 |
| `src/lexer/mod.rs` | `scan` | `lexer/scan.rs` | 1 |
| `src/lexer/scanner/mod.rs` | `LexErrorKind`, `Scanner` | `lexer/scanner/state.rs` | 1 |
| `src/lsp/hover/mod.rs` | `HoverInfo`, hover functions | `lsp/hover/model.rs` and `hover/render.rs` | 8 |
| `src/parser/mod.rs` | `parse` | `parser/grammar.rs` | 1 |
| `src/registry/mod.rs` | `RegistryError`, `LocalRegistry` | `registry/local.rs` | 7 |
| `src/runtime/mod.rs` | `runtime_archive_path` | `runtime/archive.rs` | 7 |
| `src/runtime/fs/mod.rs` | filesystem mode constants and path helpers | `runtime/fs/mode.rs` | 7 |
| `src/runtime/path/mod.rs` | path model and separators | `runtime/path/model.rs` | 7 |
| `src/semantic/analyzer/mod.rs` | `analyze`, bounded and stack-limit validators | `src/semantic/analyzer/analysis.rs` | 3 |
| `src/semantic/calls/mod.rs` | call validation `Analyzer` impls | `src/semantic/calls/validation.rs` | 3 |
| `src/semantic/enum_constructors/mod.rs` | constructor validation `Analyzer` impls | `src/semantic/enums/constructors/validation.rs` | 3 |
| `src/semantic/enum_constructors/support.rs` | variant lookup, argument diagnostics and type parsing helpers | `src/semantic/enums/constructors/variants.rs`, `src/semantic/enums/constructors/type_parsing.rs` | 3 |
| `src/semantic/errors/mod.rs` | `SemanticError` model | `src/semantic/errors/model.rs` | 3 |
| `src/semantic/mod.rs` | target filtering functions | `src/semantic/target_filter.rs` | 3 |
| `src/semantic/mod.rs` | stack-limit validation | `src/semantic/stack_limit.rs` | 3 |
| `src/semantic/ownership/mod.rs` | ownership validation `Analyzer` impls | `src/semantic/ownership/validation.rs` | 3 |
| `src/semantic/ownership/mod.rs` | ownership return cleanup impls | `src/semantic/ownership/returns.rs` | 3 |
| `src/semantic/patterns/mod.rs` | pattern coverage impls | `src/semantic/patterns/coverage.rs` | 3 |

The facade-only files are `src/semantic/mod.rs`,
`src/semantic/analyzer/mod.rs`, `src/semantic/calls/mod.rs`,
`src/semantic/enums/constructors/mod.rs`, `src/semantic/errors/mod.rs`,
`src/semantic/ownership/mod.rs`, and `src/semantic/patterns/mod.rs`; their
remaining declarations and re-exports stay in place.

## Architecture protection

`tests/architecture_conformance.rs` records the current violations by exact
path and rejects any new violation. The allowlist is removed progressively as
PRs 1–9 land.
