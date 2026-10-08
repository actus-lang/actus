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
