use crate::{github::action::parser::AuthorizedAction, workspace::Workspace};
use anyhow::Result;

use rocket::http::Status;
use std::fmt::Display;

use super::MasonCommand;

pub(super) async fn run(
    action: &AuthorizedAction<MasonCommand>,
) -> Result<Box<dyn Display + Send>, (Status, anyhow::Error)> {
    let workspace = Workspace::create(&action).await?;

    workspace
        .merge_with_base()
        .await
        .map_err(|err| (Status::InternalServerError, err))?;

    workspace
        .push()
        .await
        .map_err(|err| (Status::InternalServerError, err))?;

    Ok(Box::new(format!(
        "Successfully ran mason merge-base in {:?}",
        workspace
    )))
}
