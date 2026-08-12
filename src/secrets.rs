use std::fmt;
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Component, Path, PathBuf};

use crate::error::ClaudexError;

pub struct Secret(String);

impl Secret {
    pub fn load(path: &Path) -> Result<Self, ClaudexError> {
        reject_symlinks(path)?;
        let metadata = fs::metadata(path).map_err(|error| {
            ClaudexError::Secret(format!("cannot inspect {}: {error}", path.display()))
        })?;
        let mode = metadata.permissions().mode() & 0o777;
        validate_file_policy(
            metadata.is_file(),
            metadata.uid(),
            unsafe { libc::geteuid() },
            mode,
        )
        .map_err(|message| ClaudexError::Secret(format!("{} {message}", path.display())))?;

        let mut bytes = fs::read(path).map_err(|error| {
            ClaudexError::Secret(format!("cannot read {}: {error}", path.display()))
        })?;
        if bytes.ends_with(b"\r\n") {
            bytes.truncate(bytes.len() - 2);
        } else if bytes.ends_with(b"\n") {
            bytes.truncate(bytes.len() - 1);
        }
        if bytes.is_empty() {
            return Err(ClaudexError::Secret("API key cannot be empty".into()));
        }
        if bytes.contains(&0) {
            return Err(ClaudexError::Secret(
                "API key cannot contain a NUL byte".into(),
            ));
        }
        if bytes.iter().any(|byte| matches!(byte, b'\n' | b'\r')) {
            return Err(ClaudexError::Secret(
                "API key must contain a single line".into(),
            ));
        }
        let value = String::from_utf8(bytes)
            .map_err(|_| ClaudexError::Secret("API key must be valid UTF-8".into()))?;
        Ok(Self(value))
    }

    pub fn expose(&self) -> &str {
        &self.0
    }
}

fn validate_file_policy(
    is_file: bool,
    owner_uid: u32,
    current_uid: u32,
    mode: u32,
) -> Result<(), &'static str> {
    if !is_file {
        return Err("is not a regular file");
    }
    if owner_uid != current_uid {
        return Err("is not owned by the current user");
    }
    if mode & 0o400 == 0 {
        return Err("is not owner-readable");
    }
    if mode & 0o077 != 0 {
        return Err("has group or world permissions; require mode 0600 or stricter");
    }
    Ok(())
}

impl fmt::Debug for Secret {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Secret(<REDACTED>)")
    }
}

impl fmt::Display for Secret {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("<REDACTED>")
    }
}

fn reject_symlinks(path: &Path) -> Result<(), ClaudexError> {
    let mut current = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(_) | Component::RootDir => current.push(component.as_os_str()),
            Component::CurDir => continue,
            Component::ParentDir | Component::Normal(_) => current.push(component.as_os_str()),
        }
        if fs::symlink_metadata(&current).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
            return Err(ClaudexError::Secret(format!(
                "{} contains a symbolic link",
                path.display()
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{Secret, validate_file_policy};

    #[test]
    fn debug_and_display_are_redacted() {
        let secret = Secret("never-print-this".into());
        assert_eq!(format!("{secret:?}"), "Secret(<REDACTED>)");
        assert_eq!(format!("{secret}"), "<REDACTED>");
    }

    #[test]
    fn file_policy_rejects_a_different_owner() {
        assert_eq!(
            validate_file_policy(true, 502, 501, 0o600),
            Err("is not owned by the current user")
        );
    }
}
