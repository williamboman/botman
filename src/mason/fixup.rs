use crate::github::action_parser::AuthorizedAction;
use anyhow::Result;

use rocket::http::Status;
use std::fmt::Display;

use super::{workspace::Workspace, MasonCommand};

async fn make_generate(workspace: &Workspace) -> Result<()> {
    println!("Generating code...");
    workspace.spawn("make", ["generate"]).await
}

async fn stylua(workspace: &Workspace) -> Result<()> {
    println!("Running stylua...");
    workspace.spawn("stylua", ["."]).await
}

pub(super) async fn run(
    action: &AuthorizedAction<MasonCommand>,
) -> Result<Box<dyn Display + Send>, (Status, anyhow::Error)> {
    let workspace = Workspace::create(&action).await?;

    async {
        workspace.merge_with_base().await?;
        make_generate(&workspace).await?;
        stylua(&workspace).await?;
        let _ = workspace.commit("fixup").await;
        workspace.push().await?;
        Ok::<(), anyhow::Error>(())
    }
    .await
    .map_err(|err| (Status::InternalServerError, err))?;

    Ok(Box::new(format!(
        "Successfully ran mason fixup in {:?}",
        workspace
    )))
}
