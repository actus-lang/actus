# Security Policy

## Supported Versions

Actus is currently in an early alpha stage. Security fixes are applied to the latest `main` branch and the latest published release, when a release exists.

| Version | Supported |
| ------- | --------- |
| `main` | Yes |
| Latest alpha release | Yes |
| Older releases | No |

Because Actus is experimental software, users should not rely on it for production systems or safety-critical workloads without independent review.

## Reporting a Vulnerability

Please do not report security vulnerabilities through public GitHub issues, pull requests, discussions, or social media.

Use GitHub's private vulnerability reporting feature for this repository when it is enabled. If private vulnerability reporting is unavailable, contact the project maintainers privately through the contact method listed on the Actus GitHub organization or repository profile.

Please include:

- a clear description of the vulnerability;
- the affected commit, version, or component;
- precise reproduction steps or a minimal proof of concept;
- the expected and actual behavior;
- the potential impact;
- any suggested mitigation or fix, if available.

Please avoid including secrets, private data, or unnecessary exploit details in the initial report.

## Response Process

Maintainers will make a reasonable effort to:

1. acknowledge a report privately;
2. reproduce and assess the issue;
3. determine the affected versions and severity;
4. develop and test a fix;
5. coordinate disclosure with the reporter;
6. publish a security advisory when appropriate.

Response times may vary because Actus is maintained as an open-source project by a small team.

## Disclosure Policy

Please allow maintainers reasonable time to investigate and address a vulnerability before public disclosure.

When a fix is available, the project may publish a GitHub Security Advisory containing the affected versions, fixed versions, impact, and mitigation guidance.

Reporter credit will be provided when requested and when doing so does not create a privacy or security concern.

## Scope

This policy covers security issues in:

- the Actus compiler;
- the lexer, parser, semantic analyzer, and code generator;
- official build and release automation;
- official repository tooling that can affect released artifacts.

Issues in third-party dependencies should also be reported to their upstream maintainers when appropriate. Please include relevant dependency information in the Actus report if the issue affects Actus users.

## Safe Harbor

Security research conducted in good faith, within the scope of this policy, and without accessing, modifying, or exfiltrating data that does not belong to you will be treated as authorized activity. Researchers should stop testing and report promptly after confirming a vulnerability.
