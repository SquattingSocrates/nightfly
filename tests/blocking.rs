mod support;

use http::{HeaderMap, HeaderValue};
use nightfly::StatusCode;
use submillisecond::{response::Response as SubmsResponse, router, Json, RequestContext};

fn index() -> &'static str {
    "Hello"
}

fn non_utf8_text() -> SubmsResponse {
    SubmsResponse::builder()
        .header("Content-Type", "text/plain; charset=gbk")
        .body(b"\xc4\xe3\xba\xc3".to_vec())
        .unwrap()
}

fn ensure_hello(hello: String) -> SubmsResponse {
    assert_eq!(hello, "Hello".to_string());
    SubmsResponse::default()
}

fn empty_response() -> SubmsResponse {
    SubmsResponse::default()
}

fn res_400() -> (StatusCode, &'static str) {
    (StatusCode::BAD_REQUEST, "Resource not found")
}

fn get_json() -> Json<String> {
    Json("Hello".to_string())
}

fn default_headers(headers: HeaderMap) -> SubmsResponse {
    assert_eq!(
        headers.get("nightfly-test"),
        Some(&HeaderValue::from_str("orly").unwrap())
    );
    SubmsResponse::default()
}

fn overwrite_headers(headers: HeaderMap) -> SubmsResponse {
    assert_eq!(
        headers.get("authorization"),
        Some(&HeaderValue::from_str("secret").unwrap())
    );
    SubmsResponse::default()
}

fn appended_headers(headers: HeaderMap) -> SubmsResponse {
    let mut accepts = headers.get_all("accept").into_iter();
    assert_eq!(accepts.next().unwrap(), "application/json");
    assert_eq!(accepts.next().unwrap(), "application/json+hal");
    assert_eq!(accepts.next(), None);
    SubmsResponse::default()
}

static ADDR: &'static str = "0.0.0.0:3000";

static ROUTER: fn(RequestContext) -> SubmsResponse = router! {
    GET "/text" => index
    GET "/non_utf8_text" => non_utf8_text
    GET "/1" => empty_response
    POST "/2" => ensure_hello
    GET "/err_400" => res_400
    GET "/json" => get_json
    GET "/default_headers" => default_headers
    GET "/overwrite_headers" => overwrite_headers
    GET "/4" => appended_headers
};

wrap_server!(server, ROUTER, ADDR);

#[lunatic::test]
fn test_response_text() {
    let _ = server::ensure_server();

    let url = format!("http://{}/text", ADDR);
    let res = nightfly::get(&url).unwrap();
    println!(
        "RES {:?} = {:?} \n {:?} = {:?} \n {:?} = {:?}",
        res.url().as_str(),
        &url,
        res.status(),
        nightfly::StatusCode::OK,
        res.content_length(),
        Some(5)
    );
    assert_eq!(res.url().as_str(), &url);
    assert_eq!(res.status(), nightfly::StatusCode::OK);
    assert_eq!(res.content_length(), Some(5));

    let body = res.text().unwrap();
    assert_eq!(b"Hello", body.as_bytes());
}

#[lunatic::test]
fn test_response_non_utf_8_text() {
    // maybe wait for server to spawn
    let _ = server::ensure_server();

    let url = format!("http://{}/non_utf8_text", ADDR);
    let res = nightfly::get(&url).unwrap();
    assert_eq!(res.url().as_str(), &url);
    assert_eq!(res.status(), nightfly::StatusCode::OK);
    assert_eq!(res.content_length(), Some(4));

    let body = res.text().unwrap();
    assert_eq!("你好", &body);
    assert_eq!(b"\xe4\xbd\xa0\xe5\xa5\xbd", body.as_bytes()); // Now it's utf-8
}

#[lunatic::test]
// #[cfg(feature = "json")]
fn test_response_json() {
    // maybe wait for server to spawn
    let _ = server::ensure_server();

    let url = format!("http://{}/json", ADDR);
    let res = nightfly::get(&url).unwrap();
    assert_eq!(res.url().as_str(), &url);
    assert_eq!(res.status(), nightfly::StatusCode::OK);
    assert_eq!(res.content_length(), Some(7));

    let body = res.json::<String>().unwrap();
    assert_eq!("Hello", body);
}

#[lunatic::test]
fn test_get() {
    // maybe wait for server to spawn
    let _ = server::ensure_server();

    let url = format!("http://{}/1", ADDR);
    let res = nightfly::get(&url).unwrap();

    assert_eq!(res.url().as_str(), &url);
    assert_eq!(res.status(), nightfly::StatusCode::OK);
    // assert_eq!(res.remote_addr(), Some(ADDR));

    assert_eq!(res.text().unwrap().len(), 0)
}

#[lunatic::test]
fn test_post() {
    // maybe wait for server to spawn
    let _ = server::ensure_server();

    let url = format!("http://{}/2", ADDR);
    let res = nightfly::Client::new()
        .post(&url)
        .text("Hello")
        .send()
        .unwrap();

    assert_eq!(res.url().as_str(), &url);
    assert_eq!(res.status(), nightfly::StatusCode::OK);
}

// #[lunatic::test]
// fn test_post_form() {
//     let server = server::http(move |req| async move {
//         assert_eq!(req.method(), "POST");
//         assert_eq!(req.headers()["content-length"], "24");
//         assert_eq!(
//             req.headers()["content-type"],
//             "application/x-www-form-urlencoded"
//         );

