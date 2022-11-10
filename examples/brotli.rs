#![deny(warnings)]
extern crate nightfly;

use lunatic::Mailbox;

// This is using the `lunatic` runtime.
//
#[lunatic::main]
fn main(_: Mailbox<()>) -> () {
    // Some simple CLI args requirements...
    let url = match std::env::args().nth(1) {
        Some(url) => url,
        None => {
            println!("No CLI URL provided, using default.");
            "http://eu.httpbin.org/brotli".into()
        }
    };

    eprintln!("Fetching {:?}...", url);

    // nightfly::get() is a convenience function.
    //
    // In most cases, you should create/build a nightfly::Client and reuse
    // it for all requests.
    let res = nightfly::get(url).unwrap();

    eprintln!("Response: {:?} {}", res.version(), res.status());
    eprintln!("Headers: {:#?}\n", res.headers());

    let body = res.text().unwrap();

    println!("BODY {}", body);

    // Ok(())
}
