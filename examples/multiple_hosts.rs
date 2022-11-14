#![deny(warnings)]
extern crate nightfly;

use lunatic::Mailbox;
use nightfly::Client;

// This is using the `lunatic` runtime.
//
#[lunatic::main]
fn main(_: Mailbox<()>) -> () {
    // first, start the client pool
    let client = Client::new();
    let res1 = client.get("https://hyper.rs").send().unwrap();
    println!("Call to https://hyper.rs {}", res1.text().unwrap());

    let res2 = client
        .get("http://anglesharp.azurewebsites.net/Chunked")
        .send()
        .unwrap();

    println!(
        "Delayed chunking test at http://anglesharp.azurewebsites.net/Chunked {}",
        res2.text().unwrap()
    );

    let res3 = client.get("https://rust-lang.org").send().unwrap();
    println!("Call to https://rust-lang.org {}", res3.text().unwrap());
}
