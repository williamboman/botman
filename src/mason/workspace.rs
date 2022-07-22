use crate::{
    github::{
        action_parser::AuthorizedAction,
        client,
        data::{GitHubReaction, GitHubRef},
    },
    spawn::ContextualSpawn,
    GITHUB_PAT,
};
use anyhow::{anyhow, Result};
use rocket::http::Status;
use std::fmt::Display;

use super::MasonCommand;

pub(super) async fn clone_repo(spawner: &ContextualSpawn, github_ref: &GitHubRef) -> Result<()> {
    println!("Cloning {:?}…", github_ref.repo.full_name);
    spawner
        .spawn(
            "git",
            [
                "clone",
                "-c",
                format!(
                    "http.https://github.com/.extraheader=AUTHORIZATION: basic {}",
                    base64::encode(format!("x-access-token:{}", GITHUB_PAT.as_str()))
                )
                .as_str(),
                "--",
                github_ref.repo.as_git_url().as_str(),
                ".",
            ],
        )
        .await
}

pub(super) async fn checkout_ref(spawner: &ContextualSpawn, github_ref: &GitHubRef) -> Result<()> {
    println!("Checking out {}", github_ref.r#ref);
    spawner
        .spawn("git", ["checkout", github_ref.r#ref.as_str()])
        .await
}

pub(super) async fn merge_with_base(spawner: &ContextualSpawn, base: &GitHubRef) -> Result<()> {
    println!("Merging with {}", base.r#ref);
    spawner
        .spawn(
            "git",
            ["remote", "add", "upstream", base.repo.as_git_url().as_str()],
        )
        .await?;
    spawner
        .spawn("git", ["fetch", "upstream", &base.r#ref])
        .await?;
    spawner
        .spawn(
            "git",
            [
                "merge",
                "--no-edit",
                "-m",
                "Merge upstream",
                format!("upstream/{}", base.r#ref).as_str(),
            ],
        )
        .await?;
    Ok(())
}

pub(super) struct Workspace {
    pub spawner: ContextualSpawn,
    pub base: GitHubRef,
    pub head: GitHubRef,
}

impl Workspace {
    pub async fn commit_and_push(&self, commit_msg: &str) -> Result<()> {
        println!("Commiting changes and pushing...");
        self.spawner.spawn("git", ["add", "."]).await?;
        self.spawner
            .spawn("git", ["commit", "-m", commit_msg])
            .await?;
        self.spawner.spawn("git", ["push"]).await?;
        Ok(())
    }
}

impl Display for Workspace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Workspace {{ base: {:?}, head: {:?}, workspace_dir: {:?} }}",
            self.base,
            self.head,
            self.spawner.workspace_dir.path()
        )
    }
}

pub(super) async fn setup(
    action: &AuthorizedAction<MasonCommand>,
) -> Result<Workspace, (Status, anyhow::Error)> {
    let pr = action
        .context
        .get_pull_request()
        .await
        .map_err(|err| (Status::InternalServerError, err))?
        .ok_or_else(|| {
            (
                Status::NoContent,
                anyhow!(
                    "Umm... there's no pull request associated with {:?}",
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

    let base = pr.base;
    let head = pr.head;

    let workspace_dir =
        tempfile::tempdir().map_err(|err| (Status::InternalServerError, anyhow!(err)))?;
    let spawner = ContextualSpawn { workspace_dir };

    async {
        clone_repo(&spawner, &head).await?;
        checkout_ref(&spawner, &head).await?;
        merge_with_base(&spawner, &base).await?;
        Ok::<(), anyhow::Error>(())
    }
    .await
    .map_err(|err| (Status::InternalServerError, err))?;
    Ok(Workspace {
        spawner,
        head,
        base,
    })
}
