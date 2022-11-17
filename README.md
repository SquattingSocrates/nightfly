# nightfly

This project is an ongoing effort to port the reqwest library to the lunatic runtime

## What works:

* [x] json, text and bytes for request and response bodies
* [x] decompression with brotli, gzip and deflate
* [x] redirect handling
* [x] cookies
* [x] chunked responses
* [x] handling of multiple open tcp streams per client
* [ ] timeouts (not sure how this should look like in a lunatic setup)
* [ ] Piping of responses (requires chunk-encoding)
* [ ] pooling of connections (needs more usage of lib to find a good approach)
* [ ] proxy handling
* [ ] upgrade, socks5 support and websockets
* [ ] custom dns resolver

<!-- [![crates.io](https://img.shields.io/crates/v/nightfly.svg)](https://crates.io/crates/nightfly) -->
<!-- [![Documentation](https://docs.rs/nightfly/badge.svg)](https://docs.rs/nightfly) -->
[![MIT/Apache-2 licensed](https://img.shields.io/crates/l/nightfly.svg)](./LICENSE-APACHE)
[![CI](https://github.com/SquattingSocrates/nightfly/workflows/CI/badge.svg)](https://github.com/SquattingSocrates/nightfly/actions?query=workflow%3ACI)

An ergonomic, batteries-included HTTP Client for the lunatic runtime written in Rust.

- Plain bodies, JSON, urlencoded, multipart (see examples)
- Customizable redirect policy (IN PROGRESS)
- HTTP Proxies (IN PROGRESS)
- HTTPS via lunatic-native TLS (see examples)
- Cookie Store (IN PROGRESS)
- [Changelog](CHANGELOG.md)


## Example

This example uses [Lunatic](https://lunatic.rs) and enables some
optional features, so your `Cargo.toml` could look like this:

```toml
[dependencies]
nightfly = { "0.1.0" }
lunatic = { "0.12.0" }
```

And then the code:

```rust,no_run
use std::collections::HashMap;

#[lunatic::main]
fn main() {
    let resp = nightfly::get("https://httpbin.org/ip")
        .unwrap()
        .json::<HashMap<String, String>>()
        .unwrap();
    println!("{:#?}", resp);
    Ok(())
}
```

## Requirements

- A running version of the [lunatic VM](https://github.com/lunatic-solutions/lunatic).

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
