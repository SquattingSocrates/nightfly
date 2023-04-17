#![deny(warnings)]
extern crate nightfly;

use lunatic::Mailbox;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct TwitterUser {
    id: String,
    name: String,
    #[serde(rename = "username")]
    user_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct TwitterResponse<T> {
    data: T,
}

// This is using the `lunatic` runtime.
//
#[lunatic::main]
fn main(_: Mailbox<()>) -> () {
    // nightfly::get() is a convenience function.
    //
    // In most cases, you should create/build a nightfly::Client and reuse
    // it for all requests.
    let token = std::env::var("TWITTER_BEARER_TOKEN").expect("TWITTER_BEARER_TOKEN not set");
    let res = nightfly::Client::new()
        .get("https://api.twitter.com/2/users/by/username/NASA")
        .header("Authorization", &format!("Bearer {}", token))
        .header("Accept", "application/json")
        // choose encoding if desired
        // .header("Accept-Encoding", "gzip")
        .send()
        .unwrap();

    eprintln!("Response: {:?} {}", res.version(), res.status());
    eprintln!("Headers: {:#?}\n", res.headers());

    let body: TwitterResponse<TwitterUser> = res.json().unwrap();

    println!("Loaded twitter user {body:?}");
}
