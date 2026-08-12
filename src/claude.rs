use std::env;
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::process::Stdio;
use std::thread;
use std::time::{Duration, Instant};

use crate::config::Config;
use crate::error::ClaudexError;
use crate::secrets::Secret;

const REMOVED_ENVIRONMENT: &[&str] = &[
    "ANTHROPIC_API_KEY",
    "CLAUDE_CODE_OAUTH_TOKEN",
    "CLAUDE_CODE_OAUTH_REFRESH_TOKEN",
    "CLAUDE_CODE_OAUTH_SCOPES",
    "CLAUDE_CODE_USE_BEDROCK",
    "CLAUDE_CODE_USE_VERTEX",
    "CLAUDE_CODE_USE_FOUNDRY",
    "CLAUDE_CODE_USE_MANTLE",
    "CLAUDE_CODE_USE_ANTHROPIC_AWS",
    "ANTHROPIC_CUSTOM_MODEL_OPTION",
    "ANTHROPIC_CUSTOM_MODEL_OPTION_NAME",
    "ANTHROPIC_CUSTOM_MODEL_OPTION_DESCRIPTION",
];

pub fn launch(config: &Config, model: &str, args: &[OsString]) -> Result<(), ClaudexError> {
    let secret = config.load_secret()?;
    let executable = resolve(config)?;
    let mut command = build_command(config, &secret, &executable, model, args);
    let error = command.exec();
    Err(ClaudexError::Launch(error))
}

pub fn run_live(
    config: &Config,
    model: &str,
    args: &[OsString],
    timeout: Duration,
) -> Result<std::process::Output, ClaudexError> {
    let secret = config.load_secret()?;
    let executable = resolve(config)?;
    let mut command = build_command(config, &secret, &executable, model, args);
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|error| ClaudexError::DoctorLive(format!("cannot launch Claude Code: {error}")))?;
    let started = Instant::now();
    loop {
        if child
            .try_wait()
            .map_err(|error| {
                ClaudexError::DoctorLive(format!("cannot wait for Claude Code: {error}"))
            })?
            .is_some()
        {
            return child.wait_with_output().map_err(|error| {
                ClaudexError::DoctorLive(format!("cannot collect Claude Code output: {error}"))
            });
        }
        if started.elapsed() >= timeout {
            child.kill().map_err(|error| {
                ClaudexError::DoctorLive(format!(
                    "Claude Code timed out and could not be stopped: {error}"
                ))
            })?;
            child.wait().map_err(|error| {
                ClaudexError::DoctorLive(format!(
                    "Claude Code timed out and could not be reaped: {error}"
                ))
            })?;
            return Err(ClaudexError::DoctorLive(format!(
                "Claude Code timed out after {} ms",
                timeout.as_millis()
            )));
        }
        thread::sleep(Duration::from_millis(10));
    }
}

pub fn resolve(config: &Config) -> Result<PathBuf, ClaudexError> {
    let current_path = env::current_exe().map_err(|error| {
        ClaudexError::ClaudeNotFound(format!("cannot identify the claudex executable: {error}"))
    })?;
    let current = current_path.canonicalize().map_err(|error| {
        ClaudexError::ClaudeNotFound(format!(
            "cannot resolve {}: {error}",
            current_path.display()
        ))
    })?;
    if let Some(path) = config.claude_path_override() {
        return validate_candidate(path, Some(&current));
    }

    let path = env::var_os("PATH").ok_or_else(|| {
        ClaudexError::ClaudeNotFound("PATH is not set and no override was provided".into())
    })?;
    for directory in env::split_paths(&path) {
        let candidate = directory.join("claude");
        if let Ok(valid) = validate_candidate(candidate, Some(&current)) {
            return Ok(valid);
        }
    }
    Err(ClaudexError::ClaudeNotFound(
        "no non-recursive executable named 'claude' was found on PATH".into(),
    ))
}

fn validate_candidate(path: PathBuf, current: Option<&Path>) -> Result<PathBuf, ClaudexError> {
    let metadata = fs::metadata(&path).map_err(|error| {
        ClaudexError::ClaudeNotFound(format!("cannot inspect {}: {error}", path.display()))
    })?;
    if !metadata.is_file() || metadata.permissions().mode() & 0o111 == 0 {
        return Err(ClaudexError::ClaudeNotFound(format!(
            "{} is not an executable file",
            path.display()
        )));
    }
    let canonical = path.canonicalize().map_err(|error| {
        ClaudexError::ClaudeNotFound(format!("cannot resolve {}: {error}", path.display()))
    })?;
    if current.is_some_and(|current| current == canonical) {
        return Err(ClaudexError::ClaudeNotFound(
            "resolved Claude path points back to claudex".into(),
        ));
    }
    Ok(canonical)
}

fn build_command(
    config: &Config,
    secret: &Secret,
    executable: &Path,
    model: &str,
    args: &[OsString],
) -> Command {
    let mut command = Command::new(executable);
    command.arg("--model").arg(model).args(args);
    for name in REMOVED_ENVIRONMENT {
        command.env_remove(name);
    }
    command
        .env("ANTHROPIC_BASE_URL", &config.proxy.base_url)
        .env("ANTHROPIC_AUTH_TOKEN", secret.expose())
        .env("ANTHROPIC_DEFAULT_FABLE_MODEL", &config.models.fable)
        .env("ANTHROPIC_DEFAULT_OPUS_MODEL", &config.models.opus)
        .env("ANTHROPIC_DEFAULT_SONNET_MODEL", &config.models.sonnet)
        .env("ANTHROPIC_DEFAULT_HAIKU_MODEL", &config.models.haiku)
        .env("CLAUDE_CODE_SUBAGENT_MODEL", &config.claude.subagent_model)
        .env(
            "CLAUDE_CODE_ALWAYS_ENABLE_EFFORT",
            if config.claude.always_enable_effort {
                "1"
            } else {
                "0"
            },
        )
        .env(
            "CLAUDE_CODE_MAX_TOOL_USE_CONCURRENCY",
            config.claude.max_tool_use_concurrency.to_string(),
        )
        .env(
            "ENABLE_TOOL_SEARCH",
            config.claude.enable_tool_search.to_string(),
        );
    if let Some(custom) = &config.custom_model {
        command.env("ANTHROPIC_CUSTOM_MODEL_OPTION", &custom.id);
        if let Some(name) = &custom.name {
            command.env("ANTHROPIC_CUSTOM_MODEL_OPTION_NAME", name);
        }
        if let Some(description) = &custom.description {
            command.env("ANTHROPIC_CUSTOM_MODEL_OPTION_DESCRIPTION", description);
        }
    }
    command
}
