mod support;

use predicates::prelude::*;
use support::Fixture;

#[test]
fn no_arguments_launches_the_configured_default_model() {
    let fixture = Fixture::new();

    fixture
        .command()
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "argv[1]=--model\nargv[2]=provider-fable",
        ))
        .stdout(predicate::str::contains(
            "ANTHROPIC_BASE_URL=http://127.0.0.1:18317",
        ))
        .stdout(predicate::str::contains(
            "ANTHROPIC_CUSTOM_MODEL_OPTION_NAME=Custom provider model",
        ))
        .stdout(predicate::str::contains(
            "ANTHROPIC_CUSTOM_MODEL_OPTION_DESCRIPTION=Custom model routed through the test gateway",
        ))
        .stdout(predicate::str::contains(
            "ANTHROPIC_AUTH_TOKEN=<EXPECTED>",
        ));
}

#[test]
fn model_accepts_aliases_and_passes_one_resolved_model() {
    let fixture = Fixture::new();

    fixture
        .command()
        .args(["--model", "opus", "--resume", "session-123"])
        .assert()
        .success()
        .stdout(predicate::str::contains("argv_count=4"))
        .stdout(predicate::str::contains(
            "argv[1]=--model\nargv[2]=provider-opus\nargv[3]=--resume\nargv[4]=session-123",
        ));
}

#[test]
fn proxy_model_is_the_only_raw_model_path() {
    let fixture = Fixture::new();

    fixture
        .command()
        .args(["--proxy-model", "provider/raw(high)", "-p", "hello"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "argv[1]=--model\nargv[2]=provider/raw(high)\nargv[3]=-p\nargv[4]=hello",
        ));
}

#[test]
fn equals_form_model_selectors_preserve_alias_enforcement() {
    let fixture = Fixture::new();

    fixture
        .command()
        .arg("--model=opus")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "argv[1]=--model\nargv[2]=provider-opus",
        ));
    fixture
        .command()
        .arg("--model=provider/raw")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("unknown model alias"));
    fixture
        .command()
        .arg("--proxy-model=provider/raw")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "argv[1]=--model\nargv[2]=provider/raw",
        ));
}

