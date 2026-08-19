mod claude;
mod cli;
mod config;
mod doctor;
mod error;
mod models;
mod secrets;

use std::process::ExitCode;

use clap_complete::Shell;

use crate::cli::{Action, Invocation};
use crate::config::{Config, Overrides};
use crate::error::ClaudexError;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("claudex: {error}");
            ExitCode::from(error.exit_code())
        }
    }
}

fn run() -> Result<(), ClaudexError> {
    let invocation = Invocation::parse(std::env::args_os().skip(1))?;

    match invocation.action {
        Action::Help => {
            cli::command().print_help()?;
            println!();
        }
        Action::Version => println!("claudex {}", env!("CARGO_PKG_VERSION")),
        Action::Completions => {
            clap_complete::generate(
                Shell::Zsh,
                &mut cli::command(),
                "claudex",
                &mut std::io::stdout(),
            );
        }
        Action::Models => {
            let config = Config::load(Overrides::from_env()?)?;
            models::print(&config)?;
        }
        Action::Validate => {
            let config = Config::load(Overrides::from_env()?)?;
            let _secret = config.load_secret()?;
            claude::resolve(&config)?;
            println!("configuration valid");
        }
        Action::Doctor { live } => {
            let config = Config::load(Overrides::from_env()?)?;
            doctor::run(&config, live)?;
        }
        Action::Launch {
            alias,
            proxy_model,
            context_window,
            claude_args,
        } => {
            let config = Config::load(Overrides::from_env()?)?;
            let model = models::resolve(
                &config,
                alias.as_deref(),
                proxy_model.as_deref(),
                context_window,
            )?;
            claude::launch(&config, &model, &claude_args)?;
        }
    }

    Ok(())
}