//         let data = hyper::body::to_bytes(req.into_body()).unwrap();
//         assert_eq!(&*data, b"hello=world&sean=monstar");

//         http::Response::default()
//     });

//     let form = &[("hello", "world"), ("sean", "monstar")];

//     let url = format!("http://{}/form", ADDR);
//     let res = nightfly::Client::new()
//         .post(&url)
//         .form(form)
//         .send()
//         .expect("request send");

//     assert_eq!(res.url().as_str(), &url);
//     assert_eq!(res.status(), nightfly::StatusCode::OK);
// }

/// Calling `Response::error_for_status`` on a response with status in 4xx
/// returns a error.
#[lunatic::test]
fn test_error_for_status_4xx() {
    // maybe wait for server to spawn
    let _ = server::ensure_server();

    let url = format!("http://{}/err_400", ADDR);
    let res = nightfly::get(&url).unwrap();

    let err = res.error_for_status().unwrap_err();
    assert!(err.is_status());
    assert_eq!(err.status(), Some(nightfly::StatusCode::BAD_REQUEST));
}

/// Calling `Response::error_for_status`` on a response with status in 5xx
/// returns a error.
#[lunatic::test]
fn test_error_for_status_5xx() {
    // maybe wait for server to spawn
    let _ = server::ensure_server();

    let url = format!("http://{}/2", ADDR);
    let res = nightfly::Client::new()
        .post(&url)
        .text("invalid string")
        .send()
        .unwrap();

    let err = res.error_for_status().unwrap_err();
    assert!(err.is_status());
    assert_eq!(
        err.status(),
        Some(nightfly::StatusCode::INTERNAL_SERVER_ERROR)
    );
}

#[lunatic::test]
fn test_default_headers() {
    // maybe wait for server to spawn
    let _ = server::ensure_server();

    let mut headers = http::HeaderMap::with_capacity(1);
    headers.insert("nightfly-test", "orly".parse().unwrap());
    let client = nightfly::Client::builder()
        .default_headers(headers)
        .build()
        .unwrap();

    let url = format!("http://{}/default_headers", ADDR);
    let res = client.get(&url).send().unwrap();

    assert_eq!(res.url().as_str(), &url);
    assert_eq!(res.status(), nightfly::StatusCode::OK);
}

#[lunatic::test]
fn test_override_default_headers() {
    // maybe wait for server to spawn
    let _ = server::ensure_server();

    let mut headers = http::HeaderMap::with_capacity(1);
    headers.insert(
        http::header::AUTHORIZATION,
        http::header::HeaderValue::from_static("iamatoken"),
    );
    let client = nightfly::Client::builder()
        .default_headers(headers)
        .build()
        .unwrap();

    let url = format!("http://{}/overwrite_headers", ADDR);
    let res = client
        .get(&url)
        .header(
            http::header::AUTHORIZATION,
            http::header::HeaderValue::from_static("secret"),
        )
        .send()
        .unwrap();

    assert_eq!(res.url().as_str(), &url);
    assert_eq!(res.status(), nightfly::StatusCode::OK);
}

#[lunatic::test]
fn test_appended_headers_not_overwritten() {
    // maybe wait for server to spawn
    let _ = server::ensure_server();

    let client = nightfly::Client::new();

    let url = format!("http://{}/4", ADDR);
    let res = client
        .get(&url)
        .header(header::ACCEPT, "application/json")
        .header(header::ACCEPT, "application/json+hal")
        .send()
        .unwrap();

    assert_eq!(res.url().as_str(), &url);
    println!("GOT RES {:?}", res);
    assert_eq!(res.status(), nightfly::StatusCode::OK);

    // make sure this also works with default headers
    use nightfly::header;
    let mut headers = header::HeaderMap::with_capacity(1);
    headers.insert(
        header::ACCEPT,
        header::HeaderValue::from_static("text/html"),
    );
    let client = nightfly::Client::builder()
        .default_headers(headers)
        .build()
        .unwrap();

    let url = format!("http://{}/4", ADDR);
    let res = client
        .get(&url)
        .header(header::ACCEPT, "application/json")
        .header(header::ACCEPT, "application/json+hal")
        .send()
        .unwrap();

    assert_eq!(res.url().as_str(), &url);
    assert_eq!(res.status(), nightfly::StatusCode::OK);
}

// #[cfg(feature = "default-tls")]
// #[lunatic::test]
// fn test_allowed_methods_blocking() {
//     let resp = nightfly::Client::builder()
//         .https_only(true)
//         .build()
//         .expect("client builder")
//         .get("https://google.com")
//         .send();

//     assert_eq!(resp.is_err(), false);

//     let resp = nightfly::Client::builder()
//         .https_only(true)
//         .build()
//         .expect("client builder")
//         .get("http://google.com")
//         .send();

//     assert_eq!(resp.is_err(), true);
// }

/// Test that a [`nightfly::Body`] can be created from [`bytes::Bytes`].
#[lunatic::test]
fn test_body_from_bytes() {
    let body = "abc";
    // No external calls are needed. Only the request building is tested.
    let request = nightfly::Client::builder()
        .build()
        .expect("Could not build the client")
        .put("https://google.com")
        .body(bytes::Bytes::from(body))
        .build()
        .expect("Invalid body");

    let inner = request.body().unwrap().clone().inner();
    assert_eq!(&inner[..], body.as_bytes());
}
