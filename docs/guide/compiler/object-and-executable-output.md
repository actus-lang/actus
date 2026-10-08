# Object and executable output

Object emission produces a relocatable native object without final platform
linking. Executable emission adds the configured entry point, module objects,
runtime archive, libraries, and linker settings.

Use object output to inspect module boundaries, native symbols, layout, and
freestanding imports. Use executable output to observe a linked program on the
selected target.

A successful object build does not imply a successful executable link, and a
successful link does not establish hardware behavior.
