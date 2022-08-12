use std::collections::HashMap;

use crate::{CLIENT, GITHUB_PAT};

use super::data::{GitHubComment, GitHubReaction, GitHubRepo};
use anyhow::{anyhow, bail, Result};
use reqwest::{
    header::{HeaderMap, ACCEPT, AUTHORIZATION, USER_AGENT},
    Response,
};
use rocket::serde::DeserializeOwned;
use serde::{Deserialize, Serialize};
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

#[derive(Deserialize)]
#[allow(non_camel_case_types, non_snake_case)]
struct MinimizedComment {
    pub isMinimized: bool,
}

#[derive(Deserialize)]
#[allow(non_camel_case_types, non_snake_case)]
struct MinimizeComment {
    pub minimizedComment: MinimizedComment,
}

#[derive(Deserialize)]
#[allow(non_camel_case_types, non_snake_case)]
struct MinimizeCommentResponse {
    pub minimizeComment: MinimizeComment,
}

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
    let data = graphql::<MinimizeCommentResponse>(&GraphqlQuery {
        query: MINIMIZE_COMMENT_MUTATION.to_owned(),
        variables: Some(variables),
    })
    .await?
    .ok()?;

    if data.minimizeComment.minimizedComment.isMinimized {
        Ok(())
    } else {
        bail!("Failed to minimize comment.")
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

#[derive(Serialize)]
pub struct GraphqlQuery {
    query: String,
    variables: Option<Map<String, Value>>,
}

#[derive(Deserialize)]
pub struct GraphqlErrorLocation {
    pub line: u32,
    pub column: u32,
}

#[derive(Deserialize)]
pub struct GraphqlError {
    pub message: String,
    pub locations: Vec<GraphqlErrorLocation>,
    pub path: Vec<String>,
}

#[derive(Deserialize)]
pub struct GraphqlResponseEnvelope<Data> {
    pub data: Option<Data>,
    pub errors: Option<Vec<GraphqlError>>,
}

impl<Data> GraphqlResponseEnvelope<Data> {
    fn ok(self) -> Result<Data> {
        match self {
            Self {
                data: Some(data),
                errors: None,
            } => Ok(data),

            Self {
                data: None,
                errors: Some(errors),
            }
            | Self {
                data: Some(_),
                errors: Some(errors),
            } => Err(anyhow!("Oh noes").context(format!(
                "{:?}",
                errors
                    .iter()
                    .map(|err| err.message.as_str())
                    .collect::<Vec<&str>>()
            ))),

            Self {
                data: None,
                errors: None,
            } => Err(anyhow!("Missing both data and errors.")),
        }
    }
}

pub async fn graphql<Data: DeserializeOwned>(
    query: &GraphqlQuery,
) -> Result<GraphqlResponseEnvelope<Data>> {
    let response = CLIENT
        .post("https://api.github.com/graphql")
        .headers(HEADERS.clone())
        .json(query)
        .send()
        .await?;

    Ok(response.json().await?)
}
