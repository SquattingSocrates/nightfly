mod support;

use submillisecond::{response::Response as SubmsResponse, router, RequestContext};

fn max_age(req: RequestContext) -> SubmsResponse {
    assert_eq!(req.headers().get("cookie"), None);
    http::Response::builder()
        .header("Set-Cookie", "key=val; Max-Age=0")
        .body(Default::default())
        .unwrap()
}

fn cookie_overwrite(req: RequestContext) -> SubmsResponse {
    if req.uri() == "/overwrite" {
        http::Response::builder()
            .header("Set-Cookie", "key=val")
            .body(Default::default())
            .unwrap()
    } else if req.uri() == "/overwrite/2" {
        assert_eq!(req.headers()["cookie"], "key=val");
        http::Response::builder()
            .header("Set-Cookie", "key=val2")
            .body(Default::default())
            .unwrap()
    } else {
        assert_eq!(req.uri(), "/overwrite/3");
        assert_eq!(req.headers()["cookie"], "key=val2");
        SubmsResponse::default()
    }
}

fn cookie_simple(req: RequestContext) -> SubmsResponse {
    if req.uri() == "/2" {
        assert_eq!(req.headers()["cookie"], "key=val");
    }
    http::Response::builder()
        .header("Set-Cookie", "key=val; HttpOnly")
        .body(Default::default())
        .unwrap()
}

fn cookie_response() -> SubmsResponse {
    SubmsResponse::builder()
        .header("Set-Cookie", "key=val")
        .header(
            "Set-Cookie",
            "expires=1; Expires=Wed, 21 Oct 2015 07:28:00 GMT",
        )
        .header("Set-Cookie", "path=1; Path=/the-path")
        .header("Set-Cookie", "maxage=1; Max-Age=100")
        .header("Set-Cookie", "domain=1; Domain=mydomain")
        .header("Set-Cookie", "secure=1; Secure")
        .header("Set-Cookie", "httponly=1; HttpOnly")
        .header("Set-Cookie", "samesitelax=1; SameSite=Lax")
        .header("Set-Cookie", "samesitestrict=1; SameSite=Strict")
        .body(Default::default())
        .unwrap()
}

fn expires(req: RequestContext) -> SubmsResponse {
    assert_eq!(req.headers().get("cookie"), None);
    http::Response::builder()
        .header(
            "Set-Cookie",
            "key=val; Expires=Wed, 21 Oct 2015 07:28:00 GMT",
        )
        .body(Default::default())
        .unwrap()
}

fn path(req: RequestContext) -> SubmsResponse {
    if req.uri() == "/path" {
        assert_eq!(req.headers().get("cookie"), None);
        SubmsResponse::builder()
            .header("Set-Cookie", "key=val; Path=/subpath")
            .body(Default::default())
            .unwrap()
    } else {
        assert_eq!(req.uri(), "/subpath");
        assert_eq!(req.headers()["cookie"], "key=val");
        SubmsResponse::default()
    }
}

static ROUTER: fn(RequestContext) -> SubmsResponse = router! {
    GET "/" => cookie_response
    GET "/1" => cookie_simple
    GET "/2" => cookie_simple
    GET "/overwrite" => cookie_overwrite
    GET "/overwrite/2" => cookie_overwrite
    GET "/overwrite/3" => cookie_overwrite
    GET "/max-age" => max_age
    GET "/expires" => expires
    GET "/path" => path
    GET "/subpath" => path
};

static ADDR: &'static str = "0.0.0.0:3000";

wrap_server!(server, ROUTER, ADDR);

#[lunatic::test]
fn cookie_response_accessor() {
    let _ = server::ensure_server();

    let client = nightfly::Client::new();

    let url = format!("http://{}/", ADDR);
    let res = client.get(&url).send().unwrap();

    let cookies = res.cookies().collect::<Vec<_>>();

    // key=val
    assert_eq!(cookies[0].name(), "key");
    assert_eq!(cookies[0].value(), "val");

    // expires
    assert_eq!(cookies[1].name(), "expires");
    assert_eq!(
        cookies[1].expires().unwrap(),
        std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1445412480)
    );

    // path
    assert_eq!(cookies[2].name(), "path");
    assert_eq!(cookies[2].path().unwrap(), "/the-path");

    // max-age
    assert_eq!(cookies[3].name(), "maxage");
    assert_eq!(
        cookies[3].max_age().unwrap(),
        std::time::Duration::from_secs(100)
    );

    // domain
    assert_eq!(cookies[4].name(), "domain");
    assert_eq!(cookies[4].domain().unwrap(), "mydomain");

    // secure
    assert_eq!(cookies[5].name(), "secure");
    assert_eq!(cookies[5].secure(), true);

    // httponly
    assert_eq!(cookies[6].name(), "httponly");
    assert_eq!(cookies[6].http_only(), true);

    // samesitelax
    assert_eq!(cookies[7].name(), "samesitelax");
    assert!(cookies[7].same_site_lax());

    // samesitestrict
    assert_eq!(cookies[8].name(), "samesitestrict");
    assert!(cookies[8].same_site_strict());
}

#[lunatic::test]
fn cookie_store_simple() {
    let _ = server::ensure_server();

    let client = nightfly::Client::builder()
        .cookie_store(true)
        .build()
        .unwrap();

    let url = format!("http://{}/1", ADDR);
    client.get(&url).send().unwrap();

    let url = format!("http://{}/2", ADDR);
    client.get(&url).send().unwrap();
}

#[lunatic::test]
fn cookie_store_overwrite_existing() {
    let _ = server::ensure_server();

    let client = nightfly::Client::builder()
        .cookie_store(true)
        .build()
        .unwrap();

    let url = format!("http://{}/overwrite", ADDR);
    client.get(&url).send().unwrap();

    let url = format!("http://{}/overwrite/2", ADDR);
    client.get(&url).send().unwrap();

    let url = format!("http://{}/overwrite/3", ADDR);
    client.get(&url).send().unwrap();
}

#[lunatic::test]
fn cookie_store_max_age() {
    let _ = server::ensure_server();

    let client = nightfly::Client::builder()
        .cookie_store(true)
        .build()
        .unwrap();
    let url = format!("http://{}/max-age", ADDR);
    client.get(&url).send().unwrap();
    client.get(&url).send().unwrap();
}

#[lunatic::test]
fn cookie_store_expires() {
    let _ = server::ensure_server();

    let client = nightfly::Client::builder()
        .cookie_store(true)
        .build()
        .unwrap();

    let url = format!("http://{}/expires", ADDR);
    client.get(&url).send().unwrap();
    client.get(&url).send().unwrap();
}

#[lunatic::test]
fn cookie_store_path() {
    let _ = server::ensure_server();

    let client = nightfly::Client::builder()
        .cookie_store(true)
        .build()
        .unwrap();

    let url = format!("http://{}/path", ADDR);
    client.get(&url).send().unwrap();
    client.get(&url).send().unwrap();

    let url = format!("http://{}/subpath", ADDR);
    client.get(&url).send().unwrap();
}
