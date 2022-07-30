use crate::github::action_parser::AuthorizedAction;
use anyhow::Result;
use rocket::http::Status;
use std::fmt::Display;

use super::{workspace::Workspace, GitApplyPatch, MasonCommand};

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

pub(super) async fn run(
    action: &AuthorizedAction<MasonCommand>,
    patch: &GitApplyPatch,
) -> Result<Box<dyn Display + Send>, (Status, anyhow::Error)> {
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
