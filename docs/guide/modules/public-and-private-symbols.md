# Public and private symbols

A declaration is private unless the module facade exposes it. Public symbols
form an API contract and need complete documentation, stable types, ownership
roles, and relevant tests.

Keep implementation helpers private. Do not export a raw runtime bridge,
internal checksum helper, or private storage detail when a typed public verb can
provide the intended operation.

A public type may expose a private implementation only when the language's
module and type rules permit the complete public signature. Otherwise the
compiler reports the visibility boundary.
