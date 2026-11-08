# Compiler and toolchain identity

The command actus --version reports the compiler version, build commit, host
triple, and native backends compiled into the binary.

Use actus --version --json when a tool needs a stable record. The JSON schema
contains the Actus version, commit, LLVM version and verification status,
release tag, archive checksum, source revision, host, and backends.

The repository bootstrap marks the bundled LLVM archive as verified. An
explicit LLVM_SYS_231_PREFIX override is unverified because it does not have
the repository archive identity. Strict LLVM builds reject that state and
direct the developer to the bootstrap script. A Cranelift-only binary reports
LLVM as not-built.
