mod support;

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::fs::symlink;
use std::process::Command;

use predicates::prelude::*;
use support::Fixture;

#[test]
fn unknown_config_fields_are_rejected() {
    let fixture = Fixture::new();
    let mut config = fs::read_to_string(&fixture.config).expect("read config");
    config.push_str("\nunknown = true\n");
    fs::write(&fixture.config, config).expect("write invalid config");

    fixture
        .command()
        .args(["config", "validate"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown field"));
}

#[test]
fn environment_overrides_toml_without_compiling_machine_values() {
    let fixture = Fixture::new();

    fixture
        .command()
        .env("CLAUDEX_BASE_URL", "https://gateway.example.test")
        .env("CLAUDEX_DEFAULT_MODEL", "haiku")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "ANTHROPIC_BASE_URL=https://gateway.example.test",
        ))
        .stdout(predicate::str::contains(
            "argv[1]=--model\nargv[2]=provider-haiku",
        ));
}

#[test]
fn unsafe_key_permissions_fail_closed_without_revealing_the_value() {
    let fixture = Fixture::new();
    let key = fixture
        .config
        .parent()
        .expect("config parent")
        .join("api-key");
    fs::set_permissions(&key, fs::Permissions::from_mode(0o644)).expect("weaken key mode");

    fixture
        .command()
        .args(["config", "validate"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("group or world permissions"))
        .stderr(predicate::str::contains("fixture-key").not());
}

#[test]
fn multiline_keys_fail_closed_without_revealing_the_value() {
    let fixture = Fixture::new();
    let key = fixture
        .config
        .parent()
        .expect("config parent")
        .join("api-key");
    fs::write(&key, "fixture-key\nsecond-line\n").expect("write multiline key");

    fixture
        .command()
        .args(["config", "validate"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("single line"))
        .stderr(predicate::str::contains("fixture-key").not());
}

#[test]
fn missing_configuration_is_an_actionable_error() {
    let fixture = Fixture::new();

    fixture
        .command()
        .env("CLAUDEX_CONFIG", "/does/not/exist/config.toml")
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot read configuration"));
}

#[test]
fn api_key_file_override_takes_precedence() {
    let fixture = Fixture::new();
    let alternate_key = fixture
        .config
        .parent()
        .expect("config parent")
        .join("alternate-key");
    fs::write(&alternate_key, "alternate\n").expect("write alternate key");
    fs::set_permissions(&alternate_key, fs::Permissions::from_mode(0o600))
        .expect("secure alternate key");

    fixture
        .command()
        .env("CLAUDEX_API_KEY_FILE", &alternate_key)
        .args(["config", "validate"])
        .assert()
        .success();
}

#[test]
fn fixture_config_remains_a_valid_reference() {
    let fixture = Fixture::new();
    let replacement = fixture
        .config
        .parent()
        .expect("config parent")
        .join("replacement.toml");
    let key = fixture
        .config
        .parent()
        .expect("config parent")
        .join("api-key");
    let example = include_str!("../config.example.toml")
        .replace("~/.config/claudex/api-key", &key.to_string_lossy());
    fs::write(&replacement, example).expect("write example config");
    fixture
        .command()
        .env("CLAUDEX_CONFIG", replacement)
        .args(["config", "validate"])
        .assert()
        .success()
        .stdout("configuration valid\n");
}

#[test]
fn invalid_default_alias_and_unsupported_url_scheme_are_rejected() {
    let fixture = Fixture::new();
    let original = fs::read_to_string(&fixture.config).expect("read config");
    fs::write(
        &fixture.config,
        original.replace("model = \"fable\"", "model = \"unknown\""),
    )
    .expect("write invalid default");
    fixture
        .command()
        .args(["config", "validate"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("defaults.model"));

    fs::write(
        &fixture.config,
        original.replace(
            "http://127.0.0.1:18317",
            "file:///private/tmp/not-a-gateway",
        ),
    )
    .expect("write unsupported URL");
    fixture
        .command()
        .args(["config", "validate"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("http or https"));
}

#[test]
fn non_utf8_and_owner_unreadable_keys_are_rejected() {
    let fixture = Fixture::new();
    let key = fixture.key_path();
    fs::write(&key, [0xff, b'\n']).expect("write non-UTF-8 key");
    fixture
        .command()
        .args(["config", "validate"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("valid UTF-8"));

    fs::write(&key, "fixture-key\n").expect("restore key");
    fs::set_permissions(&key, fs::Permissions::from_mode(0o200)).expect("remove read bit");
    fixture
        .command()
        .args(["config", "validate"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("owner-readable"));
}

#[test]
fn empty_and_nul_keys_are_rejected() {
    let fixture = Fixture::new();
    let key = fixture.key_path();
    fs::write(&key, "\n").expect("write empty key");
    fixture
        .command()
        .args(["config", "validate"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("cannot be empty"));

    fs::write(&key, b"before\0after\n").expect("write NUL key");
    fixture
        .command()
        .args(["config", "validate"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("NUL"))
        .stderr(predicate::str::contains("before").not());
}

#[test]
fn one_terminal_crlf_is_trimmed() {
    let fixture = Fixture::new();
    fs::write(fixture.key_path(), "fixture-key\r\n").expect("write CRLF key");

    fixture
        .command()
        .args(["config", "validate"])
        .assert()
        .success();
}

#[test]
fn symlinked_key_and_parent_paths_are_rejected() {
    let fixture = Fixture::new();
    let real_key = fixture.key_path();
    let linked_key = real_key.with_file_name("linked-key");
    symlink(&real_key, &linked_key).expect("create key symlink");
    fixture
        .command()
        .env("CLAUDEX_API_KEY_FILE", &linked_key)
        .args(["config", "validate"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("symbolic link"));

    let parent_link = real_key.with_file_name("linked-parent");
    symlink(real_key.parent().expect("key parent"), &parent_link).expect("create parent symlink");
    fixture
        .command()
        .env("CLAUDEX_API_KEY_FILE", parent_link.join("api-key"))
        .args(["config", "validate"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("symbolic link"));
}

#[test]
fn missing_and_recursive_claude_paths_are_rejected() {
    let fixture = Fixture::new();
    fixture
        .command()
        .env("CLAUDEX_CLAUDE_PATH", "/does/not/exist/claude")
        .args(["config", "validate"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("Claude Code is not available"));

    let binary = assert_cmd::cargo::cargo_bin!("claudex");
    fixture
        .command()
        .env("CLAUDEX_CLAUDE_PATH", binary)
        .args(["config", "validate"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("points back to claudex"));
}

#[test]
fn key_path_must_be_a_regular_file() {
    let fixture = Fixture::new();
    let directory = fixture
        .config
        .parent()
        .expect("config parent")
        .join("key-directory");
    fs::create_dir(&directory).expect("create key directory");
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).expect("secure directory");

    fixture
        .command()
        .env("CLAUDEX_API_KEY_FILE", directory)
        .args(["config", "validate"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("not a regular file"));
}

#[test]
fn fifo_key_path_is_rejected_without_blocking() {
    let fixture = Fixture::new();
    let fifo = fixture
        .config
        .parent()
        .expect("config parent")
        .join("api-key-fifo");
    let status = Command::new("mkfifo")
        .arg(&fifo)
        .status()
        .expect("create FIFO");
    assert!(status.success(), "mkfifo must succeed");
    fs::set_permissions(&fifo, fs::Permissions::from_mode(0o600)).expect("secure FIFO");

    fixture
        .command()
        .env("CLAUDEX_API_KEY_FILE", fifo)
        .args(["config", "validate"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("not a regular file"));
}
