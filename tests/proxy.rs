// mod support;
// use support::*;

// use std::env;

// use nightfly::Client;

// use lunatic::{
//     abstract_process,
//     net::ToSocketAddrs,
//     process::{ProcessRef, StartProcess},
//     spawn_link,
//     supervisor::{Supervisor, SupervisorStrategy},
//     Process, Tag,
// };
// use submillisecond::{response::Response as SubmsResponse, router, Application, RequestContext};

// struct ServerSup;

// struct ServerProcess(Process<()>);

// #[abstract_process]
// impl ServerProcess {
//     #[init]
//     fn init(_: ProcessRef<Self>, _: ()) -> Self {
//         Self(spawn_link!(|| {
//             start_server().unwrap();
//         }))
//     }

//     #[terminate]
//     fn terminate(self) {
//         println!("Shutdown process");
//     }

//     #[handle_link_trapped]
//     fn handle_link_trapped(&self, _: Tag) {
//         println!("Link trapped");
//     }
// }

// impl Supervisor for ServerSup {
//     type Arg = String;
//     type Children = ServerProcess;

//     fn init(config: &mut lunatic::supervisor::SupervisorConfig<Self>, name: Self::Arg) {
//         // If a child fails, just restart it.
//         config.set_strategy(SupervisorStrategy::OneForOne);
//         // Start One `ServerProcess`
//         config.children_args(((), Some(name)));
//     }
// }

// fn max_age(req: RequestContext) -> SubmsResponse {
//     assert_eq!(req.headers().get("cookie"), None);
//     http::Response::builder()
//         .header("Set-Cookie", "key=val; Max-Age=0")
//         .body(Default::default())
//         .unwrap()
// }

// fn text() -> SubmsResponse {
//     SubmsResponse::new("Hello".into())
// }

// fn user_agent(req: RequestContext) -> SubmsResponse {
//     assert_eq!(req.headers()["user-agent"], "nightfly-test-agent");
//     SubmsResponse::default()
// }

// fn auto_headers(req: RequestContext) -> SubmsResponse {
//     assert_eq!(req.method(), "GET");

//     assert_eq!(req.headers()["accept"], "*/*");
//     assert_eq!(req.headers().get("user-agent"), None);
//     if cfg!(feature = "gzip") {
//         assert!(req.headers()["accept-encoding"]
//             .to_str()
//             .unwrap()
//             .contains("gzip"));
//     }
//     if cfg!(feature = "brotli") {
//         assert!(req.headers()["accept-encoding"]
//             .to_str()
//             .unwrap()
//             .contains("br"));
//     }
//     if cfg!(feature = "deflate") {
//         assert!(req.headers()["accept-encoding"]
//             .to_str()
//             .unwrap()
//             .contains("deflate"));
//     }

//     http::Response::default()
// }

// fn get_handler(req: RequestContext) -> SubmsResponse {
//     SubmsResponse::new("pipe me".into())
// }

// fn pipe_response(req: RequestContext) -> SubmsResponse {
//     assert_eq!(req.headers()["transfer-encoding"], "chunked");

//     let body = req.body().as_slice();
//     assert_eq!(body, b"pipe me".to_vec());

//     SubmsResponse::default()
// }

// fn start_server() -> std::io::Result<()> {
//     Application::new(router! {
//         // GET "/" => cookie_response
//         GET "/text" => text
//         GET "/user-agent" => user_agent
//         GET "/auto_headers" => auto_headers
//         GET "/get" => get_handler
//         GET "/pipe" => pipe_response
//     })
//     .serve(ADDR)
// }

// static ADDR: &'static str = "0.0.0.0:3000";

// fn ensure_server() {
//     if let Some(_) = Process::<Process<()>>::lookup("__server__") {
//         return;
//     }
//     ServerSup::start("__server__".to_owned(), None);
// }

// #[lunatic::test]
// fn http_proxy() {
//     let url = "http://hyper.rs/prox";
//     let server = server::http(move |req| {
//         assert_eq!(req.method(), "GET");
//         assert_eq!(req.uri(), url);
//         assert_eq!(req.headers()["host"], "hyper.rs");

//         async { http::Response::default() }
//     });

//     let proxy = format!("http://{}", server.addr());

//     let res = nightfly::Client::builder()
//         .proxy(nightfly::Proxy::http(&proxy).unwrap())
//         .build()
//         .unwrap()
//         .get(url)
//         .send()
//         .unwrap();

//     assert_eq!(res.url().as_str(), url);
//     assert_eq!(res.status(), nightfly::StatusCode::OK);
// }

// #[lunatic::test]
// fn http_proxy_basic_auth() {
//     let url = "http://hyper.rs/prox";
//     let server = server::http(move |req| {
//         assert_eq!(req.method(), "GET");
//         assert_eq!(req.uri(), url);
//         assert_eq!(req.headers()["host"], "hyper.rs");
//         assert_eq!(
//             req.headers()["proxy-authorization"],
//             "Basic QWxhZGRpbjpvcGVuIHNlc2FtZQ=="
//         );

//         async { http::Response::default() }
//     });

//     let proxy = format!("http://{}", server.addr());

