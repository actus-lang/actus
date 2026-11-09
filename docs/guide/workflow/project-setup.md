# Project setup

Start from the latest Actus package workflow and keep the manifest, source root,
runtime profile, and entry declaration aligned.

A setup sequence is:

1. create or initialize a package;
2. inspect `Actus.toml` and `Actus.lock`;
3. place the entry source under the configured source root;
4. add modules through canonical facades;
5. select the runtime and target required by the program;
6. run a check before adding native or external dependencies.

Keep generated build output and local artifacts outside committed source. Read
the repository contribution instructions before creating a branch or pull
request.

## Native backend setup

Actus uses LLVM as its default native backend. From the repository root, run
these two commands on a supported host:

```sh
./scripts/bootstrap_llvm.sh
cargo build
```

On Windows, use `./scripts/bootstrap_llvm.ps1` in PowerShell in place of the
Unix bootstrap command. The bootstrap script selects the host archive from
`toolchains/llvm/llvm-23.1.2.json`, verifies its declared size and SHA-256
before extraction, and installs the result under
`.actus-toolchain/llvm-23.1.2/`. It is idempotent and does not edit shell
profiles, global Cargo configuration, or system packages.

Developers with an independently managed LLVM may set
`LLVM_SYS_231_PREFIX` explicitly. Cargo then uses that prefix instead of the
repository-local one, and Actus emits an `unverified` toolchain warning. Use
the repository bootstrap for strict builds and reproducible backend evidence.

Inspect the selected compiler and toolchain identity with:

\`\`\`sh
actus --version
actus --version --json
\`\`\`

## Installing a user archive

User archives are relocatable. Extract the host archive, keep `bin/actus` next
to `lib/actus/` and `share/actus/`, and add `bin/` to `PATH` if desired. The
archive carries its standard library, runtime archive, bundled freestanding
linker, compiler-rt builtins, identity manifest, and license notices.

The alpha archives are unsigned. On macOS, a quarantine attribute may block
an extracted binary; inspect the source and remove quarantine only when the
archive is trusted, for example with `xattr -dr com.apple.quarantine <dir>`.
Windows hosted executable builds require the Windows SDK and MSVC linker; the
archive supplies the three x64 VC runtime DLLs but does not replace the SDK.
This is a known alpha limitation: hosted Windows linking needs `lld-link` and
the Windows SDK, while freestanding ARM linking works from the bundled
`ld.lld` and compiler-rt builtins.

To remove an installation, delete its extracted archive directory and remove
that directory's `bin/` entry from `PATH`; restore the previous `PATH` value if
the entry was added manually. To roll back, remove the current installation,
extract the previous archive into the same location, and rerun
`actus --version --json` before using it.
