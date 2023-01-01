use crate::github::{
    client::{self, RequestReviewersDto},
    data::{
        GitHubCheckRunConclusion, GitHubCheckRunEvent, GitHubCheckRunStatus, GitHubIssuesEvent,
        GitHubIssuesEventAction, GitHubPullRequest, GitHubWebhook,
    },
};
use anyhow::Result;
use rocket::http::Status;

async fn issue_event(event: GitHubIssuesEvent) -> Result<Status> {
    match event.action {
        GitHubIssuesEventAction::Opened if event.issue.pull_request == None => {
            client::create_issue_comment(
                &event.repository,
                event.issue.number,
                "@mason-org/triage",
            )
            .await?;
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
                    let _ = client::request_review(
                        &event.repository,
                        pr.number,
                        &RequestReviewersDto {
                            reviewers: vec![],
                            team_reviewers: vec!["triage".to_string()],
                        },
                    )
                    .await;
                    let _ = client::create_issue_comment(
                        &event.repository,
                        pr.number,
                        "@mason-org/triage",
                    )
                    .await;
                }
            }
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
        _ => Status::NotImplemented,
    }
}
