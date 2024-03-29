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

const COMMENT_FOOTER: &str = r#"

<sup>` 🤖 This is an automated comment. `  [` 📖 Source code `](https://github.com/williamboman/botman)</sup>"#;

pub async fn create_issue_comment(
    repo: &GitHubRepo,
    issue_number: u64,
    comment: &str,
) -> Result<GitHubComment> {
    let merged_comment = comment.to_string() + COMMENT_FOOTER;
    println!(
        "Creating issue comment {:?} {} {:?}",
        merged_comment, issue_number, repo
    );
    post_json(
        format!("{}/issues/{}/comments", repo.as_api_url(), issue_number).as_str(),
        &HashMap::from([("body", &merged_comment)]),
    )
    .await
    .inspect_err(|e| {
        eprintln!("{}", e);
        eprintln!(
            "Failed to create issue comment {:?} {} {:?}",
            merged_comment, issue_number, repo
        )
    })
}

pub async fn create_issue_comment_reaction(
    repo: &GitHubRepo,
    comment: &GitHubComment,
    reaction: &GitHubReaction,
) -> Result<Value> {
    println!(
        "Creating issue comment reaction {:?} {:?} {:?}",
        reaction, comment, repo
    );
    post_json(
        format!(
            "{}/issues/comments/{}/reactions",
            repo.as_api_url(),
            comment.id
        )
        .as_str(),
        &HashMap::from([("content", reaction)]),
    )
    .await
    .inspect_err(|e| {
        eprintln!("{}", e);
        eprintln!(
            "Failed to create issue comment reaction {:?} {:?} {:?}",
            reaction, comment, repo
        )
    })
}

pub async fn add_labels_to_issue(
    repo: &GitHubRepo,
    labels: Vec<&str>,
    issue_number: u64,
) -> Result<Value> {
    println!("Adding labels to issue {:?} {}", labels, issue_number);
    post_json(
        format!("{}/issues/{}/labels", repo.as_api_url(), issue_number).as_str(),
        &HashMap::from([("labels", &labels)]),
    )
    .await
    .inspect_err(|e| {
        eprintln!("{}", e);
        eprintln!(
            "Failed to add labels to issue #{} {:?}",
            issue_number, labels
        );
    })
}

pub async fn create_column_card(column_id: u64, issue_id: u64) -> Result<Value> {
    println!("Creating column card {} {}", column_id, issue_id);
    post_json(
        format!(
            "https://api.github.com/projects/columns/{}/cards",
            column_id
        )
        .as_str(),
        &HashMap::from([
            ("content_type", Value::String("Issue".to_string())),
            ("content_id", Value::Number(issue_id.into())),
        ]),
    )
    .await
    .inspect_err(|e| {
        eprintln!("{}", e);
        eprintln!("Failed to create column card {} {}", column_id, issue_id);
    })
}

#[derive(Serialize, Debug)]
pub struct RequestReviewersDto {
    pub reviewers: Vec<String>,
    pub team_reviewers: Vec<String>,
}

pub async fn request_review(
    repo: &GitHubRepo,
    pull_request_number: u64,
    reviewers: &RequestReviewersDto,
) -> Result<Value> {
    println!(
        "Requesting reviewers {:?} {} {:?}",
        repo, pull_request_number, reviewers
    );
    post_json(
        format!(
            "{}/pulls/{}/requested_reviewers",
            repo.as_api_url(),
            pull_request_number
        )
        .as_str(),
        reviewers,
    )
    .await
    .inspect_err(|e| {
        eprintln!("{}", e);
        eprintln!(
            "Failed to request reviewers {:?} {} {:?}",
            repo, pull_request_number, reviewers
        );
    })
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

const UNMINIMIZE_COMMENT_MUTATION: &str = r#"
mutation unminimizeComment($input: UnminimizeCommentInput!) {
    unminimizeComment(input: $input) {
        unminimizedComment {
            isMinimized
        }
    }
}
"#;

#[derive(Deserialize)]
#[allow(non_camel_case_types, non_snake_case)]
pub struct MinimizedComment {
    pub isMinimized: bool,
}

#[derive(Deserialize)]
#[allow(non_camel_case_types, non_snake_case)]
pub struct MinimizeComment {
    pub minimizedComment: MinimizedComment,
}

#[derive(Deserialize)]
#[allow(non_camel_case_types, non_snake_case)]
pub struct MinimizeCommentResponse {
    pub minimizeComment: MinimizeComment,
}

#[derive(Deserialize)]
#[allow(non_camel_case_types, non_snake_case)]
pub struct UnminimizeComment {
    pub unminimizedComment: MinimizedComment,
}

#[derive(Deserialize)]
#[allow(non_camel_case_types, non_snake_case)]
pub struct UnminimizeCommentResponse {
    pub unminimizeComment: UnminimizeComment,
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

#[allow(non_snake_case)]
#[derive(Serialize)]
struct UnminimizeCommentInput {
    clientMutationId: Option<String>,
    subjectId: String,
}

pub async fn minimize_comment(comment: &GitHubComment) -> Result<MinimizeCommentResponse> {
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
    .await?;

    if data.minimizeComment.minimizedComment.isMinimized {
        Ok(data)
    } else {
        bail!("Failed to minimize comment.")
    }
}

pub async fn unminimize_comment(comment: &GitHubComment) -> Result<UnminimizeCommentResponse> {
    println!("Unminimizing comment {:?}", comment);
    let mut variables = Map::new();
    variables.insert(
        "input".to_owned(),
        json!(UnminimizeCommentInput {
            clientMutationId: None, // dafuq is this?
            subjectId: comment.node_id.to_owned(),
        }),
    );
    let data = graphql::<UnminimizeCommentResponse>(&GraphqlQuery {
        query: UNMINIMIZE_COMMENT_MUTATION.to_owned(),
        variables: Some(variables),
    })
    .await
    .inspect_err(|e| {
        eprintln!("{:?}", e);
        eprintln!("Failed to unminimize comment {:?}", comment)
    })?;

    if data.unminimizeComment.unminimizedComment.isMinimized {
        Ok(data)
    } else {
        bail!("Failed to minimize comment")
    }
}

pub async fn get(url: &str) -> Result<Response> {
    let response = CLIENT.get(url).headers(HEADERS.clone()).send().await;
    match response {
        Ok(response) if response.status().is_success() => Ok(response),
        Ok(err_response) => {
            eprintln!("{:?}", err_response);
            bail!(
                "Failed to fetch url {}, response status: {}",
                url,
                err_response.status()
            )
        }
        Err(err) => {
            eprintln!("{:?}", err);
            bail!("Failed to fetch url {}, response status: ", url)
        }
    }
}

pub async fn post_json<Payload: Serialize, Response: DeserializeOwned>(
    url: &str,
    payload: &Payload,
) -> Result<Response> {
    let response = CLIENT
        .post(url)
        .headers(HEADERS.clone())
        .json(payload)
        .send()
        .await;

    match response {
        Ok(response) if response.status().is_success() => Ok(response.json().await?),
        Ok(err_response) => {
            eprintln!("{:?}", err_response);
            bail!(
                "Failed to call {}, response status: {}",
                url,
                err_response.status()
            );
        }
        Err(err) => {
            eprintln!("{:?}", err);
            bail!("Failed to call {}", url);
        }
    }
}

#[derive(Serialize)]
pub struct GraphqlQuery {
    query: String,
    variables: Option<Map<String, Value>>,
}

#[derive(Deserialize, Debug)]
pub struct GraphqlErrorLocation {
    pub line: u32,
    pub column: u32,
}

#[derive(Deserialize, Debug)]
pub struct GraphqlResponseError {
    pub message: String,
    pub locations: Vec<GraphqlErrorLocation>,
    pub path: Vec<String>,
}

#[derive(Deserialize)]
pub struct GraphqlResponseEnvelope<Data> {
    pub data: Option<Data>,
    pub errors: Option<Vec<GraphqlResponseError>>,
}

#[derive(Debug)]
pub enum GraphqlError {
    Request(reqwest::Error),
    Response(Vec<GraphqlResponseError>),
    NoData,
}

impl From<reqwest::Error> for GraphqlError {
    fn from(err: reqwest::Error) -> Self {
        Self::Request(err)
    }
}

impl<Data> GraphqlResponseEnvelope<Data> {
    fn ok(self) -> Result<Data, GraphqlError> {
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
            } => Err(GraphqlError::Response(errors)),

            Self {
                data: None,
                errors: None,
            } => Err(GraphqlError::NoData),
        }
    }
}

pub async fn graphql<Data: DeserializeOwned>(query: &GraphqlQuery) -> Result<Data> {
    let response = CLIENT
        .post("https://api.github.com/graphql")
        .headers(HEADERS.clone())
        .json(query)
        .send()
        .await?;

    response
        .json::<GraphqlResponseEnvelope<Data>>()
        .await?
        .ok()
        .map_err(|e| {
            eprintln!("{:?}", e);
            match e {
                GraphqlError::Request(req_err) => anyhow!(
                    "Failed to call GraphQL, response status: {:?}",
                    req_err.status()
                ),
                GraphqlError::Response(_) => anyhow!("Received GraphQL errors."),
                GraphqlError::NoData => anyhow!("Didn't receive GraphQL data or errors."),
            }
        })
}
