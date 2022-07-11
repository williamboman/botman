use std::fmt::Display;

use crate::github::{
    action_parser::*,
    client,
    data::{GitHubIssueComment, GitHubIssueCommentAction, GitHubReaction},
};
use anyhow::{anyhow, Result};
use rocket::http::Status;

mod fixup;

#[derive(Debug)]
pub enum MasonCommand {
    Fixup,
}

impl TryFrom<RawCommand> for MasonCommand {
    type Error = anyhow::Error;

    fn try_from(value: RawCommand) -> Result<Self, Self::Error> {
        match value.raw_command.as_str() {
            "fixup" => Ok(MasonCommand::Fixup),
            s => Err(anyhow!("{} is not a valid mason command.", s)),
        }
    }
}

impl AuthorizedAction<MasonCommand> {
    async fn execute(&self) -> Result<Box<dyn Display + Send>, (Status, anyhow::Error)> {
        match self.action.command {
            MasonCommand::Fixup => fixup::run(self).await,
        }
    }
}

#[post("/v1/mason/issue-comment", format = "json", data = "<event>")]
pub async fn index(event: GitHubIssueComment) -> Status {
    let repo = event.repository.clone();
    let comment = event.comment.clone();
    match event.action {
        GitHubIssueCommentAction::Created => match event.try_into() {
            Ok(action @ AuthorizedAction::<MasonCommand> { .. }) => match action.execute().await {
                Ok(result) => {
                    println!("{}", result);
                    Status::NoContent
                }
                Err((status, err)) => {
                    let _ = client::create_issue_comment_reaction(
                        &repo,
                        &comment,
                        &GitHubReaction::MinusOne,
                    )
                    .await;
                    eprintln!("ERROR: {:?}", err);
                    status
                }
            },
            Err(err) => {
                println!("Failed to parse action from comment: {:?}", err);
                let _ = client::create_issue_comment_reaction(
                    &repo,
                    &comment,
                    &GitHubReaction::MinusOne,
                )
                .await;
                Status::NoContent
            }
        },
        GitHubIssueCommentAction::Updated | GitHubIssueCommentAction::Deleted => Status::NoContent,
    }
}
