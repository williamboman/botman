use crate::github::{client, data::GitHubReaction};

use self::parser::{AuthorizedAction, AuthorizedActionExecutor, RawCommand};

use super::data::{GitHubIssueCommentEvent, GitHubIssueCommentEventAction};
use rocket::http::Status;

pub mod apply;
pub mod common;
pub mod parser;

pub async fn handle_issue_comment<Command>(event: GitHubIssueCommentEvent) -> Status
where
    Command: TryFrom<RawCommand, Error = anyhow::Error>,
    Command: AuthorizedActionExecutor,
{
    let repo = event.repository.clone();
    let comment = event.comment.clone();
    match event.action {
        GitHubIssueCommentEventAction::Created => match event.try_into() {
            Ok(action @ AuthorizedAction::<Command> { .. }) => {
                match Command::execute(action).await {
                    Ok(result) => {
                        println!("{}", result);
                        Status::NoContent
                    }
                    Err((status, err)) => {
                        let _ = client::unminimize_comment(&comment).await;
                        let _ = client::create_issue_comment_reaction(
                            &repo,
                            &comment,
                            &GitHubReaction::MinusOne,
                        )
                        .await;
                        eprintln!("ERROR: {:?}", err);
                        status
                    }
                }
            }
            Err(err) => {
                println!("Failed to parse action from comment: {:?}", err);
                Status::NoContent
            }
        },
        GitHubIssueCommentEventAction::Edited | GitHubIssueCommentEventAction::Deleted => {
            Status::NoContent
        }
    }
}
