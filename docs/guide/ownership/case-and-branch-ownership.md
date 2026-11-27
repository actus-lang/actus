# Case and branch ownership

Ownership is checked independently on every `if`, `case`, and loop path. Paths
join only when their resulting ownership states are compatible.

A diverging branch such as `return`, `break`, or `continue` does not contribute
a value to an expression branch join, but its cleanup still runs for values
that remain live on that path.

A loop body is one path that runs again. A value that is declared before the loop
and moved inside it must therefore leave the loop on that path, with `break` or
`return`; otherwise the next iteration repeats the move and the compiler reports
`E1005` at the move. A `continue` after the move counts as the next iteration.

A `case` on a subject without `dat` takes a read-only view of its owner, and
the view lasts for the case and nothing longer. Inside the case the owner is
frozen, so an arm cannot move or drop it, but an arm may view another field of
it with `abs`. When the case ends, the views of the subject and of its arms end
with it, the owner is mutable again, and it can be moved or dropped: an arm that
read another field leaves the owner as free as an arm that read nothing.

```actus
struct Config {
    erg first: Buffer,
    erg second: Option[Buffer],
}

verb inspect_first(abs held: Buffer) {
}

verb main() {
    erg config: Config = Config { first: Buffer[4], second: Option[Buffer].None, };
    case config.second {
        Option.None => {},
        Option.Some(inner) => {
            inspect_first(held: abs config.first);
        },
    };
    drop(config);
}
```

Enum payload moves are branch-local. An `abs` case subject remains a read-only
owner after the case; a `dat` subject may be consumed by the match. A branch
cannot use a payload after moving it, and another branch cannot rely on a move
that happened only in a different branch.

When a branch error appears, compare each branch's final owner, moved, borrowed,
and dropped states. The fix belongs at the first operation that creates the
incompatible state.
