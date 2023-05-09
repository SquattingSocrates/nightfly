mod support;

use submillisecond::{response::Response as SubmsResponse, router, RequestContext};
use support::RouterFn;

fn deflate(req: RequestContext) -> SubmsResponse {
    assert_eq!(req.method(), "HEAD");

    SubmsResponse::builder()
        .header("content-encoding", "deflate")
        .header("content-length", 100)
        .body(Default::default())
        .unwrap()
}

fn accept(req: RequestContext) -> SubmsResponse {
    assert_eq!(req.headers()["accept"], "application/json");
    assert!(req.headers()["accept-encoding"]
        .to_str()
        .unwrap()
        .contains("deflate"));
    SubmsResponse::default()
}

fn accept_encoding(req: RequestContext) -> SubmsResponse {
    assert_eq!(req.headers()["accept"], "*/*");
    assert_eq!(req.headers()["accept-encoding"], "identity");
    SubmsResponse::default()
}

static ADDR: &'static str = "0.0.0.0:3001";

static ROUTER: RouterFn = router! {
    HEAD "/deflate" => deflate
    GET "/accept" => accept
    GET "/accept-encoding" => accept_encoding
};

wrap_server!(deflate_server, ROUTER, ADDR);

// ====================================
// Test cases
// ====================================

#[lunatic::test]
fn test_deflate_empty_body() {
    let _ = deflate_server::ensure_server();

    let client = nightfly::Client::new();
    let res = client
        .head(&format!("http://{}/deflate", ADDR))
        .send()
        .unwrap();

    let body = res.text().unwrap();

    assert_eq!(body, "");
}

#[lunatic::test]
fn test_accept_header_is_not_changed_if_set() {
    let _ = deflate_server::ensure_server();

    let client = nightfly::Client::new();

    let res = client
        .get(&format!("http://{}/accept", ADDR))
        .header(
            nightfly::header::ACCEPT,
            nightfly::header::HeaderValue::from_static("application/json"),
        )
        .send()
        .unwrap();

    assert_eq!(res.status(), nightfly::StatusCode::OK);
}

#[lunatic::test]
fn test_accept_encoding_header_is_not_changed_if_set() {
    let _ = deflate_server::ensure_server();

    let client = nightfly::Client::new();

    let res = client
        .get(&format!("http://{}/accept-encoding", ADDR))
        .header(
            nightfly::header::ACCEPT_ENCODING,
            nightfly::header::HeaderValue::from_static("identity"),
        )
        .send()
        .unwrap();

    assert_eq!(res.status(), nightfly::StatusCode::OK);
}
