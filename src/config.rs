use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::ClaudexError;
use crate::secrets::Secret;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub proxy: ProxyConfig,
    pub defaults: DefaultsConfig,
    pub models: ModelMap,
    #[serde(default)]
    pub context_windows: BTreeMap<String, u32>,
    #[serde(default)]
    pub custom_model: Option<CustomModel>,
    #[serde(default)]
    pub claude: ClaudeConfig,
    #[serde(skip)]
    claude_path_override: Option<PathBuf>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProxyConfig {
    pub base_url: String,
    pub api_key_file: PathBuf,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DefaultsConfig {
    pub model: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelMap {
    pub fable: String,
    pub opus: String,
    pub sonnet: String,
    pub haiku: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CustomModel {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ClaudeConfig {
    pub subagent_model: String,
    pub always_enable_effort: bool,
    pub max_tool_use_concurrency: u16,
    pub enable_tool_search: bool,
}

impl Default for ClaudeConfig {
    fn default() -> Self {
        Self {
            subagent_model: "inherit".into(),
            always_enable_effort: true,
            max_tool_use_concurrency: 3,
            enable_tool_search: false,
        }
    }
}

#[derive(Default)]
pub struct Overrides {
    pub config_path: Option<PathBuf>,
    pub base_url: Option<String>,
    pub api_key_file: Option<PathBuf>,
    pub default_model: Option<String>,
    pub claude_path: Option<PathBuf>,
}

impl Overrides {
    pub fn from_env() -> Result<Self, ClaudexError> {
        Ok(Self {
            config_path: env::var_os("CLAUDEX_CONFIG").map(PathBuf::from),
            base_url: string_override("CLAUDEX_BASE_URL")?,
            api_key_file: env::var_os("CLAUDEX_API_KEY_FILE").map(PathBuf::from),
            default_model: string_override("CLAUDEX_DEFAULT_MODEL")?,
            claude_path: env::var_os("CLAUDEX_CLAUDE_PATH").map(PathBuf::from),
        })
    }
}

fn string_override(name: &str) -> Result<Option<String>, ClaudexError> {
    match env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(env::VarError::NotUnicode(_)) => {
            Err(ClaudexError::Config(format!("{name} must be valid UTF-8")))
        }
    }
}

impl Config {
    pub fn load(overrides: Overrides) -> Result<Self, ClaudexError> {
        let path = overrides.config_path.unwrap_or_else(default_config_path);
        let source = fs::read_to_string(&path).map_err(|source| ClaudexError::ConfigRead {
            path: path.clone(),
            source,
        })?;
        let mut config: Self =
            toml::from_str(&source).map_err(|source| ClaudexError::ConfigParse {
                path: path.clone(),
                source,
            })?;

        if let Some(base_url) = overrides.base_url {
            config.proxy.base_url = base_url;
        }
        if let Some(api_key_file) = overrides.api_key_file {
            config.proxy.api_key_file = api_key_file;
        }
        if let Some(default_model) = overrides.default_model {
            config.defaults.model = default_model;
        }

        config.proxy.api_key_file = expand_home(&config.proxy.api_key_file)?;
        config.validate()?;
        config.claude_path_override = overrides.claude_path;
        Ok(config)
    }

    fn validate(&self) -> Result<(), ClaudexError> {
        if self.proxy.base_url.trim().is_empty() {
            return Err(ClaudexError::Config(
                "proxy.base_url cannot be empty".into(),
            ));
        }
        let base_url = reqwest::Url::parse(&self.proxy.base_url)
            .map_err(|error| ClaudexError::Config(format!("proxy.base_url is invalid: {error}")))?;
        if !matches!(base_url.scheme(), "http" | "https") {
            return Err(ClaudexError::Config(
                "proxy.base_url must use http or https".into(),
            ));
        }
        if base_url.host_str().is_none()
            || base_url.query().is_some()
            || base_url.fragment().is_some()
        {
            return Err(ClaudexError::Config(
                "proxy.base_url must have a host and no query or fragment".into(),
            ));
        }
        if self.claude.max_tool_use_concurrency == 0 {
            return Err(ClaudexError::Config(
                "claude.max_tool_use_concurrency must be at least 1".into(),
            ));
        }
        for (name, value) in self.models.entries() {
            if value.trim().is_empty() {
                return Err(ClaudexError::Config(format!(
                    "models.{name} cannot be empty"
                )));
            }
            self.validate_context_window(value, &format!("models.{name}"))?;
        }
        if let Some(custom) = &self.custom_model {
            if custom.id.trim().is_empty() {
                return Err(ClaudexError::Config(
                    "custom_model.id cannot be empty".into(),
                ));
            }
            self.validate_context_window(&custom.id, "custom_model.id")?;
        }
        if self.models.get(&self.defaults.model).is_none() {
            return Err(ClaudexError::Config(
                "defaults.model must be one of: fable, haiku, opus, sonnet".into(),
            ));
        }
        Ok(())
    }

    fn validate_context_window(&self, model: &str, source: &str) -> Result<(), ClaudexError> {
        match self.context_windows.get(model) {
            Some(0) => Err(ClaudexError::Config(format!(
                "context_windows.{model} must be greater than 0"
            ))),
            Some(_) => Ok(()),
            None => Err(ClaudexError::Config(format!(
                "{source} selects '{model}', but context_windows has no entry for that model"
            ))),
        }
    }

    pub fn context_window_for(
        &self,
        model: &str,
        override_tokens: Option<u32>,
    ) -> Result<u32, ClaudexError> {
        if let Some(tokens) = override_tokens {
            return Ok(tokens);
        }
        self.context_windows.get(model).copied().ok_or_else(|| {
            ClaudexError::Arguments(format!(
                "model '{model}' has no configured context window; add context_windows.\"{model}\" to the config"
            ))
        })
    }

    pub fn load_secret(&self) -> Result<Secret, ClaudexError> {
        Secret::load(&self.proxy.api_key_file)
    }

    pub fn claude_path_override(&self) -> Option<PathBuf> {
        self.claude_path_override.clone()
    }
}

impl ModelMap {
    pub fn entries(&self) -> [(&'static str, &str); 4] {
        [
            ("fable", &self.fable),
            ("haiku", &self.haiku),
            ("opus", &self.opus),
            ("sonnet", &self.sonnet),
        ]
    }

    pub fn get(&self, alias: &str) -> Option<&str> {
        match alias {
            "fable" => Some(&self.fable),
            "haiku" => Some(&self.haiku),
            "opus" => Some(&self.opus),
            "sonnet" => Some(&self.sonnet),
            _ => None,
        }
    }
}

fn default_config_path() -> PathBuf {
    home_dir().join(".config/claudex/config.toml")
}

fn expand_home(path: &Path) -> Result<PathBuf, ClaudexError> {
    let value = path.to_string_lossy();
    if value == "~" {
        return Ok(home_dir());
    }
    if let Some(rest) = value.strip_prefix("~/") {
        return Ok(home_dir().join(rest));
    }
    Ok(path.to_path_buf())
}

fn home_dir() -> PathBuf {
    env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"))
}
