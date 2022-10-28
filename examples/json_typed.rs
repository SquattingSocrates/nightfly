//! This example illustrates the way to send and receive statically typed JSON.
//!
//! In contrast to the arbitrary JSON example, this brings up the full power of
//! Rust compile-time type system guaranties though it requires a little bit
//! more code.

use std::collections::HashMap;

use lunatic::Mailbox;
// These require the `serde` dependency.
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Post {
    id: Option<i32>,
    title: String,
    body: String,
    #[serde(rename = "userId")]
    user_id: i32,
}

#[derive(Debug, Serialize, Deserialize)]
struct AnythingResponse<T> {
    args: HashMap<String, String>,
    data: String,
    files: HashMap<String, String>,
    form: HashMap<String, String>,
    headers: HashMap<String, String>,
    json: Option<T>,
    method: String,
    origin: String,
    url: String,
}

// This is using the `lunatic` runtime
//
#[lunatic::main]
fn main(_: Mailbox<()>) -> Result<(), nightfly::Error> {
    let new_post = Post {
        id: None,
        title: "Nightfly.rs".into(),
        body: "https://docs.rs/nightfly".into(),
        user_id: 1,
    };
    let new_post: AnythingResponse<Post> = nightfly::Client::new()
        .post("http://eu.httpbin.org/anything")
        .json(&new_post)
        .send()
        .unwrap()
        .json()
        .unwrap();

    println!("{:#?}", new_post);
    // Post {
    //     id: Some(
    //         101
    //     ),
    //     title: "Nightfly.rs",
    //     body: "https://docs.rs/nightfly",
    //     user_id: 1
    // }
    // Ok(())
}
