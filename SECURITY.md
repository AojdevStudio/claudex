# Security model

`claudex` is a local credential-bearing process boundary. It reads one gateway token, places it only in the child Claude Code environment as `ANTHROPIC_AUTH_TOKEN`, and replaces itself with that process on normal launches.

## Secret invariants

- Live credentials never belong in the repository, TOML configuration, command-line arguments, logs, error messages, checksums, or migration fingerprints.
- The key path may not contain a symbolic link. It must resolve to a regular file owned by the current user.
- The key file must be owner-readable and have no group or world permission bits.
- Empty, NUL-containing, non-UTF-8, and multiline values are rejected. Exactly one terminal LF or CRLF is removed.
- The Rust secret type redacts both `Debug` and `Display` and has no serialization implementation.

## Child isolation

Before setting gateway values, `claudex` removes ambient Anthropic API keys, Claude OAuth tokens, and Bedrock, Vertex, Foundry, Mantle, and Anthropic-on-AWS selectors. It injects only the configured gateway endpoint, bearer token, model mappings, picker metadata, subagent model, effort behavior, concurrency, and tool-search behavior.

The executable resolves a real `claude` binary and rejects a path that canonicalizes back to `claudex`. Normal launches use Unix `exec`, preserving terminal ownership, signals, and the final Claude Code exit status.

## Scope

`claudex` does not store or refresh OAuth credentials, configure gateway providers or plugins, use a management key, implement fallback, or collect analytics. Gateway security, provider authentication, quotas, and routing remain outside this process.

Report suspected vulnerabilities privately through GitHub's security advisory flow for this repository. Do not include a live credential in a report.
