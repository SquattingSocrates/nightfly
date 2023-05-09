#[macro_use]
pub mod support;

use submillisecond::{response::Response as SubmsResponse, router, RequestContext};
use support::RouterFn;

fn redirect(code: u16) -> SubmsResponse {
    http::Response::builder()
        .status(code)
        .header("location", "/dst")
        .header("server", "test-redirect")
        .body(Default::default())
        .unwrap()
}

fn handle_301() -> SubmsResponse {
    redirect(301)
}

fn handle_302() -> SubmsResponse {
    redirect(302)
}

fn handle_303() -> SubmsResponse {
    redirect(303)
}

fn handle_307() -> SubmsResponse {
    println!("HANDLING 307");
    redirect(307)
}

fn handle_308() -> SubmsResponse {
    redirect(308)
}

fn dst(body: Vec<u8>) -> SubmsResponse {
    SubmsResponse::builder()
        .header("server", "test-dst")
        .body(body)
        .unwrap()
}

fn dst_get() -> SubmsResponse {
    dst("GET".into())
}

fn dst_post() -> SubmsResponse {
    dst("POST".into())
}

fn end_server(req: RequestContext) -> SubmsResponse {
    lunatic_log::info!("END SERVER {:?}", req.headers());
    assert_eq!(req.headers().get("cookie"), None);

    assert_eq!(
        req.headers()["referer"],
        format!("http://{}/sensitive", ADDR)
    );
    http::Response::default()
}

fn mid_server(req: RequestContext) -> SubmsResponse {
    assert_eq!(req.headers()["cookie"], "foo=bar");
    http::Response::builder()
        .status(302)
        .header("location", format!("http://{}/end", END_ADDR))
        .body(Default::default())
        .unwrap()
}

fn loop_handler(req: RequestContext) -> SubmsResponse {
    assert_eq!(req.uri(), "/loop");
    http::Response::builder()
        .status(302)
        .header("location", "/loop")
        .body(Default::default())
        .unwrap()
}

fn no_redirect() -> SubmsResponse {
    http::Response::builder()
        .status(302)
        .header("location", "/dont")
        .body(Default::default())
        .unwrap()
}

fn no_referer() -> SubmsResponse {
    SubmsResponse::builder()
        .status(302)
        .header("location", "/dst-no-refer")
        .body(Default::default())
        .unwrap()
}

fn dst_no_referer(req: RequestContext) -> SubmsResponse {
    assert_eq!(req.uri(), "/dst");
    assert_eq!(req.headers().get("referer"), None);

    SubmsResponse::default()
}

fn yikes() -> SubmsResponse {
    http::Response::builder()
        .status(302)
        .header("location", "http://www.yikes{KABOOM}")
        .body(Default::default())
        .unwrap()
}

fn handle_302_cookie() -> SubmsResponse {
    http::Response::builder()
        .status(302)
        .header("location", "/dst")
        .header("set-cookie", "key=value")
        .body(Default::default())
        .unwrap()
}

fn dst_cookie(req: RequestContext) -> SubmsResponse {
    assert_eq!(req.headers()["cookie"], "key=value");
    http::Response::default()
}

static ROUTER: RouterFn = router! {
    POST "/301" => handle_301
    POST "/302" => handle_302
    POST "/303" => handle_303
    POST "/307" => handle_307
    POST "/308" => handle_308
    GET "/307" => handle_307
    GET "/308" => handle_308
    GET "/dst" => dst_get
    POST "/dst" => dst_post
    GET "/sensitive" => mid_server
    GET "/loop" => loop_handler
    GET "/no-redirect" => no_redirect
    GET "/no-refer" => no_referer
    GET "/dst-no-refer" => dst_no_referer
    GET "/yikes" => yikes
    GET "/dst-cookie" => dst_cookie
    GET "/302-cookie" => handle_302_cookie
};

static END_ROUTER: RouterFn = router! {
    GET "/end" => end_server
};

static ADDR: &'static str = "0.0.0.0:3000";
static END_ADDR: &'static str = "0.0.0.0:3005";

wrap_server!(server, ROUTER, ADDR);
wrap_server!(end_server, END_ROUTER, END_ADDR);

#[lunatic::test]
fn test_redirect_301_and_302_and_303_changes_post_to_get() {
    let _ = server::ensure_server();
    let client = nightfly::Client::new();
    let codes = [301u16, 302, 303];

    for &code in codes.iter() {
        let url = format!("http://{}/{}", ADDR, code);
        let dst = format!("http://{}/{}", ADDR, "dst");
        let res = client.post(&url).send().unwrap();
        println!("RES code {} -> {:?}", code, res);
        assert_eq!(res.url().as_str(), dst);
        assert_eq!(res.status(), nightfly::StatusCode::OK);
        assert_eq!(
            res.headers().get(nightfly::header::SERVER).unwrap(),
            &"test-dst"
        );
        assert_eq!(res.body, b"GET".to_vec());
    }
}

