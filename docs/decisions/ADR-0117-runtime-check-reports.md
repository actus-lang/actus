# ADR-0117: Runtime Check Reports

## Status

Accepted.

## Context

Integer overflow (ADR-0115), division and remainder by zero, `MIN / -1`,
out-of-range shift counts, out-of-range casts, fixed-array indexes outside
their capacity and full arenas are checked at run time. A failed check
executes the target's trap instruction. On Linux the process dies with
`SIGILL` and the shell prints only `Illegal instruction`, with exit status
132: nothing names the check or the line that failed. A program's author has
to guess which of several checks fired and where.

Freestanding code, such as firmware, may run without the Actus runtime and
without a stderr to print to. For it a trap is the right failure: it is one
instruction, needs no library, and a debugger or fault handler sees the
address.

## Decision

A hosted executable, built with `--emit exe` and the hosted entry contract,
reports a failed runtime check instead of trapping:

```text
error: integer overflow at src/main.act:3:17
```

The message names the check and the source location of the expression that
failed, as `path:line:column` with a 1-based line and a 1-based character
column. A path under the current directory is shown relative to it.

The checks report these messages:

| Check | Message |
| --- | --- |
| trapping `+`, `-`, `*`, unary `-`, `MIN / -1` | `integer overflow` |
| `/` or `%` by zero | `division by zero` |
| shift count not below the bit width | `shift count out of range` |
| conversion, explicit (`as`) or implicit, to a type that cannot hold the value | `integer value out of range` |
| fixed-array index outside the capacity | `index out of bounds` |
| arena placement past the capacity | `arena capacity exceeded` |

The program first writes out any stdout output it still holds, then prints
the message to stderr and exits with status 101.

The report is the runtime function `actus_runtime_check_failed`, which takes
one null-terminated message and never returns. Both native backends emit, in
place of each trap, a call to it with the message as constant data, followed
by a trap that is never reached.

The backend contract carries the choice in `BackendModule::runtime_checks`:
`Trap` keeps the trap instruction and is the default; `Report` holds the
location of each checking instruction, keyed by its span. The compiler
driver fills it from the source map for hosted executables only. Objects
(`--emit obj`), freestanding builds and test executables keep the trap.

## Consequences

- A failed check in a program run with `actus run` or as a built executable
  names what failed and where. The exit status is 101 instead of a signal.
- Each check site in a hosted executable adds one constant string and one
  cold call; code that passes its checks runs the same comparisons as before.
- Freestanding code and objects are unchanged.
- `actus test` executables still trap; reporting there is a later step.
