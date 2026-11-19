# Object and executable output

Object emission produces one relocatable native object for the whole program,
without final platform linking (ADR-0113). Every imported unit is compiled into
that object, and its only global definition is the configured entry point;
every other function is local. Its undefined symbols are what the runtime, a
provider, or the host must supply. Executable emission links that object with
the runtime archive, libraries, and linker settings.

Use object output to inspect native symbols, layout, and freestanding imports. Use executable output to observe a linked program on the
selected target.

A successful object build does not imply a successful executable link, and a
successful link does not establish hardware behavior.
