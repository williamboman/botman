use anyhow::{anyhow, Result};
use std::{ffi::OsStr, fmt::Display, path::Path, process::Stdio};

pub struct ContextualSpawn<'a> {
    pub cwd: &'a Path,
}

impl<'a> ContextualSpawn<'a> {
    pub async fn spawn<I, S>(&self, cmd: S, args: I) -> Result<()>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr> + Display,
    {
        let output = tokio::process::Command::new(&cmd)
            .current_dir(&self.cwd)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        if output.status.success() {
            Ok(())
        } else {
            Err(anyhow!(
                "{} failed\n{}",
                cmd,
                String::from_utf8_lossy(&output.stderr)
            ))
        }
    }
}
