use std::fmt::Display;

use crate::{
    github::{
        action::{
            common::GitApplyPatch,
            parser::{AuthorizedAction, AuthorizedActionExecutor, RawCommand},
        },
        client::{self, RequestReviewersDto},
        data::{
            GitHubCheckRunConclusion, GitHubCheckRunEvent, GitHubCheckRunStatus, GitHubIssuesEvent,
            GitHubIssuesEventAction, GitHubPullRequest, GitHubPullRequestEvent,
            GitHubPullRequestEventAction, GitHubRepo, GitHubWebhook,
        },
    },
    hacktober::hacktoberfest_label,
};
use anyhow::{anyhow, bail, Result};
use rocket::http::Status;

#[derive(Debug)]
enum NotifyReason {
    RenovateFailedCI,
    NewIssue,
    NewPullRequest,
}

impl NotifyReason {
    fn explain(&self) -> String {
        format!("`notify(Reason::{:?})`", self)
    }
}

async fn notify_triage(repo: &GitHubRepo, issue_number: u64, reason: NotifyReason) {
    match reason {
        NotifyReason::RenovateFailedCI | NotifyReason::NewPullRequest => {
            let _ = client::request_review(
                repo,
                issue_number,
                &RequestReviewersDto {
                    reviewers: vec![],
                    team_reviewers: vec!["triage".to_string()],
                },
            )
            .await;
        }
        NotifyReason::NewIssue => {}
    }

    if let Ok(comment) = client::create_issue_comment(
        repo,
        issue_number,
        &format!(
            "@mason-org/triage {}\n\n<sup>This comment ensures a notification is delivered to maintainers.</sup>",
            reason.explain()
        ),
    )
    .await {
        let _ = client::minimize_comment(&comment).await;
    }
}

async fn issue_event(event: GitHubIssuesEvent) -> Result<Status> {
    match event.action {
        GitHubIssuesEventAction::Opened if event.issue.pull_request == None => {
            notify_triage(
                &event.repository,
                event.issue.number,
                NotifyReason::NewIssue,
            )
            .await;
        }
        _ => {}
    }
    Ok(Status::NoContent)
}

async fn check_run_event(event: GitHubCheckRunEvent) -> Result<Status> {
    match event.check_run.status {
        GitHubCheckRunStatus::Completed
            if event.check_run.conclusion == Some(GitHubCheckRunConclusion::Failure)
                || event.check_run.conclusion == Some(GitHubCheckRunConclusion::Cancelled)
                || event.check_run.conclusion == Some(GitHubCheckRunConclusion::StartupFailure)
                || event.check_run.conclusion == Some(GitHubCheckRunConclusion::TimedOut) =>
        {
            if let Some(check_run_pr) = event.check_run.pull_requests.first() {
                let pr: GitHubPullRequest = client::get(&check_run_pr.url).await?.json().await?;

                if pr.user.login == "renovate[bot]" && pr.requested_teams.len() == 0 {
                    notify_triage(&event.repository, pr.number, NotifyReason::RenovateFailedCI)
                        .await;
                }
            }
        }
        _ => {}
    }
    Ok(Status::NoContent)
}

async fn pull_request(event: GitHubPullRequestEvent) -> Result<Status> {
    hacktoberfest_label(&event).await;

    match event.action {
        GitHubPullRequestEventAction::Opened
            if event.pull_request.user.login != "renovate[bot]" =>
        {
            notify_triage(
                &event.repository,
                event.pull_request.number,
                NotifyReason::NewPullRequest,
            )
            .await
        }
        _ => {}
    }
    Ok(Status::NoContent)
}

#[derive(Debug)]
enum MasonRegistryCommand {
    Apply(GitApplyPatch),
}

impl TryFrom<RawCommand> for MasonRegistryCommand {
    type Error = anyhow::Error;

    fn try_from(value: RawCommand) -> Result<Self, Self::Error> {
        match value.raw_command.as_str() {
            "apply" => {
                let arguments = value
                    .raw_arguments
                    .ok_or_else(|| anyhow!("apply is missing arguments."))?;
                Ok(Self::Apply(arguments.try_into()?))
            }
            s => bail!("{} is not a valid mason-registry command.", s),
        }
    }
}

#[async_trait]
impl AuthorizedActionExecutor for MasonRegistryCommand {
    async fn execute(
        action: AuthorizedAction<MasonRegistryCommand>,
    ) -> Result<Box<dyn Display + Send>, (Status, anyhow::Error)> {
        match &action.action.command {
            MasonRegistryCommand::Apply(patch) => {
                crate::github::action::apply::run(&action, patch).await
            }
        }
    }
}

#[post(
    "/v1/mason-registry/github-webhook",
    format = "json",
    data = "<webhook>"
)]
pub async fn index(webhook: GitHubWebhook) -> Status {
    println!("{:?}", webhook);
    match webhook {
        GitHubWebhook::IssueComment(event) => {
            Ok(crate::github::action::handle_issue_comment::<MasonRegistryCommand>(event).await)
        }
        GitHubWebhook::Issues(event) => issue_event(event).await,
        GitHubWebhook::CheckRun(event) => check_run_event(event).await,
        GitHubWebhook::PullRequest(event) => pull_request(event).await,
        #[allow(unreachable_patterns)]
        _ => Ok(Status::NotImplemented),
    }
    .unwrap_or(Status::InternalServerError)
}
