use crate::config::Config;
use crate::error::ClaudexError;

pub struct ResolvedModel {
    pub id: String,
    pub context_window: u32,
}

pub fn resolve(
    config: &Config,
    alias: Option<&str>,
    proxy_model: Option<&str>,
    context_window_override: Option<u32>,
) -> Result<ResolvedModel, ClaudexError> {
    let id = if let Some(raw) = proxy_model {
        if raw.trim().is_empty() {
            return Err(ClaudexError::Arguments(
                "--proxy-model requires a non-empty value".into(),
            ));
        }
        raw.to_owned()
    } else {
        let alias = alias.unwrap_or(&config.defaults.model);
        config.models.get(alias).map(str::to_owned).ok_or_else(|| {
            ClaudexError::Arguments(format!(
                "unknown model alias '{alias}'; valid aliases: fable, haiku, opus, sonnet"
            ))
        })?
    };
    let context_window = config.context_window_for(&id, context_window_override)?;
    Ok(ResolvedModel { id, context_window })
}

pub fn print(config: &Config) -> Result<(), ClaudexError> {
    for (alias, model) in config.models.entries() {
        let default = if alias == config.defaults.model {
            "\tdefault"
        } else {
            ""
        };
        let context_window = config.context_window_for(model, None)?;
        println!("{alias}\t{model}{default}\tcontext={context_window}");
    }
    if let Some(custom) = &config.custom_model {
        let context_window = config.context_window_for(&custom.id, None)?;
        println!("custom\t{}\tpicker\tcontext={context_window}", custom.id);
    }
    Ok(())
}