#[test]
fn model_and_proxy_model_are_mutually_exclusive() {
    let fixture = Fixture::new();

    fixture
        .command()
        .args(["--model", "opus", "--proxy-model", "raw"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used together"));
}

#[test]
fn unknown_alias_fails_before_claude_launches() {
    let fixture = Fixture::new();

    fixture
        .command()
        .args(["--model", "raw-model-id"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown model alias"))
        .stderr(predicate::str::contains("fable, haiku, opus, sonnet"));
}

#[test]
fn double_dash_preserves_values_that_look_like_claudex_commands() {
    let fixture = Fixture::new();

    fixture
        .command()
        .args(["--model", "sonnet", "--", "--model", "models", "doctor"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "argv[1]=--model\nargv[2]=provider-sonnet\nargv[3]=--\nargv[4]=--model\nargv[5]=models\nargv[6]=doctor",
        ));
}

#[test]
fn help_and_version_are_claudex_output() {
    let fixture = Fixture::new();

    fixture
        .command()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--proxy-model"));
    fixture
        .command()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "claudex {}",
            env!("CARGO_PKG_VERSION")
        )));
}

#[test]
fn models_and_config_validate_are_pipeable_commands() {
    let fixture = Fixture::new();

    fixture
        .command()
        .arg("models")
        .assert()
        .success()
        .stdout(predicate::str::contains("fable\tprovider-fable\tdefault"))
        .stdout(predicate::str::contains("haiku\tprovider-haiku"));
    fixture
        .command()
        .args(["config", "validate"])
        .assert()
        .success()
        .stdout("configuration valid\n");
}

#[test]
fn claude_flags_and_values_preserve_their_relative_order() {
    let fixture = Fixture::new();

    fixture
        .command()
        .args([
            "-p",
            "models",
            "--resume",
            "doctor",
            "--output-format",
            "json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "argv[3]=-p\nargv[4]=models\nargv[5]=--resume\nargv[6]=doctor\nargv[7]=--output-format\nargv[8]=json",
        ));
}

#[test]
fn launch_removes_ambient_provider_credentials() {
    let fixture = Fixture::new();

    fixture
        .command()
        .assert()
        .success()
        .stdout(predicate::str::contains("ANTHROPIC_API_KEY=<UNSET>"))
        .stdout(predicate::str::contains("CLAUDE_CODE_OAUTH_TOKEN=<UNSET>"))
        .stdout(predicate::str::contains(
            "CLAUDE_CODE_OAUTH_REFRESH_TOKEN=<UNSET>",
        ))
        .stdout(predicate::str::contains("CLAUDE_CODE_OAUTH_SCOPES=<UNSET>"))
        .stdout(predicate::str::contains("CLAUDE_CODE_USE_BEDROCK=<UNSET>"))
        .stdout(predicate::str::contains("CLAUDE_CODE_USE_VERTEX=<UNSET>"))
        .stdout(predicate::str::contains("CLAUDE_CODE_USE_FOUNDRY=<UNSET>"))
        .stdout(predicate::str::contains("CLAUDE_CODE_USE_MANTLE=<UNSET>"))
        .stdout(predicate::str::contains(
            "CLAUDE_CODE_USE_ANTHROPIC_AWS=<UNSET>",
        ))
        .stdout(predicate::str::contains("must-be-removed").not());
}

#[test]
fn absent_custom_picker_removes_ambient_picker_values() {
    let fixture = Fixture::new();
    let source = std::fs::read_to_string(&fixture.config).expect("read fixture config");
    let start = source.find("[custom_model]").expect("custom section");
    let end = source.find("[claude]").expect("Claude section");
    let without_custom = format!("{}{}", &source[..start], &source[end..]);
    std::fs::write(&fixture.config, without_custom).expect("remove custom picker config");

    fixture
        .command()
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "ANTHROPIC_CUSTOM_MODEL_OPTION=<UNSET>",
        ))
        .stdout(predicate::str::contains(
            "ANTHROPIC_CUSTOM_MODEL_OPTION_NAME=<UNSET>",
        ))
        .stdout(predicate::str::contains(
            "ANTHROPIC_CUSTOM_MODEL_OPTION_DESCRIPTION=<UNSET>",
        ))
        .stdout(predicate::str::contains("must-be-replaced").not());
}

#[test]
fn exact_child_environment_comes_from_configuration() {
    let fixture = Fixture::new();

    fixture
        .command()
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "ANTHROPIC_DEFAULT_FABLE_MODEL=provider-fable",
        ))
        .stdout(predicate::str::contains(
            "ANTHROPIC_DEFAULT_OPUS_MODEL=provider-opus",
        ))
        .stdout(predicate::str::contains(
            "ANTHROPIC_DEFAULT_SONNET_MODEL=provider-sonnet",
        ))
        .stdout(predicate::str::contains(
            "ANTHROPIC_DEFAULT_HAIKU_MODEL=provider-haiku",
        ))
        .stdout(predicate::str::contains(
            "ANTHROPIC_CUSTOM_MODEL_OPTION=provider-custom",
        ))
        .stdout(predicate::str::contains(
            "CLAUDE_CODE_SUBAGENT_MODEL=inherit",
        ))
        .stdout(predicate::str::contains(
            "CLAUDE_CODE_ALWAYS_ENABLE_EFFORT=1",
        ))
        .stdout(predicate::str::contains(
            "CLAUDE_CODE_MAX_TOOL_USE_CONCURRENCY=3",
        ))
        .stdout(predicate::str::contains("ENABLE_TOOL_SEARCH=false"));
}

#[test]
fn ordinary_environment_is_inherited_and_auth_token_is_replaced() {
    let fixture = Fixture::new();

    fixture
        .command()
        .assert()
        .success()
        .stdout(predicate::str::contains("CLAUDEX_ORDINARY_ENV=inherited"))
        .stdout(predicate::str::contains("ANTHROPIC_AUTH_TOKEN=<EXPECTED>"))
        .stdout(predicate::str::contains("must-be-replaced").not())
        .stderr(predicate::str::contains("must-be-replaced").not());
}

#[test]
fn explicit_selector_overrides_the_environment_default() {
    let fixture = Fixture::new();

    fixture
        .command()
        .env("CLAUDEX_DEFAULT_MODEL", "haiku")
        .args(["--model", "opus"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "argv[1]=--model\nargv[2]=provider-opus",
        ));
}
