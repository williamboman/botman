use std::collections::HashMap;

use crate::{CLIENT, GITHUB_PAT};

use super::data::{GitHubComment, GitHubReaction, GitHubRepo};
use anyhow::{bail, Result};
use reqwest::{
    header::{HeaderMap, ACCEPT, AUTHORIZATION, USER_AGENT},
    Response,
};
use serde::Serialize;
use serde_json::{json, Map, Value};

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
    println!(
        "Creating issue comment reaction {:?} {:?} {:?}",
        reaction, comment, repo
    );
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

const MINIMIZE_COMMENT_MUTATION: &str = r#"
mutation minimizeComment($input: MinimizeCommentInput!) {
    minimizeComment(input: $input) {
        minimizedComment {
            isMinimized
        }
    }
}
"#;

#[allow(non_camel_case_types, dead_code)]
#[derive(Serialize)]
enum ReportedContentClassifier {
    ABUSE,
    DUPLICATE,
    OFF_TOPIC,
    OUTDATED,
    RESOLVED,
    SPAM,
}

#[allow(non_snake_case)]
#[derive(Serialize)]
struct MinimizeCommentInput {
    classifier: ReportedContentClassifier,
    clientMutationId: Option<String>,
    subjectId: String,
}

pub async fn minimize_comment(comment: &GitHubComment) -> Result<()> {
    println!("Minimizing comment {:?}", comment);
    let mut variables = Map::new();
    variables.insert(
        "input".to_owned(),
        json!(MinimizeCommentInput {
            classifier: ReportedContentClassifier::RESOLVED,
            clientMutationId: None, // dafuq is this?
            subjectId: comment.node_id.to_owned(),
        }),
    );
    let response = graphql(&GraphqlQuery {
        query: MINIMIZE_COMMENT_MUTATION.to_owned(),
        variables: Some(variables),
    })
    .await?;
    println!("I minimized comment!!! {}", response.text().await?);
    Ok(())
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

#[derive(Serialize)]
pub struct GraphqlQuery {
    query: String,
    variables: Option<Map<String, Value>>,
}

pub async fn graphql(query: &GraphqlQuery) -> Result<Response, reqwest::Error> {
    CLIENT
        .post("https://api.github.com/graphql")
        .headers(HEADERS.clone())
        .json(query)
        .send()
        .await
}
