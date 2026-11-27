# Phase 57: Ownership Expressiveness and a Formal Model

## Status: Planned

The ownership roles (`erg`, `abs`, `ins`, `dat`) are visible at every
declaration and every call site, which makes Actus code easier to read than
code whose moves and borrows only the types reveal. The checker that enforces
them is stricter than it needs to be, though, and it has no formal model that
the implementation can be measured against:

- **Liveness is lexical.** A view is live until the end of the block that
  holds it (ADR-0016, and E1011 and E1079 since the arena fix), even when it
  is never read again. A program that stops using a view and then mutates,
  moves or drops its owner in the same block is rejected although it is safe.
- **Branches do not keep their own state.** A view declared inside an `if`
  or `case` branch is rejected by the branch join with E1061 (bug #78), so a
  view cannot be written in a conditional block at all.
- **An `abs` return has one origin.** ADR-0021 requires a returned view to
  come from exactly one `abs` parameter (E1018, E1066). A verb that returns a
  view into whichever of two inputs it selects cannot be written.
- **Soundness rests on tests only.** The bugs found while writing the
  diagnostic catalog (#41, #48, #79, #84, #85) were programs the checker
  accepted although the rules forbid them. Nothing but individual tests says
  what the rules are, so a hole is found only when someone happens to write
  the program that exposes it.

## Goals

1. **Liveness by last use.** A view ends at its last use instead of at the
   end of its block, computed per control-flow path, as non-lexical lifetimes
   do in Rust. The existing errors keep their codes; programs that are safe
   under the new rule are accepted. An ADR replaces the lexical rule of
   ADR-0016 and states how a view that escapes into a struct, a loop or a
   returned value is handled. Because the end of a view is then computed
   instead of written, the ADR keeps it visible: the roles stay written at
   every declaration and call; every E1011 and E1079 names the last use that
   keeps the view live; and the language server shows where each view ends.
2. **Branch-aware state.** Each branch of an `if` or `case` is analysed with
   its own ownership and view state, and the states are joined at the end of
   the construct. A view declared and used inside one branch is valid
   (closes #78), and E1061 reports a real disagreement between branches in
   words, naming the binding and the two states.
3. **Views with more than one origin.** A verb can return a view whose origin
   is one of several `abs` parameters, written in the signature so the caller
   sees it, for example `-> abs (left | right) Buffer`. The caller keeps all
   named origins frozen while the result is live. The syntax stays a role,
   not a lifetime parameter: the reader sees which inputs the result may
   borrow, without generic lifetime names. An ADR fixes the syntax, the rules
   at the call site and the interaction with generics and methods.
4. **A formal model of ownership.** The roles, moves, views, freezing, drop,
   arenas and the rules of goals 1 to 3 are written as a small formal model
   in a proof assistant (Lean is the first candidate), with a proof that a
   program the model accepts never reads a moved, dropped or released value
   and never mutates a frozen owner. The model lives in the repository next
   to the ADRs it formalises.
5. **The compiler checked against the model.** A test generator produces
   small programs from the model's rules and compares the model's verdict
   with `actus check` on each, in CI. A disagreement is a bug in one of the
   two, and every disagreement found becomes a regression test.

## Exit criteria

- The rejected-but-safe programs listed in the ADRs of goals 1 to 3 compile,
  and every existing rejection that protects a real hazard still fails with
  its code; each change has semantic and CLI tests seen failing once.
- #78 is closed, and the E1061 message names the binding and the states.
- The formal model covers every ownership ADR in force, its soundness proof
  is checked in CI, and the comparison test of goal 5 runs in CI with no
  known disagreement.
- The ownership guide and the diagnostic catalog describe the new rules, and
  the catalog examples for E1011, E1018, E1061, E1066 and E1079 are updated.
