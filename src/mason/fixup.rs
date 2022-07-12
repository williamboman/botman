use crate::{
    github::{
        action_parser::AuthorizedAction,
        client,
        data::{GitHubReaction, GitHubRef},
    },
    spawn::ContextualSpawn,
    GITHUB_LOGIN, GITHUB_PAT,
};
use anyhow::{anyhow, Result};
use rocket::http::Status;
use std::fmt::Display;

use super::MasonCommand;

async fn clone_repo(spawner: &ContextualSpawn<'_>, github_ref: &GitHubRef) -> Result<()> {
    println!("Cloning {:?}…", github_ref.repo.full_name);
    spawner
        .spawn(
            "git",
            [
                "clone",
                "--",
                github_ref
                    .repo
                    .as_git_url(
                        format!("{}:{}", GITHUB_LOGIN.as_str(), GITHUB_PAT.as_str()).as_str(),
                    )
                    .as_str(),
                ".",
            ],
        )
        .await
}

async fn checkout_ref(spawner: &ContextualSpawn<'_>, github_ref: &GitHubRef) -> Result<()> {
    println!("Checking out {}", github_ref.r#ref);
    spawner
        .spawn("git", ["checkout", github_ref.r#ref.as_str()])
        .await
}

async fn make_generate(spawner: &ContextualSpawn<'_>) -> Result<()> {
    println!("Generating code...");
    spawner.spawn("make", ["generate"]).await
}

async fn stylua(spawner: &ContextualSpawn<'_>) -> Result<()> {
    println!("Running stylua...");
    spawner.spawn("stylua", ["."]).await
}

async fn commit_and_push(spawner: &ContextualSpawn<'_>) -> Result<()> {
    println!("Commiting changes and pushing...");
    spawner.spawn("git", ["add", "."]).await?;
    spawner
        .spawn("git", ["commit", "-m", "fixup"])
        .await?;
    spawner.spawn("git", ["push"]).await?;
    Ok(())
}

pub async fn run(
    action: &AuthorizedAction<MasonCommand>,
) -> Result<Box<dyn Display + Send>, (Status, anyhow::Error)> {
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

    let tmp_dir = tempfile::tempdir().map_err(|err| (Status::InternalServerError, anyhow!(err)))?;
    let spawner = ContextualSpawn {
        cwd: tmp_dir.path(),
    };

    let runner = async {
        clone_repo(&spawner, &pr.head).await?;
        checkout_ref(&spawner, &pr.head).await?;
        make_generate(&spawner).await?;
        stylua(&spawner).await?;
        commit_and_push(&spawner).await?;
        Ok::<(), anyhow::Error>(())
    };

    runner
        .await
        .map_err(|err| (Status::InternalServerError, err))?;

    Ok(Box::new(format!(
        "Successfully ran mason generate in {:?}",
        pr.head
    )))
}
