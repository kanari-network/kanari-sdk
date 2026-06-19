use std::io;
use std::path::Path;

pub fn secure_path(path: &Path, is_dir: bool) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = if is_dir { 0o700 } else { 0o600 };
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))?;
    }

    #[cfg(windows)]
    {
        use std::process::Command;

        let identity_output = Command::new("whoami").output()?;
        if !identity_output.status.success() {
            return Err(io::Error::other(
                "failed to resolve current Windows identity",
            ));
        }
        let identity = String::from_utf8(identity_output.stdout)
            .map_err(|_| io::Error::other("Windows identity is not UTF-8"))?;
        let identity = identity.trim();
        let grant = if is_dir {
            format!("{identity}:(OI)(CI)F")
        } else {
            format!("{identity}:F")
        };

        let status = Command::new("icacls")
            .arg(path)
            .args(["/inheritance:r", "/grant:r", &grant])
            .status()?;
        if !status.success() {
            return Err(io::Error::other(format!(
                "failed to restrict ACL for {}",
                path.display()
            )));
        }
    }

    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn secures_temporary_directory_and_file() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("kanari-auth-permissions-{unique}"));
        let file = dir.join("auth.db");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&file, b"test").unwrap();

        secure_path(&dir, true).unwrap();
        secure_path(&file, false).unwrap();

        std::fs::remove_file(file).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }
}