//     let res = nightfly::Client::builder()
//         .proxy(
//             nightfly::Proxy::http(&proxy)
//                 .unwrap()
//                 .basic_auth("Aladdin", "open sesame"),
//         )
//         .build()
//         .unwrap()
//         .get(url)
//         .send()
//         .unwrap();

//     assert_eq!(res.url().as_str(), url);
//     assert_eq!(res.status(), nightfly::StatusCode::OK);
// }

// #[lunatic::test]
// fn http_proxy_basic_auth_parsed() {
//     let url = "http://hyper.rs/prox";
//     let server = server::http(move |req| {
//         assert_eq!(req.method(), "GET");
//         assert_eq!(req.uri(), url);
//         assert_eq!(req.headers()["host"], "hyper.rs");
//         assert_eq!(
//             req.headers()["proxy-authorization"],
//             "Basic QWxhZGRpbjpvcGVuIHNlc2FtZQ=="
//         );

//         async { http::Response::default() }
//     });

//     let proxy = format!("http://Aladdin:open sesame@{}", server.addr());

//     let res = nightfly::Client::builder()
//         .proxy(nightfly::Proxy::http(&proxy).unwrap())
//         .build()
//         .unwrap()
//         .get(url)
//         .send()
//         .unwrap();

//     assert_eq!(res.url().as_str(), url);
//     assert_eq!(res.status(), nightfly::StatusCode::OK);
// }

// #[lunatic::test]
// fn system_http_proxy_basic_auth_parsed() {
//     let url = "http://hyper.rs/prox";
//     let server = server::http(move |req| {
//         assert_eq!(req.method(), "GET");
//         assert_eq!(req.uri(), url);
//         assert_eq!(req.headers()["host"], "hyper.rs");
//         assert_eq!(
//             req.headers()["proxy-authorization"],
//             "Basic QWxhZGRpbjpvcGVuIHNlc2FtZQ=="
//         );

//         async { http::Response::default() }
//     });

//     // save system setting first.
//     let system_proxy = env::var("http_proxy");

//     // set-up http proxy.
//     env::set_var(
//         "http_proxy",
//         format!("http://Aladdin:open sesame@{}", server.addr()),
//     );

//     let res = nightfly::Client::builder()
//         .build()
//         .unwrap()
//         .get(url)
//         .send()
//         .unwrap();

//     assert_eq!(res.url().as_str(), url);
//     assert_eq!(res.status(), nightfly::StatusCode::OK);

//     // reset user setting.
//     match system_proxy {
//         Err(_) => env::remove_var("http_proxy"),
//         Ok(proxy) => env::set_var("http_proxy", proxy),
//     }
// }

// #[lunatic::test]
// fn test_no_proxy() {
//     let server = server::http(move |req| {
//         assert_eq!(req.method(), "GET");
//         assert_eq!(req.uri(), "/4");

//         async { http::Response::default() }
//     });
//     let proxy = format!("http://{}", server.addr());
//     let url = format!("http://{}/4", server.addr());

//     // set up proxy and use no_proxy to clear up client builder proxies.
//     let res = nightfly::Client::builder()
//         .proxy(nightfly::Proxy::http(&proxy).unwrap())
//         .no_proxy()
//         .build()
//         .unwrap()
//         .get(&url)
//         .send()
//         .unwrap();

//     assert_eq!(res.url().as_str(), &url);
//     assert_eq!(res.status(), nightfly::StatusCode::OK);
// }

// #[cfg_attr(not(feature = "__internal_proxy_sys_no_cache"), ignore)]
// #[lunatic::test]
// fn test_using_system_proxy() {
//     let url = "http://not.a.real.sub.hyper.rs/prox";
//     let server = server::http(move |req| {
//         assert_eq!(req.method(), "GET");
//         assert_eq!(req.uri(), url);
//         assert_eq!(req.headers()["host"], "not.a.real.sub.hyper.rs");

//         async { http::Response::default() }
//     });

//     // Note: we're relying on the `__internal_proxy_sys_no_cache` feature to
//     // check the environment every time.

//     // save system setting first.
//     let system_proxy = env::var("http_proxy");
//     // set-up http proxy.
//     env::set_var("http_proxy", format!("http://{}", server.addr()));

//     // system proxy is used by default
//     let res = nightfly::get(url).unwrap();

//     assert_eq!(res.url().as_str(), url);
//     assert_eq!(res.status(), nightfly::StatusCode::OK);

//     // reset user setting.
//     match system_proxy {
//         Err(_) => env::remove_var("http_proxy"),
//         Ok(proxy) => env::set_var("http_proxy", proxy),
//     }
// }

// #[lunatic::test]
// fn http_over_http() {
//     let url = "http://hyper.rs/prox";

//     let server = server::http(move |req| {
//         assert_eq!(req.method(), "GET");
//         assert_eq!(req.uri(), url);
//         assert_eq!(req.headers()["host"], "hyper.rs");

//         async { http::Response::default() }
//     });

//     let proxy = format!("http://{}", server.addr());

//     let res = nightfly::Client::builder()
//         .proxy(nightfly::Proxy::http(&proxy).unwrap())
//         .build()
//         .unwrap()
//         .get(url)
//         .send()
//         .unwrap();

//     assert_eq!(res.url().as_str(), url);
//     assert_eq!(res.status(), nightfly::StatusCode::OK);
// }
