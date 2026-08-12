use std::ffi::OsString;

use clap::{Arg, Command};

use crate::error::ClaudexError;

pub enum Action {
    Help,
    Version,
    Models,
    Validate,
    Doctor {
        live: bool,
    },
    Completions,
    Launch {
        alias: Option<String>,
        proxy_model: Option<String>,
        claude_args: Vec<OsString>,
    },
}

pub struct Invocation {
    pub action: Action,
}

impl Invocation {
    pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Self, ClaudexError> {
        let args: Vec<OsString> = args.into_iter().collect();

        if matches!(
            args.first().and_then(|value| value.to_str()),
            Some("--help" | "-h")
        ) {
            return Ok(Self {
                action: Action::Help,
            });
        }
        if matches!(
            args.first().and_then(|value| value.to_str()),
            Some("--version" | "-V")
        ) {
            return Ok(Self {
                action: Action::Version,
            });
        }
        if args.first().is_some_and(|value| value == "models") {
            ensure_exact_args(&args, 1, "models")?;
            return Ok(Self {
                action: Action::Models,
            });
        }
        if args.first().is_some_and(|value| value == "doctor") {
            let live = match args.get(1).and_then(|value| value.to_str()) {
                None => false,
                Some("--live") if args.len() == 2 => true,
                _ => {
                    return Err(ClaudexError::Arguments(
                        "usage: claudex doctor [--live]".into(),
                    ));
                }
            };
            return Ok(Self {
                action: Action::Doctor { live },
            });
        }
        if args.first().is_some_and(|value| value == "config") {
            if args.len() == 2 && args[1] == "validate" {
                return Ok(Self {
                    action: Action::Validate,
                });
            }
            return Err(ClaudexError::Arguments(
                "usage: claudex config validate".into(),
            ));
        }
        if args.first().is_some_and(|value| value == "completions") {
            if args.len() == 2 && args[1] == "zsh" {
                return Ok(Self {
                    action: Action::Completions,
                });
            }
            return Err(ClaudexError::Arguments(
                "usage: claudex completions zsh".into(),
            ));
        }

        parse_launch(args)
    }
}

fn parse_launch(args: Vec<OsString>) -> Result<Invocation, ClaudexError> {
    let mut alias = None;
    let mut proxy_model = None;
    let mut claude_args = Vec::new();
    let mut index = 0;

    while index < args.len() {
        if args[index] == "--" {
            claude_args.extend(args[index..].iter().cloned());
            break;
        }

        if args[index] == "--model" {
            let value = required_value(&args, index, "--model")?;
            if alias.replace(value).is_some() {
                return Err(ClaudexError::Arguments(
                    "--model may be specified only once".into(),
                ));
            }
            index += 2;
            continue;
        }
        if args[index] == "--proxy-model" {
            let value = required_value(&args, index, "--proxy-model")?;
            if proxy_model.replace(value).is_some() {
                return Err(ClaudexError::Arguments(
                    "--proxy-model may be specified only once".into(),
                ));
            }
            index += 2;
            continue;
        }

        claude_args.push(args[index].clone());
        index += 1;
    }

    if alias.is_some() && proxy_model.is_some() {
        return Err(ClaudexError::Arguments(
            "--model and --proxy-model cannot be used together".into(),
        ));
    }

    Ok(Invocation {
        action: Action::Launch {
            alias,
            proxy_model,
            claude_args,
        },
    })
}

fn required_value(args: &[OsString], index: usize, flag: &str) -> Result<String, ClaudexError> {
    let value = args
        .get(index + 1)
        .filter(|value| *value != "--")
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ClaudexError::Arguments(format!("{flag} requires a non-empty value")))?;
    Ok(value.to_owned())
}

fn ensure_exact_args(args: &[OsString], count: usize, command: &str) -> Result<(), ClaudexError> {
    if args.len() == count {
        Ok(())
    } else {
        Err(ClaudexError::Arguments(format!("usage: claudex {command}")))
    }
}

pub fn command() -> Command {
    Command::new("claudex")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Launch Claude Code through an OpenAI-compatible model gateway")
        .arg(
            Arg::new("model")
                .long("model")
                .value_name("ALIAS")
                .help("Select a configured model alias"),
        )
        .arg(
            Arg::new("proxy-model")
                .long("proxy-model")
                .value_name("ID")
                .help("Select an explicit raw proxy model ID"),
        )
        .subcommand(Command::new("models").about("List configured model aliases"))
        .subcommand(
            Command::new("doctor")
                .about("Check the local launch path")
                .arg(
                    Arg::new("live")
                        .long("live")
                        .help("Run one Claude Code print-mode inference"),
                ),
        )
        .subcommand(
            Command::new("config").subcommand(Command::new("validate").about("Validate config")),
        )
        .subcommand(
            Command::new("completions")
                .subcommand(Command::new("zsh").about("Generate zsh completions")),
        )
        .after_help("Use -- to forward values that resemble claudex commands.")
}
