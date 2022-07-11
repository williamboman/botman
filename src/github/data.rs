use anyhow::{anyhow, Result};
use std::{fmt::Debug, str::FromStr};

use serde::{de, Deserialize, Serialize};

#[derive(Deserialize, Debug, Clone)]
pub struct GitHubUser {
    pub id: u64,
    pub login: String,
}

#[derive(Deserialize, Debug)]
pub struct GitHubComment {
    pub id: u64,
    pub body: Option<String>,
    pub user: GitHubUser,
}

#[derive(Clone)]
pub struct GitHubRepoId {
    pub owner: String,
    pub name: String,
}

impl FromStr for GitHubRepoId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let (owner, name) = s
            .split_once("/")
            .ok_or_else(|| anyhow!("Failed to parse GitHubRepoId."))?;

        Ok(GitHubRepoId {
            owner: owner.to_owned(),
            name: name.to_owned(),
        })
    }
}

impl Debug for GitHubRepoId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.owner, self.name)
    }
}

impl<'de> Deserialize<'de> for GitHubRepoId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(String::deserialize(deserializer)
            .map_err(de::Error::custom)?
            .parse()
            .map_err(de::Error::custom)?)
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct GitHubRepo {
    pub id: u64,
    pub full_name: GitHubRepoId,
}

impl GitHubRepo {
    pub fn as_git_url(&self, basic_auth: &str) -> String {
        format!(
            "https://{}@github.com/{}/{}.git",
            basic_auth, self.full_name.owner, self.full_name.name
        )
    }

    pub fn as_api_url(&self) -> String {
        format!(
            "https://api.github.com/repos/{}/{}",
            self.full_name.owner, self.full_name.name
        )
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct GitHubRef {
    pub r#ref: String,
    pub sha: String,
    pub user: GitHubUser,
    pub repo: GitHubRepo,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GitHubPullRequest {
    pub head: GitHubRef,
    pub base: GitHubRef,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum GitHubPullRequestReviewCommentAction {
    Created,
    Edited,
    Deleted,
}

#[derive(Deserialize, Debug)]
pub struct GitHubPullRequestReviewComment {
    pub action: GitHubPullRequestReviewCommentAction,
    pub comment: GitHubComment,
    pub pull_request: GitHubPullRequest,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum GitHubPullRequestReviewAction {
    Submitted,
    Edited,
    Dismissed,
}

#[derive(Deserialize, Debug)]
pub struct GitHubPullRequestReview {
    pub action: GitHubPullRequestReviewAction,
    pub review: GitHubComment,
    pub pull_request: GitHubPullRequest,
}

#[derive(Deserialize, Debug)]
pub enum GitHubReaction {
    PlusOne,  // 👍
    MinusOne, // 👎
    Laugh,    // 😄
    Confused, // 😄
    Heart,    // ❤️
    Hooray,   // 🎉
    Rocket,   // 🚀
    Eyes,     // 👀
}

impl Serialize for GitHubReaction {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(match self {
            GitHubReaction::PlusOne => "+1",
            GitHubReaction::MinusOne => "-1",
            GitHubReaction::Laugh => "laugh",
            GitHubReaction::Confused => "confused",
            GitHubReaction::Heart => "heart",
            GitHubReaction::Hooray => "hooray",
            GitHubReaction::Rocket => "rocket",
            GitHubReaction::Eyes => "eyes",
        })
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum GitHubIssueCommentAction {
    Created,
    Updated,
    Deleted,
}

#[derive(Deserialize, Debug)]
pub struct GitHubIssueComment {
    pub action: GitHubIssueCommentAction,
    pub issue: GitHubIssue,
    pub comment: GitHubComment,
    pub repository: GitHubRepo,
}

#[derive(Deserialize, Debug)]
pub struct GitHubIssuePullRequest {
    pub url: String,
    pub merged_at: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct GitHubIssue {
    pub id: u64,
    pub user: GitHubUser,
    pub pull_request: Option<GitHubIssuePullRequest>,
}
