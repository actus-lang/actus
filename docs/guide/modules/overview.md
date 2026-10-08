# Modules and facades overview

An Actus module groups related declarations and defines which of them other
source files may use. The module's canonical facade is the public gateway.

A module has two separate concerns:

- its internal implementation, where sibling files may share the module scope;
- its external API, which contains only declarations exposed by the facade.

Consumers should import through the public facade. They should not bypass a
parent facade to reach a private child implementation.
