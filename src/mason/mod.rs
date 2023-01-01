use std::fmt::Display;

use crate::github::{
    action_parser::*,
    client,
    data::{
        GitHubIssueCommentEvent, GitHubIssueCommentEventAction, GitHubIssuesEvent,
        GitHubIssuesEventAction, GitHubPullRequestEvent, GitHubPullRequestEventAction,
        GitHubReaction, GitHubWebhook,
    },
};
use anyhow::{anyhow, bail, Result};
use chrono::{Datelike, NaiveDate, Utc};
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

async fn issue_comment(event: GitHubIssueCommentEvent) -> Status {
    let repo = event.repository.clone();
    let comment = event.comment.clone();
    match event.action {
        GitHubIssueCommentEventAction::Created => match event.try_into() {
            Ok(action @ AuthorizedAction::<MasonCommand> { .. }) => match action.execute().await {
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
            },
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

const NEW_PACKAGE_COMMENT: &str = r#"Hello! Pull requests are always very welcomed to add new packages. If the distribution of the package is simple, the installation will most likely be so as well. See [CONTRIBUTING.md](https://github.com/williamboman/mason.nvim/blob/main/CONTRIBUTING.md) and the [API reference](https://github.com/williamboman/mason.nvim/blob/main/doc/reference.md) for more details! You may also use existing packages as reference."#;

lazy_static! {
    static ref MASON_PROJECT_ID: u64 = 14574269;
    static ref MASON_PROJECT_COLUMN_PRIO_ID: u64 = 19009769;
    static ref MASON_PROJECT_COLUMN_TRIAGE_ID: u64 = 19009768;
    static ref MASON_PROJECT_COLUMN_BACKLOG_ID: u64 = 19009770;
    static ref MASON_PROJECT_COLUMN_SUPPORT_ID: u64 = 19114644;
    static ref MASON_PROJECT_COLUMN_CLOSED_ID: u64 = 19009772;
}

async fn issue_event(event: GitHubIssuesEvent) -> Status {
    match event.action {
        GitHubIssuesEventAction::Opened => {
            if event.issue.has_label("new-package-request") {
                let _ = tokio::join!(
                    client::create_issue_comment(
                        &event.repository,
                        event.issue.number,
                        NEW_PACKAGE_COMMENT,
                    ),
                    client::create_column_card(*MASON_PROJECT_COLUMN_PRIO_ID, event.issue.id),
                    client::add_labels_to_issue(
                        &event.repository,
                        vec!["help wanted"],
                        event.issue.number,
                    )
                );
            } else {
                let _ = client::create_column_card(*MASON_PROJECT_COLUMN_TRIAGE_ID, event.issue.id)
                    .await;
            }
            Status::NoContent
        }
        _ => Status::NoContent,
    }
}

async fn hacktoberfest_label(event: &GitHubPullRequestEvent) {
    let now = Utc::now().date_naive();
    let start = NaiveDate::from_ymd_opt(now.year(), 9, 25);
    let end = NaiveDate::from_ymd_opt(now.year(), 11, 5);
    if let (Some(start), Some(end)) = (start, end) {
        if (start <= now) && (now <= end) {
            let _ = client::add_labels_to_issue(
                &event.repository,
                vec!["hacktoberfest-accepted"],
                event.pull_request.number,
            )
            .await;
        }
    }
}

async fn pull_request(event: GitHubPullRequestEvent) -> Status {
    match event.action {
        GitHubPullRequestEventAction::Closed if event.pull_request.merged => {
            if event.pull_request.user.login != "williambotman" {
                hacktoberfest_label(&event).await;
            }
            Status::NoContent
        }
        _ => Status::NoContent,
    }
}

#[post("/v1/mason/github-webhook", format = "json", data = "<webhook>")]
pub async fn index(webhook: GitHubWebhook) -> Status {
    println!("{:?}", webhook);
    match webhook {
        GitHubWebhook::IssueComment(event) => issue_comment(event).await,
        GitHubWebhook::Issues(event) => issue_event(event).await,
        GitHubWebhook::PullRequest(event) => pull_request(event).await,
        _ => Status::NotImplemented,
    }
}
