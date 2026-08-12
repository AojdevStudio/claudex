#![allow(dead_code)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use tempfile::TempDir;

pub struct Fixture {
    root: TempDir,
    pub config: PathBuf,
    pub fake_claude: PathBuf,
}

impl Fixture {
    pub fn new() -> Self {
        let temp_root = std::env::temp_dir()
            .canonicalize()
            .expect("resolve platform temp directory");
        let root = tempfile::Builder::new()
            .prefix("claudex-test-")
            .tempdir_in(temp_root)
            .expect("create fixture directory");
        let key = root.path().join("api-key");
        fs::write(&key, "fixture-key\n").expect("write fixture key");
        fs::set_permissions(&key, fs::Permissions::from_mode(0o600)).expect("secure fixture key");

        let fake_claude = root.path().join("claude");
        fs::write(
            &fake_claude,
            r#"#!/bin/sh
printf 'argv_count=%s\n' "$#"
index=1
for argument in "$@"; do
  printf 'argv[%s]=%s\n' "$index" "$argument"
  index=$((index + 1))
done
for name in ANTHROPIC_BASE_URL ANTHROPIC_API_KEY CLAUDE_CODE_OAUTH_TOKEN CLAUDE_CODE_OAUTH_REFRESH_TOKEN CLAUDE_CODE_OAUTH_SCOPES CLAUDE_CODE_USE_BEDROCK CLAUDE_CODE_USE_VERTEX CLAUDE_CODE_USE_FOUNDRY CLAUDE_CODE_USE_MANTLE CLAUDE_CODE_USE_ANTHROPIC_AWS ANTHROPIC_DEFAULT_FABLE_MODEL ANTHROPIC_DEFAULT_OPUS_MODEL ANTHROPIC_DEFAULT_SONNET_MODEL ANTHROPIC_DEFAULT_HAIKU_MODEL ANTHROPIC_CUSTOM_MODEL_OPTION ANTHROPIC_CUSTOM_MODEL_OPTION_NAME ANTHROPIC_CUSTOM_MODEL_OPTION_DESCRIPTION CLAUDE_CODE_SUBAGENT_MODEL CLAUDE_CODE_ALWAYS_ENABLE_EFFORT CLAUDE_CODE_MAX_TOOL_USE_CONCURRENCY ENABLE_TOOL_SEARCH; do
  if value=$(printenv "$name"); then
    printf '%s=%s\n' "$name" "$value"
  else
    printf '%s=<UNSET>\n' "$name"
  fi
done
if [ "$(printenv ANTHROPIC_AUTH_TOKEN)" = "fixture-key" ]; then
  printf 'ANTHROPIC_AUTH_TOKEN=<EXPECTED>\n'
else
  printf 'ANTHROPIC_AUTH_TOKEN=<UNEXPECTED>\n'
fi
printf 'CLAUDEX_ORDINARY_ENV=%s\n' "${CLAUDEX_ORDINARY_ENV:-<UNSET>}"
"#,
        )
        .expect("write fake Claude");
        fs::set_permissions(&fake_claude, fs::Permissions::from_mode(0o700))
            .expect("make fake Claude executable");

        let config = root.path().join("config.toml");
        write_config(&config, &key);

        Self {
            root,
            config,
            fake_claude,
        }
    }

    pub fn key_path(&self) -> PathBuf {
        self.root.path().join("api-key")
    }

    pub fn marker_path(&self) -> PathBuf {
        self.root.path().join("claude-ran")
    }

    pub fn set_fake_claude(&self, source: &str) {
        fs::write(&self.fake_claude, source).expect("replace fake Claude");
        fs::set_permissions(&self.fake_claude, fs::Permissions::from_mode(0o700))
            .expect("make replacement executable");
    }

    pub fn set_base_url(&self, base_url: &str) {
        let source = fs::read_to_string(&self.config).expect("read fixture config");
        let updated = source.replace("http://127.0.0.1:18317", base_url);
        fs::write(&self.config, updated).expect("update fixture base URL");
    }

    pub fn command(&self) -> Command {
        let mut command = assert_cmd::cargo::cargo_bin_cmd!("claudex");
        command
            .env("CLAUDEX_CONFIG", &self.config)
            .env("CLAUDEX_CLAUDE_PATH", &self.fake_claude)
            .env("ANTHROPIC_API_KEY", "must-be-removed")
            .env("ANTHROPIC_AUTH_TOKEN", "must-be-replaced")
            .env("CLAUDE_CODE_OAUTH_TOKEN", "must-be-removed")
            .env("CLAUDE_CODE_OAUTH_REFRESH_TOKEN", "must-be-removed")
            .env("CLAUDE_CODE_OAUTH_SCOPES", "must-be-removed")
            .env("CLAUDE_CODE_USE_BEDROCK", "1")
            .env("CLAUDE_CODE_USE_VERTEX", "1")
            .env("CLAUDE_CODE_USE_FOUNDRY", "1")
            .env("CLAUDE_CODE_USE_MANTLE", "1")
            .env("CLAUDE_CODE_USE_ANTHROPIC_AWS", "1")
            .env("ANTHROPIC_CUSTOM_MODEL_OPTION", "must-be-replaced")
            .env("ANTHROPIC_CUSTOM_MODEL_OPTION_NAME", "must-be-replaced")
            .env(
                "ANTHROPIC_CUSTOM_MODEL_OPTION_DESCRIPTION",
                "must-be-replaced",
            )
            .env("CLAUDEX_ORDINARY_ENV", "inherited");
        command
    }
}

pub fn write_config(path: &Path, key: &Path) {
    fs::write(
        path,
        format!(
            r#"[proxy]
base_url = "http://127.0.0.1:18317"
api_key_file = "{}"

[defaults]
model = "fable"

[models]
fable = "provider-fable"
opus = "provider-opus"
sonnet = "provider-sonnet"
haiku = "provider-haiku"

[custom_model]
id = "provider-custom"
name = "Custom provider model"
description = "Custom model routed through the test gateway"

[claude]
subagent_model = "inherit"
always_enable_effort = true
max_tool_use_concurrency = 3
enable_tool_search = false
"#,
            key.display()
        ),
    )
    .expect("write fixture config");
}
