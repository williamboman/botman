#![feature(result_option_inspect)]
use std::env::var;

use reqwest::Client;

#[macro_use]
extern crate rocket;

#[macro_use]
extern crate lazy_static;

mod github;
mod hacktober;
mod mason;
mod mason_registry;

// TODO: verify these exist at startup
lazy_static! {
    static ref CLIENT: Client = reqwest::Client::new();
    static ref GITHUB_LOGIN: String = var("GITHUB_LOGIN").expect("No GITHUB_LOGIN.");
    static ref GITHUB_PAT: String = var("GITHUB_PAT").expect("No PAT.");
    static ref GITHUB_WEBHOOK_SECRET: String =
        var("GITHUB_WEBHOOK_SECRET").expect("No GITHUB_WEBHOOK_SECRET.");
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/api", routes![mason::index, mason_registry::index])
}
