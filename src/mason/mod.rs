use std::fmt::Display;

use crate::{
    github::{
        action::{common::GitApplyPatch, parser::*},
        client,
        data::{GitHubIssuesEvent, GitHubIssuesEventAction, GitHubPullRequestEvent, GitHubWebhook},
    },
    hacktober::hacktoberfest_label,
};
use anyhow::{anyhow, bail, Result};
use rocket::http::Status;

mod fixup;
mod merge_base;

#[derive(Debug)]
enum MasonCommand {
    Fixup,
    MergeBase,
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
            "merge-base" => Ok(Self::MergeBase),
            s => bail!("{} is not a valid mason command.", s),
        }
    }
}

#[async_trait]
impl AuthorizedActionExecutor for MasonCommand {
    async fn execute(
        action: AuthorizedAction<MasonCommand>,
    ) -> Result<Box<dyn Display + Send>, (Status, anyhow::Error)> {
        match &action.action.command {
            MasonCommand::Fixup => fixup::run(&action).await,
            MasonCommand::Apply(patch) => crate::github::action::apply::run(&action, patch).await,
            MasonCommand::MergeBase => merge_base::run(&action).await,
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

async fn pull_request(event: GitHubPullRequestEvent) -> Status {
    hacktoberfest_label(&event).await;
    Status::NoContent
}

#[post("/v1/mason/github-webhook", format = "json", data = "<webhook>")]
pub async fn index(webhook: GitHubWebhook) -> Status {
    println!("{:?}", webhook);
    match webhook {
        GitHubWebhook::IssueComment(event) => {
            crate::github::action::handle_issue_comment::<MasonCommand>(event).await
        }
        GitHubWebhook::Issues(event) => issue_event(event).await,
        GitHubWebhook::PullRequest(event) => pull_request(event).await,
        _ => Status::NotImplemented,
    }
}
