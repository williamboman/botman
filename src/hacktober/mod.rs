use chrono::{Datelike, NaiveDate, Utc};

use crate::github::{client, data::GitHubPullRequestEvent};

pub async fn hacktoberfest_label(event: &GitHubPullRequestEvent) {
    if !event.pull_request.merged {
        return;
    }

    match event.pull_request.user.login.as_str() {
        "williambotman" | "renovate[bot]" => {
            return;
        }
        _ => {}
    }

    let now = Utc::now().date_naive();
    let start = NaiveDate::from_ymd_opt(now.year(), 9, 25);
    let end = NaiveDate::from_ymd_opt(now.year(), 11, 5);
    if let (Some(start), Some(end)) = (start, end) {
        if (start <= now) && (now <= end) {
            let _ = client::add_labels_to_issue(
                &event.repository,
                vec!["hacktoberfest-accepted"],
                event.pull_request.number,
            )
            .await;
        }
    }
}
