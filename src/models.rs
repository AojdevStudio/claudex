use crate::config::Config;
use crate::error::ClaudexError;

pub fn resolve(
    config: &Config,
    alias: Option<&str>,
    proxy_model: Option<&str>,
) -> Result<String, ClaudexError> {
    if let Some(raw) = proxy_model {
        if raw.trim().is_empty() {
            return Err(ClaudexError::Arguments(
                "--proxy-model requires a non-empty value".into(),
            ));
        }
        return Ok(raw.to_owned());
    }

    let alias = alias.unwrap_or(&config.defaults.model);
    config.models.get(alias).map(str::to_owned).ok_or_else(|| {
        ClaudexError::Arguments(format!(
            "unknown model alias '{alias}'; valid aliases: fable, haiku, opus, sonnet"
        ))
    })
}

pub fn print(config: &Config) {
    for (alias, model) in config.models.entries() {
        let suffix = if alias == config.defaults.model {
            "\tdefault"
        } else {
            ""
        };
        println!("{alias}\t{model}{suffix}");
    }
}
