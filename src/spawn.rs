use anyhow::{anyhow, bail, Result};
use std::{ffi::OsStr, fmt::Display, process::Stdio};
use tempfile::TempDir;
use tokio::io::AsyncWriteExt;

pub struct ContextualSpawn {
    pub workspace_dir: TempDir,
}

impl ContextualSpawn {
    pub async fn spawn<I, S>(&self, cmd: S, args: I) -> Result<()>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr> + Display,
    {
        self.spawn_with_stdin(cmd, args, None).await
    }

    pub async fn spawn_with_stdin<I, S>(
        &self,
        cmd: S,
        args: I,
        stdin: Option<Vec<u8>>,
    ) -> Result<()>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr> + Display,
    {
        let mut child = tokio::process::Command::new(&cmd)
            .current_dir(&self.workspace_dir.path())
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        if let Some(stdin_buffer) = stdin {
            let mut stdin_handle = child
                .stdin
                .take()
                .ok_or_else(|| anyhow!("Failed to access stdin handle."))?;
            stdin_handle.write_all(&stdin_buffer).await?;
            stdin_handle.flush().await?;
            stdin_handle.shutdown().await?;
        }

        let output = child.wait_with_output().await?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!("cmd {} failed:\n{}", cmd, stderr);
            bail!("{} failed\n{}", cmd, stderr)
        }
    }
}
