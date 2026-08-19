# claudex

`claudex` is a small Rust launcher for running an installed Claude Code binary through an OpenAI-compatible model gateway. It keeps model aliases and machine-specific endpoints in local TOML, loads the gateway key from a locked-down file, removes conflicting provider credentials from the child environment, and then replaces itself with Claude Code.

It does not implement inference, OAuth, provider routing, plugins, failover, or self-update. Those remain the gateway's responsibility.

## Install

Homebrew provides the same install and upgrade workflow on macOS, Linux, and WSL 2:

```bash
brew install AojdevStudio/tap/claudex
brew upgrade claudex
```

The release pipeline produces prebuilt archives for Apple Silicon macOS, x86-64 Linux, and ARM64 Linux. The formula installs the executable and zsh completions. Native `claude` remains a separate command and keeps its normal authentication.

## Configure

```bash
mkdir -p ~/.config/claudex
cp config.example.toml ~/.config/claudex/config.toml
chmod 600 ~/.config/claudex/config.toml
```

Edit the endpoint, alias mappings, and exact context windows for the machine. Every model referenced by `[models]` or `[custom_model]` must have a positive token count in `[context_windows]`. Claudex exports the selected model's value as `CLAUDE_CODE_MAX_CONTEXT_TOKENS`, so Claude Code compacts against the gateway model's real limit instead of assuming 200k for an unknown model ID.

```toml
[models]
fable = "provider-large"
haiku = "provider-small"

[context_windows]
"provider-large" = 1000000
"provider-small" = 200000
```

Keep the API key outside the TOML file:

```bash
mkdir -p ~/.config/cliproxyapi
chmod 700 ~/.config/cliproxyapi
chmod 600 ~/.config/cliproxyapi/api-key
```

The API-key path must contain no symlinks and resolve to a regular file owned by the current user. The file must be owner-readable, have no group or world permission bits, and contain one non-empty UTF-8 line. BWS can remain the source of record while a separately managed process materializes this runtime file.

## Use

```bash
claudex
claudex --model fable
claudex --model opus --resume my-session
claudex --proxy-model 'provider/raw-model(high)' -p 'Return exactly OK'
claudex --proxy-model 'provider/new-model' --context-window 500k
claudex -- --model value-that-must-reach-claude

claudex models
claudex config validate
claudex doctor
claudex doctor --live
claudex completions zsh
claudex --version
```

`--model` accepts only the four configured aliases: `fable`, `opus`, `sonnet`, and `haiku`. `--proxy-model` is the explicit raw gateway-model path. Raw IDs use their `[context_windows]` entry; an unmapped raw ID fails before Claude Code starts. Use `--context-window` with a positive integer, `k`, or `m` suffix (for example, `500k`, `1m`, or `1050000`) to override the selected model for one launch. Every other argument retains its relative ordering when forwarded to Claude Code. Use `--` when a forwarded value looks like a claudex selector or command.

`claudex models` prints the effective window for every alias and the optional custom picker entry. The context window is process-wide and fixed when Claudex starts Claude Code. If you switch to a model with a different limit from Claude Code's `/model` picker, start a new session with `claudex --proxy-model MODEL_ID` so the matching window is applied.

Configuration precedence is:

1. `--model` or `--proxy-model`, plus an optional one-launch `--context-window`
2. supported `CLAUDEX_*` environment overrides
3. `~/.config/claudex/config.toml`
4. non-sensitive application defaults

Supported overrides are `CLAUDEX_CONFIG`, `CLAUDEX_BASE_URL`, `CLAUDEX_API_KEY_FILE`, `CLAUDEX_DEFAULT_MODEL`, and `CLAUDEX_CLAUDE_PATH`. `CLAUDEX_DOCTOR_TIMEOUT_MS` can shorten or extend the default eight-second doctor HTTP timeout.

## Diagnostics

`claudex doctor` validates configuration, key safety, Claude Code discovery, and the authenticated gateway model-listing endpoint. It performs no inference.

`claudex doctor --live` runs the same checks and then launches the installed Claude Code binary in print mode with a minimal exact-response probe. It exits `4` when the process fails or returns an unexpected response. Configuration errors exit `2`; gateway HTTP failures and timeouts exit `3`.

Requested and pipeable results use stdout. Diagnostics use stderr. Secrets are not included in debug output, errors, argv, or committed configuration.

## Development

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets
MACOSX_DEPLOYMENT_TARGET=11.0 cargo build --release --target aarch64-apple-darwin
git diff --check
```

Release archives target Apple Silicon Macs running macOS 11.0 or later, x86-64 Linux, and ARM64 Linux. Linux releases use statically linked MUSL targets for portability across distributions. See [SECURITY.md](SECURITY.md) for the credential boundary and [docs/rollback.md](docs/rollback.md) for cutover recovery.
