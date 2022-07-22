use crate::github::action_parser::AuthorizedAction;
use anyhow::Result;

use rocket::http::Status;
use std::fmt::Display;

use super::{workspace::Workspace, MasonCommand};

async fn make_generate(workspace: &Workspace) -> Result<()> {
    println!("Generating code...");
    workspace.spawner.spawn("make", ["generate"]).await
}

async fn stylua(workspace: &Workspace) -> Result<()> {
    println!("Running stylua...");
    workspace.spawner.spawn("stylua", ["."]).await
}

pub(super) async fn run(
    action: &AuthorizedAction<MasonCommand>,
) -> Result<Box<dyn Display + Send>, (Status, anyhow::Error)> {
    let workspace = super::workspace::setup(&action).await?;

    async {
        make_generate(&workspace).await?;
        stylua(&workspace).await?;
        workspace.commit_and_push("fixup").await?;
        Ok::<(), anyhow::Error>(())
    }
    .await
    .map_err(|err| (Status::InternalServerError, err))?;

    Ok(Box::new(format!(
        "Successfully ran mason fixup in {}",
        workspace
    )))
}
