use anyhow::{anyhow, Result};
use std::{fmt::Debug, str::FromStr};

use serde::{de, Deserialize, Serialize};

#[derive(Deserialize, Debug, Clone)]
pub struct GitHubUser {
    pub id: u64,
    pub login: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GitHubTeam {
    pub id: u64,
    pub name: String,
    pub slug: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GitHubComment {
    pub id: u64,
    pub node_id: String,
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
    pub fn as_git_url(&self) -> String {
        format!(
            "https://github.com/{}/{}.git",
            self.full_name.owner, self.full_name.name
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
    pub id: u64,
    pub number: u64,
    pub head: GitHubRef,
    pub base: GitHubRef,
    pub merged: bool,
    pub user: GitHubUser,
    pub requested_teams: Vec<GitHubTeam>
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum GitHubPullRequestEventAction {
    Assigned,
    AutoMergeDisabled,
    AutoMergeEnabled,
    /// If the action is closed and the merged key is false, the pull request was closed with
    /// unmerged commits. If the action is closed and the merged key is true, the pull request was
    /// merged.
    Closed,
    ConvertedToDraft,
    /// Triggered when a pull request is removed from a merge queue
    Dequeued,
    Edited,
    /// Triggered when a pull request is added to a merge queue
    Enqueued,
    Labeled,
    Locked,
    Opened,
    ReadyForReview,
    Reopened,
    ReviewRequestRemoved,
    ReviewRequested,
    /// Triggered when a pull request's head branch is updated. For example, when the head branch
    /// is updated from the base branch, when new commits are pushed to the head branch, or when
    /// the base branch is changed.
    Synchronize,
    Unassigned,
    Unlabeled,
    Unlocked,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GitHubPullRequestEvent {
    pub action: GitHubPullRequestEventAction,
    pub repository: GitHubRepo,
    pub pull_request: GitHubPullRequest,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum GitHubCheckRunEventAction {
    Completed,
    Created,
    RequestedAction,
    Rerequested,
}

#[derive(PartialEq, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum GitHubCheckRunConclusion {
    ActionRequired,
    Cancelled,
    Failure,
    Neutral,
    Pending,
    Skipped,
    Stale,
    StartupFailure,
    Success,
    TimedOut,
    Waiting,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum GitHubCheckRunStatus {
    Completed,
    InProgress,
    Pending,
    Queued,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GitHubCheckRun {
    pub id: u64,
    pub conclusion: Option<GitHubCheckRunConclusion>,
    pub pull_requests: Vec<GitHubCheckRunPullRequest>,
    pub started_at: String,
    pub status: GitHubCheckRunStatus,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GitHubCheckRunRepo {
    pub id: u64,
    pub name: String,
    pub url: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GitHubCheckRunRef {
    pub r#ref: String,
    pub repo: GitHubCheckRunRepo,
    pub sha: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GitHubCheckRunPullRequest {
    pub id: u64,
    pub number: u64,
    pub url: String,
    pub base: GitHubCheckRunRef,
    pub head: GitHubCheckRunRef,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GitHubCheckRunEvent {
    pub action: GitHubCheckRunEventAction,
    pub repository: GitHubRepo,
    pub check_run: GitHubCheckRun,
}

#[derive(Debug)]
pub enum GitHubWebhook {
    IssueComment(GitHubIssueCommentEvent),
    Issues(GitHubIssuesEvent),
    PullRequest(GitHubPullRequestEvent),
    CheckRun(GitHubCheckRunEvent),
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
pub enum GitHubIssueCommentEventAction {
    Created,
    Edited,
    Deleted,
}

#[derive(Deserialize, Debug)]
pub struct GitHubIssueCommentEvent {
    pub action: GitHubIssueCommentEventAction,
    pub issue: GitHubIssue,
    pub comment: GitHubComment,
    pub repository: GitHubRepo,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum GitHubIssuesEventAction {
    Opened,
    Edited,
    Deleted,
    Pinned,
    Unpinned,
    Closed,
    Reopened,
    Assigned,
    Unassigned,
    Labeled,
    Unlabeled,
    Locked,
    Unlocked,
    Transferred,
    Milestoned,
    Demilestoned,
}

#[derive(Deserialize, Debug)]
pub struct GitHubIssuesEvent {
    pub action: GitHubIssuesEventAction,
    pub issue: GitHubIssue,
    pub repository: GitHubRepo,
    pub sender: GitHubUser,
}

#[derive(PartialEq, Deserialize, Debug)]
pub struct GitHubIssuePullRequest {
    pub url: String,
    pub merged_at: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum GitHubIssueState {
    Open,
}

#[derive(Deserialize, Debug)]
pub struct GitHubIssueLabel {
    pub id: u64,
    pub name: String,
    pub description: String,
}

#[derive(Deserialize, Debug)]
pub struct GitHubIssue {
    pub id: u64,
    pub number: u64,
    pub user: GitHubUser,
    pub title: String,
    pub body: Option<String>,
    pub assignees: Vec<GitHubUser>,
    pub locked: bool,
    pub comments: u64,
    pub labels: Vec<GitHubIssueLabel>,
    pub state: GitHubIssueState,
    pub pull_request: Option<GitHubIssuePullRequest>,
}

impl GitHubIssue {
    pub fn has_label(&self, label: &str) -> bool {
        self.labels.iter().any(|l| l.name == label)
    }
}
