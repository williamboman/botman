use crate::{github::action::parser::AuthorizedAction, workspace::Workspace};
use anyhow::Result;
use rocket::http::Status;
use std::fmt::Display;

use super::{common::GitApplyPatch, parser::RawCommand};

async fn apply_patch(workspace: &Workspace, patch: &GitApplyPatch) -> Result<()> {
    println!("Applying patch\n{}", patch.patch);
    workspace
        .spawn_with_stdin(
            "git",
            ["apply", "--", "-"],
            Some(patch.patch.to_owned().into_bytes()),
        )
        .await?;
    Ok(())
}

pub async fn run<Command>(
    action: &AuthorizedAction<Command>,
    patch: &GitApplyPatch,
) -> Result<Box<dyn Display + Send>, (Status, anyhow::Error)>
where
    Command: TryFrom<RawCommand, Error = anyhow::Error>,
{
    let workspace = Workspace::create(action).await?;

    async {
        apply_patch(&workspace, patch).await?;
        workspace.commit("apply diff").await?;
        workspace.push().await?;
        Ok::<(), anyhow::Error>(())
    }
    .await
    .map_err(|err| (Status::InternalServerError, err))?;

    Ok(Box::new(format!(
        "Successfully ran mason apply in {:?}",
        workspace
    )))
}
