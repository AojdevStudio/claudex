use std::ffi::CString;
use std::fmt;
use std::fs::File;
use std::io::{self, Read};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Component, Path};

use crate::error::ClaudexError;

pub struct Secret(String);

impl Secret {
    pub fn load(path: &Path) -> Result<Self, ClaudexError> {
        let mut file = open_without_symlinks(path)?;
        let metadata = file.metadata().map_err(|error| {
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

        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).map_err(|error| {
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

fn open_without_symlinks(path: &Path) -> Result<File, ClaudexError> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| {
                ClaudexError::Secret(format!("cannot resolve current directory: {error}"))
            })?
            .join(path)
    };
    let parts: Vec<_> = absolute
        .components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(value.to_owned()),
            Component::ParentDir => Some("..".into()),
            Component::Prefix(_) | Component::RootDir | Component::CurDir => None,
        })
        .collect();
    if parts.is_empty() {
        return Err(ClaudexError::Secret(format!(
            "{} is not a regular file",
            path.display()
        )));
    }

    let mut directory = File::open("/").map_err(|error| {
        ClaudexError::Secret(format!("cannot safely open {}: {error}", path.display()))
    })?;
    for (index, part) in parts.iter().enumerate() {
        let name = CString::new(part.as_bytes())
            .map_err(|_| ClaudexError::Secret(format!("{} contains a NUL byte", path.display())))?;
        let is_final = index + 1 == parts.len();
        let flags = libc::O_RDONLY
            | libc::O_CLOEXEC
            | libc::O_NOFOLLOW
            | if is_final { 0 } else { libc::O_DIRECTORY };
        // SAFETY: `directory` owns a live directory fd and `name` is a NUL-terminated C string.
        let fd = unsafe { libc::openat(directory.as_raw_fd(), name.as_ptr(), flags) };
        if fd < 0 {
            let error = io::Error::last_os_error();
            let detail = match error.raw_os_error() {
                Some(libc::ELOOP | libc::ENOTDIR) => {
                    "contains a symbolic link or non-directory path component".to_owned()
                }
                Some(libc::EACCES) if is_final => "is not owner-readable".to_owned(),
                _ => error.to_string(),
            };
            return Err(ClaudexError::Secret(format!(
                "cannot safely open {}: {detail}",
                path.display()
            )));
        }
        // SAFETY: `openat` returned this owned fd and it has not been wrapped or closed elsewhere.
        let opened = unsafe { File::from_raw_fd(fd) };
        if is_final {
            return Ok(opened);
        }
        directory = opened;
    }
    unreachable!("non-empty path has a final component")
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
