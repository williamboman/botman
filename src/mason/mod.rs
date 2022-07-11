use std::fmt::Display;

use crate::github::{
    action_parser::*,
    data::{GitHubIssueComment, GitHubIssueCommentAction},
};
use anyhow::{anyhow, Result};
use rocket::http::Status;

mod generate;

#[derive(Debug)]
pub enum MasonCommand {
    Generate,
}

impl TryFrom<RawCommand> for MasonCommand {
    type Error = anyhow::Error;

    fn try_from(value: RawCommand) -> Result<Self, Self::Error> {
        match value.raw_command.as_str() {
            "generate" => Ok(MasonCommand::Generate),
            s => Err(anyhow!("{} is not a valid mason command.", s)),
        }
    }
}

impl AuthorizedAction<MasonCommand> {
    async fn execute(&self) -> Result<Box<dyn Display>, (Status, anyhow::Error)> {
        match self.action.command {
            MasonCommand::Generate => generate::run(self).await,
        }
    }
}

#[post("/v1/mason/issue-comment", format = "json", data = "<event>")]
pub async fn index(event: GitHubIssueComment) -> Status {
    match event.action {
        GitHubIssueCommentAction::Created | GitHubIssueCommentAction::Updated => match event
            .try_into()
        {
            Ok(action @ AuthorizedAction::<MasonCommand> { .. }) => match action.execute().await {
                Ok(result) => {
                    println!("{}", result);
                    Status::NoContent
                }
                Err((status, err)) => {
                    eprintln!("ERROR: {:?}", err);
                    status
                }
            },
            Err(err) => {
                println!("Failed to parse action from comment: {:?}", err);
                Status::NoContent
            }
        },
        GitHubIssueCommentAction::Deleted => Status::NoContent,
    }
}
