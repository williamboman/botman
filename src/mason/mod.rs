use std::fmt::Display;

use crate::github::{
    action_parser::*,
    client,
    data::{GitHubIssueComment, GitHubIssueCommentAction, GitHubReaction},
};
use anyhow::{anyhow, bail, Result};
use rocket::http::Status;

mod apply;
mod fixup;
mod workspace;

#[derive(Debug)]
struct GitApplyPatch {
    patch: String,
}

impl TryFrom<String> for GitApplyPatch {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self> {
        let massaged_value = value
            .trim_start_matches(char::is_whitespace)
            .replace("\r", "");
        let lines = massaged_value.split_inclusive("\n");
        let mut lines_iter = lines.clone().into_iter();
        let header = lines_iter.next().ok_or_else(|| anyhow!("No header."))?;
        if !header.starts_with("```diff") {
            bail!("Not a diff.")
        }
        let mut patch = String::new();
        for line in lines_iter {
            match line {
                "```" => break,
                _ => patch.push_str(line),
            }
        }
        Ok(Self { patch })
    }
}

#[derive(Debug)]
enum MasonCommand {
    Fixup,
    Apply(GitApplyPatch),
}

impl TryFrom<RawCommand> for MasonCommand {
    type Error = anyhow::Error;

    fn try_from(value: RawCommand) -> Result<Self, Self::Error> {
        match value.raw_command.as_str() {
            "fixup" => Ok(Self::Fixup),
            "apply" => {
                let arguments = value
                    .raw_arguments
                    .ok_or_else(|| anyhow!("apply is missing arguments."))?;
                Ok(Self::Apply(arguments.try_into()?))
            }
            s => bail!("{} is not a valid mason command.", s),
        }
    }
}

impl AuthorizedAction<MasonCommand> {
    async fn execute(&self) -> Result<Box<dyn Display + Send>, (Status, anyhow::Error)> {
        match &self.action.command {
            MasonCommand::Fixup => fixup::run(self).await,
            MasonCommand::Apply(patch) => apply::run(self, patch).await,
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
                Status::NoContent
            }
        },
        GitHubIssueCommentAction::Edited | GitHubIssueCommentAction::Deleted => Status::NoContent,
    }
}
