//! This example illustrates the way to send and receive arbitrary JSON.
//!
//! This is useful for some ad-hoc experiments and situations when you don't
//! really care about the structure of the JSON and just need to display it or
//! process it at runtime.

use lunatic::Mailbox;

#[lunatic::main]
fn main(_: Mailbox<()>) -> Result<(), nightfly::Error> {
    let echo_json: serde_json::Value = nightfly::Client::builder()
        .user_agent("my-own-user-agent")
        .build()
        .unwrap()
        .post("http://eu.httpbin.org/anything")
        .json(&serde_json::json!({
            "title": "Nightfly.rs",
            "body": "https://docs.rs/nightfly",
            "userId": 1
        }))
        .send()
        .unwrap()
        .json()
        .unwrap();

    println!("{:#?}", echo_json);
    // Object(
    //     {
    //         "body": String(
    //             "https://docs.rs/nightfly"
    //         ),
    //         "id": Number(
    //             101
    //         ),
    //         "title": String(
    //             "Nightfly.rs"
    //         ),
    //         "userId": Number(
    //             1
    //         )
    //     }
    // )
    // Ok(())
}
