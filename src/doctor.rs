use std::env;
use std::ffi::OsString;
use std::time::Duration;

use crate::claude;
use crate::config::Config;
use crate::error::ClaudexError;
use crate::models;

const LIVE_PROMPT: &str = "Return exactly CLAUDEX_DOCTOR_OK";
const LIVE_EXPECTED: &str = "CLAUDEX_DOCTOR_OK";

pub fn run(config: &Config, live: bool) -> Result<(), ClaudexError> {
    println!("PASS config");
    let secret = config.load_secret()?;
    println!("PASS key-file");
    let _claude = claude::resolve(config)?;
    println!("PASS claude-path");

    let endpoint = format!("{}/v1/models", config.proxy.base_url.trim_end_matches('/'));
    let timeout = timeout()?;
    let response = reqwest::blocking::Client::builder()
        .connect_timeout(timeout.min(Duration::from_secs(3)))
        .timeout(timeout)
        .build()
        .map_err(|error| ClaudexError::DoctorNetwork(error.to_string()))?
        .get(endpoint)
        .bearer_auth(secret.expose())
        .send()
        .map_err(|error| {
            if error.is_timeout() {
                ClaudexError::DoctorNetwork("request timed out".into())
            } else {
                ClaudexError::DoctorNetwork(error.to_string())
            }
        })?;
    if !response.status().is_success() {
        return Err(ClaudexError::DoctorNetwork(format!(
            "model listing returned HTTP {}",
            response.status().as_u16()
        )));
    }
    println!("PASS proxy-models");

    if live {
        let model = models::resolve(config, None, None)?;
        let output = claude::run_live(
            config,
            &model,
            &[
                OsString::from("-p"),
                OsString::from("--no-session-persistence"),
                OsString::from(LIVE_PROMPT),
            ],
        )?;
        if !output.status.success() {
            return Err(ClaudexError::DoctorLive(format!(
                "Claude Code exited with {}",
                output.status
            )));
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.trim() != LIVE_EXPECTED {
            return Err(ClaudexError::DoctorLive(format!(
                "expected {LIVE_EXPECTED}, received a different response"
            )));
        }
        println!("PASS live-inference");
    }

    Ok(())
}

fn timeout() -> Result<Duration, ClaudexError> {
    let Some(value) = env::var_os("CLAUDEX_DOCTOR_TIMEOUT_MS") else {
        return Ok(Duration::from_secs(8));
    };
    let milliseconds = value
        .to_str()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .ok_or_else(|| {
            ClaudexError::Config("CLAUDEX_DOCTOR_TIMEOUT_MS must be a positive integer".into())
        })?;
    Ok(Duration::from_millis(milliseconds))
}
