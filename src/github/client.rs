use std::collections::HashMap;

use crate::{CLIENT, GITHUB_PAT};

use super::data::{GitHubComment, GitHubReaction, GitHubRepo};
use anyhow::{bail, Result};
use reqwest::{
    header::{HeaderMap, ACCEPT, AUTHORIZATION, USER_AGENT},
    Response,
};
use serde::Serialize;

// TODO maybe create a struct or something idk

lazy_static! {
    static ref HEADERS: HeaderMap = {
        let mut header_map = HeaderMap::new();
        header_map.insert(ACCEPT, "application/json".parse().unwrap());
        header_map.insert(
            AUTHORIZATION,
            format!("token {}", GITHUB_PAT.as_str()).parse().unwrap(),
        );
        header_map.insert(
            USER_AGENT,
            "botman (+https://github.com/williamboman/botman)"
                .parse()
                .unwrap(),
        );
        header_map
    };
}

pub async fn create_issue_comment_reaction(
    repo: &GitHubRepo,
    comment: &GitHubComment,
    reaction: &GitHubReaction,
) -> Result<()> {
    let response = post_json(
        format!(
            "{}/issues/comments/{}/reactions",
            repo.as_api_url(),
            comment.id
        )
        .as_str(),
        &HashMap::from([("content", reaction)]),
    )
    .await?;

    if response.status().is_success() {
        println!(
            "Creating issue comment reaction {:?} {:?} {:?}",
            reaction, comment, repo
        );
        Ok(())
    } else {
        bail!(
            "Failed to create issue comment reaction {:?} {:?} {:?}",
            reaction,
            comment,
            repo
        )
    }
}

pub async fn get(url: &str) -> Result<Response, reqwest::Error> {
    CLIENT.get(url).headers(HEADERS.clone()).send().await
}

pub async fn post_json<P: Serialize>(url: &str, payload: &P) -> Result<Response, reqwest::Error> {
    CLIENT
        .post(url)
        .headers(HEADERS.clone())
        .json(payload)
        .send()
        .await
}
