use crate::{github::action::parser::AuthorizedAction, workspace::Workspace};
use anyhow::Result;

use rocket::http::Status;
use std::fmt::Display;

use super::parser::RawCommand;

pub async fn run<Command>(
    action: &AuthorizedAction<Command>,
) -> Result<Box<dyn Display + Send>, (Status, anyhow::Error)>
where
    Command: TryFrom<RawCommand, Error = anyhow::Error>,
{
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
        "Successfully ran merge-base in {:?}",
        workspace
    )))
}
