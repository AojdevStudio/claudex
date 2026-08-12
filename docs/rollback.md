# Cutover and rollback

Keep the fingerprinted shell-function and configuration backups until both proxy and native-Claude inference pass after cutover.

## Before cutover

```bash
mkdir -p ~/.config/claudex/migration
chmod 700 ~/.config/claudex/migration
command cp -p ~/.zshrc.local ~/.config/claudex/migration/zshrc.local.before-claudex-binary
command cp -p ~/.config/claudex/config.toml ~/.config/claudex/migration/config.toml.before-cutover
```

Canary the Homebrew executable by absolute path while the function still exists:

```bash
"$(brew --prefix)/bin/claudex" config validate
"$(brew --prefix)/bin/claudex" doctor
"$(brew --prefix)/bin/claudex" models
"$(brew --prefix)/bin/claudex" -p 'Return exactly CLAUDEX_OK'
```

Remove only the previously fingerprinted `claudex()` function block. Start a fresh shell and verify `whence -va claudex` resolves the Homebrew executable and no function.

## Roll back

```bash
brew unlink claudex
command cp -p ~/.config/claudex/migration/zshrc.local.before-claudex-binary ~/.zshrc.local
command cp -p ~/.config/claudex/migration/config.toml.before-cutover ~/.config/claudex/config.toml
exec zsh -l
```

Confirm the restored function fingerprint matches the recorded pre-cutover fingerprint, then prove native Claude remains independent:

```bash
env -u ANTHROPIC_BASE_URL \
  -u ANTHROPIC_AUTH_TOKEN \
  -u ANTHROPIC_API_KEY \
  -u CLAUDE_CODE_OAUTH_TOKEN \
  -u CLAUDE_CODE_USE_BEDROCK \
  -u CLAUDE_CODE_USE_VERTEX \
  -u CLAUDE_CODE_USE_FOUNDRY \
  -u CLAUDE_CODE_USE_MANTLE \
  -u CLAUDE_CODE_USE_ANTHROPIC_AWS \
  claude -p 'Return exactly CLAUDE_NATIVE_OK'
```

After the rehearsal succeeds, restore the binary with `brew link claudex`, start a fresh shell, and rerun both exact inference probes. Use `brew uninstall claudex` instead of `brew unlink claudex` only when testing complete package removal.
