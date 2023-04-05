use crate::{
    github::{
        action::parser::{AuthorizedAction, RawCommand},
        client,
        data::{GitHubReaction, GitHubRef},
    },
    GITHUB_PAT,
};
use anyhow::{anyhow, bail, Result};
use rocket::http::Status;
use std::{ffi::OsStr, fmt::Display, process::Stdio};
use tempfile::TempDir;
use tokio::io::AsyncWriteExt;

#[derive(Debug)]
pub struct Workspace {
    pub workdir: TempDir,
    pub base: GitHubRef,
    pub head: GitHubRef,
}

impl Workspace {
    pub async fn create<Command>(
        action: &AuthorizedAction<Command>,
    ) -> Result<Workspace, (Status, anyhow::Error)>
    where
        Command: TryFrom<RawCommand, Error = anyhow::Error>,
    {
        let pr = action
            .context
            .get_pull_request()
            .await
            .map_err(|err| (Status::InternalServerError, err))?
            .ok_or_else(|| {
                (
                    Status::NoContent,
                    anyhow!(
                        "Umm… there's no pull request associated with {:?}",
                        action.context
                    ),
                )
            })?;

        client::create_issue_comment_reaction(
            action.context.get_repo(),
            action.context.get_trigger(),
            &GitHubReaction::PlusOne,
        )
        .await
        .map_err(|err| (Status::ServiceUnavailable, err))?;

        client::minimize_comment(action.context.get_trigger())
            .await
            .map_err(|err| (Status::ServiceUnavailable, err))?;

        let base = pr.base;
        let head = pr.head;

        let workspace = Workspace {
            workdir: tempfile::tempdir()
                .map_err(|err| (Status::InternalServerError, anyhow!(err)))?,
            head,
            base,
        };

        async {
            workspace.clone_repo().await?;
            workspace.checkout_ref().await?;
            Ok::<(), anyhow::Error>(())
        }
        .await
        .map_err(|err| (Status::InternalServerError, err))?;
        Ok(workspace)
    }

    pub async fn commit(&self, commit_msg: &str) -> Result<()> {
        println!("Committing changes…");
        self.spawn("git", ["add", "."]).await?;
        self.spawn("git", ["commit", "-m", commit_msg]).await?;
        Ok(())
    }

    pub async fn push(&self) -> Result<()> {
        println!("Pushing changes…");
        self.spawn("git", ["push"]).await?;
        Ok(())
    }

    pub async fn merge_with_base(&self) -> Result<()> {
        println!("Merging with {}", self.base.r#ref);
        self.spawn("git", ["fetch", "upstream", &self.base.r#ref])
            .await?;
        self.spawn(
            "git",
            [
                "merge",
                "--no-edit",
                "-X",
                "ours",
                "-m",
                "Merge upstream",
                format!("upstream/{}", self.base.r#ref).as_str(),
            ],
        )
        .await?;
        Ok(())
    }

    async fn clone_repo(&self) -> Result<()> {
        println!("Cloning {:?}…", self.head.repo.full_name);
        self.spawn(
            "git",
            [
                "clone",
                "-c",
                format!(
                    "http.https://github.com/.extraheader=AUTHORIZATION: basic {}",
                    base64::encode(format!("x-access-token:{}", GITHUB_PAT.as_str()))
                )
                .as_str(),
                "-c",
                "checkout.defaultRemote=origin",
                "--",
                self.head.repo.as_git_url().as_str(),
                ".",
            ],
        )
        .await?;
        self.spawn(
            "git",
            [
                "remote",
                "add",
                "upstream",
                self.base.repo.as_git_url().as_str(),
            ],
        )
        .await?;
        self.spawn("git", ["fetch", "upstream"]).await?;
        Ok(())
    }

    async fn checkout_ref(&self) -> Result<()> {
        println!("Checking out {}", self.head.r#ref);
        self.spawn("git", ["checkout", self.head.r#ref.as_str()])
            .await
    }

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
            .current_dir(&self.workdir.path())
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
