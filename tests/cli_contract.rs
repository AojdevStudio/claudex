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
        .stdout(predicate::str::contains("ANTHROPIC_AUTH_TOKEN=<SET>"));
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
        .stdout(predicate::str::contains("claudex 0.1.0"));
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
        .stdout(predicate::str::contains("CLAUDE_CODE_USE_BEDROCK=<UNSET>"))
        .stdout(predicate::str::contains("CLAUDE_CODE_USE_VERTEX=<UNSET>"))
        .stdout(predicate::str::contains("CLAUDE_CODE_USE_FOUNDRY=<UNSET>"))
        .stdout(predicate::str::contains(
            "CLAUDE_CODE_USE_ANTHROPIC_AWS=<UNSET>",
        ))
        .stdout(predicate::str::contains("must-be-removed").not());
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
