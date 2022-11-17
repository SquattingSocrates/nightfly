mod support;

use http::HeaderMap;
use nightfly::Client;

// use lunatic::net::ToSocketAddrs;
use submillisecond::{response::Response as SubmsResponse, router, RequestContext};

fn text() -> SubmsResponse {
    SubmsResponse::new("Hello".into())
}

fn user_agent(req: RequestContext) -> SubmsResponse {
    assert_eq!(req.headers()["user-agent"], "nightfly-test-agent");
    SubmsResponse::default()
}

fn auto_headers(req: RequestContext) -> SubmsResponse {
    assert_eq!(req.method(), "GET");

    assert_eq!(req.headers()["accept"], "*/*");
    assert_eq!(req.headers().get("user-agent"), None);
    if cfg!(feature = "gzip") {
        assert!(req.headers()["accept-encoding"]
            .to_str()
            .unwrap()
            .contains("gzip"));
    }
    if cfg!(feature = "brotli") {
        assert!(req.headers()["accept-encoding"]
            .to_str()
            .unwrap()
            .contains("br"));
    }
    if cfg!(feature = "deflate") {
        assert!(req.headers()["accept-encoding"]
            .to_str()
            .unwrap()
            .contains("deflate"));
    }

    http::Response::default()
}

fn get_handler() -> SubmsResponse {
    SubmsResponse::new("pipe me".into())
}

fn pipe_response(body: Vec<u8>, _headers: HeaderMap) -> SubmsResponse {
    lunatic_log::info!("BODY {:?} | header {:?}", body, _headers);
    // assert_eq!(headers["transfer-encoding"], "chunked");

    assert_eq!(body, b"pipe me".to_vec());

    SubmsResponse::default()
}

static ROUTER: fn(RequestContext) -> SubmsResponse = router! {
    GET "/text" => text
    GET "/user-agent" => user_agent
    GET "/auto_headers" => auto_headers
    GET "/get" => get_handler
    POST "/pipe" => pipe_response
};
static ADDR: &'static str = "0.0.0.0:3002";

wrap_server!(server, ROUTER, ADDR);

#[lunatic::test]
fn test_auto_headers() {
    let _ = server::ensure_server();

    println!("BEFORE CALLING AUTO_HEADERS");

    let url = format!("http://{}/auto_headers", ADDR);
    let res = nightfly::Client::builder()
        // .no_proxy()
        .build()
        .unwrap()
        .get(&url)
        .send()
        .unwrap();

    println!("AUTO HEADERS {:?}", res);
    assert_eq!(res.url().as_str(), &url);
    assert_eq!(res.status(), nightfly::StatusCode::OK);
    // assert_eq!(res.remote_addr(), ADDR.to_socket_addrs().unwrap().next());
}

#[lunatic::test]
fn test_user_agent() {
    let _ = server::ensure_server();

    let url = format!("http://{}/user-agent", ADDR);
    let res = nightfly::Client::builder()
        .user_agent("nightfly-test-agent")
        .build()
        .expect("client builder")
        .get(&url)
        .send()
        .expect("request");

    assert_eq!(res.status(), nightfly::StatusCode::OK);
}

#[lunatic::test]
fn test_response_text() {
    let _ = server::ensure_server();

    let client = Client::new();

    let res = client
        .get(&format!("http://{}/text", ADDR))
        .send()
        .expect("Failed to get");
    assert_eq!(res.content_length(), Some(5));
    let text = res.text().expect("Failed to get text");
    assert_eq!("Hello", text);
}

#[lunatic::test]
fn test_response_bytes() {
    let _ = server::ensure_server();

    let client = Client::new();

    let res = client
        .get(&format!("http://{}/text", ADDR))
        .send()
        .expect("Failed to get");
    assert_eq!(res.content_length(), Some(5));
    let bytes = res.bytes().expect("res.bytes()");
    assert_eq!("Hello", bytes);
}

#[lunatic::test]
#[cfg(feature = "json")]
fn response_json() {
    let _ = server::ensure_server();

    let server = server::http(move |_req| async { http::Response::new("\"Hello\"".into()) });

    let client = Client::new();

    let res = client
        .get(&format!("http://{}/json", ADDR))
        .send()
        .expect("Failed to get");
    let text = res.json::<String>().expect("Failed to get json");
    assert_eq!("Hello", text);
}

#[lunatic::test]
fn body_pipe_response() {
    let _ = server::ensure_server();

    let client = Client::new();

    let res1 = client
        .get(&format!("http://{}/get", ADDR))
        .send()
        .expect("get1");

    assert_eq!(res1.status(), nightfly::StatusCode::OK);
    assert_eq!(res1.content_length(), Some(7));

    println!("GOT THIS RES1 {:?}", res1.body());

    // and now ensure we can "pipe" the response to another request
    let res2 = client
        .post(&format!("http://{}/pipe", ADDR))
        .body(res1)
        .send()
        .expect("res2");

    assert_eq!(res2.status(), nightfly::StatusCode::OK);
}

// #[lunatic::test]
// fn overridden_dns_resolution_with_gai() {
//     let _ = server::ensure_server();
//     let server = server::http(move |_req| async { http::Response::new("Hello".into()) });

//     let overridden_domain = "rust-lang.org";
//     let url = format!(
//         "http://{}:{}/domain_override",
//         overridden_domain,
//         ADDR.port()
//     );
//     let client = nightfly::Client::builder()
//         .resolve(overridden_domain, ADDR)
//         .build()
//         .expect("client builder");
//     let req = client.get(&url);
//     let res = req.send().expect("request");

