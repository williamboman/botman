use crate::{
    github::{
        client::{self, RequestReviewersDto},
        data::{
            GitHubCheckRunConclusion, GitHubCheckRunEvent, GitHubCheckRunStatus, GitHubIssuesEvent,
            GitHubIssuesEventAction, GitHubPullRequest, GitHubPullRequestEvent,
            GitHubPullRequestEventAction, GitHubRepo, GitHubWebhook,
        },
    },
    hacktober::hacktoberfest_label,
};
use anyhow::Result;
use rocket::http::Status;

#[derive(Debug)]
enum NotifyReason {
    RenovateFailedCI,
    NewIssue,
    NewPullRequest,
}

impl NotifyReason {
    fn explain(&self) -> String {
        format!("`notify({:?})`", self)
    }
}

async fn notify_triage(
    repo: &GitHubRepo,
    issue_number: u64,
    request_review: bool,
    reason: NotifyReason,
) {
    if request_review {
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
    if let Ok(comment) = client::create_issue_comment(
        repo,
        issue_number,
        &format!(
            "@mason-org/triage {}\n<sup>This comment ensures a notification is delivered to maintainers.</sup>",
            reason.explain()
        ),
    )
    .await {
        let _ = client::minimize_comment(&comment);
    }
}

async fn issue_event(event: GitHubIssuesEvent) -> Result<Status> {
    match event.action {
        GitHubIssuesEventAction::Opened if event.issue.pull_request == None => {
            notify_triage(
                &event.repository,
                event.issue.number,
                false,
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
                    notify_triage(
                        &event.repository,
                        pr.number,
                        true,
                        NotifyReason::RenovateFailedCI,
                    )
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
                true,
                NotifyReason::NewPullRequest,
            )
            .await
        }
        _ => {}
    }
    Ok(Status::NoContent)
}

#[post(
    "/v1/mason-registry/github-webhook",
    format = "json",
    data = "<webhook>"
)]
pub async fn index(webhook: GitHubWebhook) -> Status {
    println!("{:?}", webhook);
    match webhook {
        GitHubWebhook::Issues(event) => issue_event(event)
            .await
            .unwrap_or(Status::InternalServerError),
        GitHubWebhook::CheckRun(event) => check_run_event(event)
            .await
            .unwrap_or(Status::InternalServerError),
        GitHubWebhook::PullRequest(event) => pull_request(event)
            .await
            .unwrap_or(Status::InternalServerError),
        _ => Status::NotImplemented,
    }
}
