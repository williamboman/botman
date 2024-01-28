use anyhow::{anyhow, bail, Result};
use hmac::{Hmac, Mac};
use rocket::{
    data::{self, FromData},
    http::Status,
    Data, Request,
};
use serde::Deserialize;

use sha2::Sha256;
use std::str::FromStr;

use crate::{
    github::data::{
        GitHubCheckRunEvent, GitHubIssueCommentEvent, GitHubIssuesEvent, GitHubPullRequestEvent,
        GitHubWebhook,
    },
    GITHUB_WEBHOOK_SECRET,
};

#[derive(Debug)]
pub struct GitHubSignature {
    pub prefix: String,
    pub payload: Vec<u8>,
}

impl FromStr for GitHubSignature {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some((prefix, payload)) = s.split_once("=") {
            Ok(Self {
                prefix: prefix.to_owned(),
                payload: hex::decode(payload)?,
            })
        } else {
            bail!("Bad GitHubSignature format.")
        }
    }
}

#[async_trait]
impl<'r> FromData<'r> for GitHubWebhook {
    type Error = anyhow::Error;

    async fn from_data(req: &'r Request<'_>, data: Data<'r>) -> data::Outcome<'r, Self> {
        let limit = req.limits().get("json").unwrap_or(data::Limits::JSON);
        let payload_str = match data.open(limit).into_string().await {
            Ok(s) if s.is_complete() => s.into_inner(),
            Ok(_) => {
                return data::Outcome::Error((
                    Status::PayloadTooLarge,
                    anyhow!("Payload exceeds limit {}", limit),
                ))
            }
            Err(e) => return data::Outcome::Error((Status::BadRequest, e.into())),
        };

        let mut hmac = Hmac::<Sha256>::new_from_slice(GITHUB_WEBHOOK_SECRET.as_bytes())
            .expect("Failed to create hmac.");
        hmac.update(payload_str.as_bytes());

        if let Some(signature) = req
            .headers()
            .get_one("X-Hub-Signature-256")
            .and_then(|s| s.parse::<GitHubSignature>().ok())
        {
            if let Ok(()) = hmac.verify_slice(&signature.payload) {
                return match parse_and_map_json(req, &payload_str) {
                    Ok(value) => data::Outcome::Success(value),
                    Err(err) => data::Outcome::Error(err),
                };
            } else {
                return data::Outcome::Error((
                    Status::Forbidden,
                    anyhow!("Mismatching signature."),
                ));
            }
        } else {
            return data::Outcome::Error((
                Status::Unauthorized,
                anyhow!("Bad or missing signature."),
            ));
        }
    }
}

fn parse<'r, T: Deserialize<'r>>(payload: &'r str) -> Result<T, (Status, anyhow::Error)> {
    serde_json::from_str::<T>(&payload).map_err(|x| (Status::UnprocessableEntity, anyhow!(x)))
}

fn parse_and_map_json<'r>(
    request: &'r Request<'_>,
    payload: &'r str,
) -> Result<GitHubWebhook, (Status, anyhow::Error)> {
    match request.headers().get_one("X-GitHub-Event") {
        Some("issue_comment") => {
            parse::<GitHubIssueCommentEvent>(payload).map(GitHubWebhook::IssueComment)
        }
        Some("issues") => parse::<GitHubIssuesEvent>(payload).map(GitHubWebhook::Issues),
        Some("pull_request") => {
            parse::<GitHubPullRequestEvent>(payload).map(GitHubWebhook::PullRequest)
        }
        Some("check_run") => parse::<GitHubCheckRunEvent>(payload).map(GitHubWebhook::CheckRun),
        Some(event) => Err((
            Status::NotImplemented,
            anyhow!("Event {} is not supported.", event),
        )),
        None => Err((
            Status::BadRequest,
            anyhow!("Missing X-GitHub-Event header."),
        )),
    }
}