//     assert_eq!(res.status(), nightfly::StatusCode::OK);
//     let text = res.text().expect("Failed to get text");
//     assert_eq!("Hello", text);
// }

// #[lunatic::test]
// fn overridden_dns_resolution_with_gai_multiple() {
//     let _ = server::ensure_server();
//     let server = server::http(move |_req| async { http::Response::new("Hello".into()) });

//     let overridden_domain = "rust-lang.org";
//     let url = format!(
//         "http://{}:{}/domain_override",
//         overridden_domain,
//         ADDR.port()
//     );
//     // the server runs on IPv4 localhost, so provide both IPv4 and IPv6 and let the happy eyeballs
//     // algorithm decide which address to use.
//     let client = nightfly::Client::builder()
//         .resolve_to_addrs(
//             overridden_domain,
//             &[
//                 std::net::SocketAddr::new(
//                     std::net::IpAddr::V6(std::net::Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)),
//                     ADDR.port(),
//                 ),
//                 ADDR,
//             ],
//         )
//         .build()
//         .expect("client builder");
//     let req = client.get(&url);
//     let res = req.send().expect("request");

//     assert_eq!(res.status(), nightfly::StatusCode::OK);
//     let text = res.text().expect("Failed to get text");
//     assert_eq!("Hello", text);
// }

#[cfg(feature = "trust-dns")]
#[lunatic::test]
fn overridden_dns_resolution_with_trust_dns() {
    let _ = env_logger::builder().is_test(true).try_init();
    let server = server::http(move |_req| async { http::Response::new("Hello".into()) });

    let overridden_domain = "rust-lang.org";
    let url = format!(
        "http://{}:{}/domain_override",
        overridden_domain,
        ADDR.port()
    );
    let client = nightfly::Client::builder()
        .resolve(overridden_domain, ADDR)
        .trust_dns(true)
        .build()
        .expect("client builder");
    let req = client.get(&url);
    let res = req.send().expect("request");

    assert_eq!(res.status(), nightfly::StatusCode::OK);
    let text = res.text().expect("Failed to get text");
    assert_eq!("Hello", text);
}

#[cfg(feature = "trust-dns")]
#[lunatic::test]
fn overridden_dns_resolution_with_trust_dns_multiple() {
    let _ = env_logger::builder().is_test(true).try_init();
    let server = server::http(move |_req| async { http::Response::new("Hello".into()) });

    let overridden_domain = "rust-lang.org";
    let url = format!(
        "http://{}:{}/domain_override",
        overridden_domain,
        ADDR.port()
    );
    // the server runs on IPv4 localhost, so provide both IPv4 and IPv6 and let the happy eyeballs
    // algorithm decide which address to use.
    let client = nightfly::Client::builder()
        .resolve_to_addrs(
            overridden_domain,
            &[
                std::net::SocketAddr::new(
                    std::net::IpAddr::V6(std::net::Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)),
                    ADDR.port(),
                ),
                ADDR,
            ],
        )
        .trust_dns(true)
        .build()
        .expect("client builder");
    let req = client.get(&url);
    let res = req.send().expect("request");

    assert_eq!(res.status(), nightfly::StatusCode::OK);
    let text = res.text().expect("Failed to get text");
    assert_eq!("Hello", text);
}

#[cfg(any(feature = "native-tls", feature = "__rustls",))]
#[test]
fn use_preconfigured_tls_with_bogus_backend() {
    struct DefinitelyNotTls;

    nightfly::Client::builder()
        .use_preconfigured_tls(DefinitelyNotTls)
        .build()
        .expect_err("definitely is not TLS");
}

#[cfg(feature = "native-tls")]
#[test]
fn use_preconfigured_native_tls_default() {
    extern crate native_tls_crate;

    let tls = native_tls_crate::TlsConnector::builder()
        .build()
        .expect("tls builder");

    nightfly::Client::builder()
        .use_preconfigured_tls(tls)
        .build()
        .expect("preconfigured default tls");
}

#[cfg(feature = "__rustls")]
#[test]
fn use_preconfigured_rustls_default() {
    extern crate rustls;

    let root_cert_store = rustls::RootCertStore::empty();
    let tls = rustls::ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(root_cert_store)
        .with_no_client_auth();

    nightfly::Client::builder()
        .use_preconfigured_tls(tls)
        .build()
        .expect("preconfigured rustls tls");
}

#[cfg(feature = "__rustls")]
#[lunatic::test]
#[ignore = "Needs TLS support in the test server"]
fn http2_upgrade() {
    let server = server::http(move |_| async move { http::Response::default() });

    let url = format!("https://localhost:{}", ADDR.port());
    let res = nightfly::Client::builder()
        .danger_accept_invalid_certs(true)
        .use_rustls_tls()
        .build()
        .expect("client builder")
        .get(&url)
        .send()
        .expect("request");

    assert_eq!(res.status(), nightfly::StatusCode::OK);
    assert_eq!(res.version(), nightfly::Version::HTTP_2);
}

#[lunatic::test]
fn test_allowed_methods() {
    let resp = nightfly::Client::builder()
        .https_only(true)
        .build()
        .expect("client builder")
        .get("https://google.com")
        .send();

    assert!(resp.is_ok());

    let resp = nightfly::Client::builder()
        .https_only(true)
        .build()
        .expect("client builder")
        .get("http://google.com")
        .send();

    assert!(resp.is_err());
}