#[lunatic::test]
fn test_redirect_307_and_308_tries_to_get_again() {
    let _ = server::ensure_server();

    let client = nightfly::Client::new();
    let codes = [307u16, 308];
    for &code in codes.iter() {
        let url = format!("http://{}/{}", ADDR, code);
        let dst = format!("http://{}/{}", ADDR, "dst");
        let res = client.get(&url).send().unwrap();
        assert_eq!(res.url().as_str(), dst);
        assert_eq!(res.status(), nightfly::StatusCode::OK);
        assert_eq!(
            res.headers().get(nightfly::header::SERVER).unwrap(),
            &"test-dst"
        );
        assert_eq!(res.body, b"GET".to_vec());
    }
}

#[lunatic::test]
fn test_redirect_307_and_308_tries_to_post_again() {
    let _ = server::ensure_server();

    let client = nightfly::Client::new();
    let codes = [307u16, 308];
    for &code in codes.iter() {
        let url = format!("http://{}/{}", ADDR, code);
        let dst = format!("http://{}/{}", ADDR, "dst");
        let res = client.post(&url).body("Hello").send().unwrap();
        assert_eq!(res.url().as_str(), dst);
        assert_eq!(res.status(), nightfly::StatusCode::OK);
        assert_eq!(
            res.headers().get(nightfly::header::SERVER).unwrap(),
            &"test-dst"
        );
        assert_eq!(res.body, b"POST".to_vec());
    }
}

#[lunatic::test]
fn test_redirect_removes_sensitive_headers() {
    let _ = server::ensure_server();
    let _ = end_server::ensure_server();
    let res = nightfly::Client::builder()
        .build()
        .unwrap()
        .get(&format!("http://{}/sensitive", ADDR))
        .header(
            nightfly::header::COOKIE,
            nightfly::header::HeaderValue::from_static("foo=bar"),
        )
        .send()
        .unwrap();
    println!("SENSITIVE {:?}", res);
    assert_eq!(res.status, 200);
}

#[lunatic::test]
fn test_redirect_policy_can_return_errors() {
    let _ = server::ensure_server();

    let url = format!("http://{}/loop", ADDR);
    let err = nightfly::get(&url).unwrap_err();
    assert!(err.is_redirect());
}

#[lunatic::test]
fn test_redirect_policy_can_stop_redirects_without_an_error() {
    let _ = server::ensure_server();

    let url = format!("http://{}/no-redirect", ADDR);

    let res = nightfly::Client::builder()
        .redirect(nightfly::redirect::Policy::none())
        .build()
        .unwrap()
        .get(&url)
        .send();

    let res = res.unwrap();
    assert_eq!(res.url().as_str(), url);
    assert_eq!(res.status(), nightfly::StatusCode::FOUND);
}

#[lunatic::test]
fn test_referer_is_not_set_if_disabled() {
    let _ = server::ensure_server();

    nightfly::Client::builder()
        .referer(false)
        .build()
        .unwrap()
        .get(&format!("http://{}/no-refer", ADDR))
        .send()
        .unwrap();
}

#[lunatic::test]
fn test_invalid_location_stops_redirect_gh484() {
    let _ = server::ensure_server();

    let url = format!("http://{}/yikes", ADDR);

    let res = nightfly::get(&url).unwrap();

    assert_eq!(res.url().as_str(), url);
    assert_eq!(res.status(), nightfly::StatusCode::FOUND);
}

#[cfg(feature = "cookies")]
#[lunatic::test]
fn test_redirect_302_with_set_cookies() {
    let _ = server::ensure_server();

    let url = format!("http://{}/302-cookie", ADDR);
    let dst = format!("http://{}/{}", ADDR, "dst");

    let client = nightfly::ClientBuilder::new().build().unwrap();
    let res = client.get(&url).send().unwrap();

    assert_eq!(res.url().as_str(), dst);
    assert_eq!(res.status(), nightfly::StatusCode::OK);
}

// #[cfg(feature = "__rustls")]
// #[lunatic::test]
// #[ignore = "Needs TLS support in the test server"]
// fn test_redirect_https_only_enforced_gh1312() {
//     let server = server::http(move |_req| async move {
//         http::Response::builder()
//             .status(302)
//             .header("location", "http://insecure")
//             .body(Default::default())
//             .unwrap()
//     });

//     let url = format!("https://{}/yikes", ADDR);

//     let res = nightfly::Client::builder()
//         .danger_accept_invalid_certs(true)
//         .use_rustls_tls()
//         .https_only(true)
//         .build()
//         .expect("client builder")
//         .get(&url)
//         .send();

//     let err = res.unwrap_err();
//     assert!(err.is_redirect());
// }
