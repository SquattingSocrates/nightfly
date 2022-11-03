// mod support;

// use std::time::Duration;

// use lunatic::{
//     abstract_process,
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

// fn slow() -> SubmsResponse {
//     // delay returning the response
//     lunatic::sleep(Duration::from_secs(2));
//     SubmsResponse::default()
// }

// fn start_server() -> std::io::Result<()> {
//     Application::new(router! {
//         GET "/slow" => slow
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
// fn client_timeout() {
//     let _ = ensure_server();

//     let client = nightfly::Client::builder()
//         .timeout(Duration::from_millis(500))
//         .build()
//         .unwrap();

//     let url = format!("http://{}/slow", ADDR);

//     let res = client.get(&url).send();

//     let err = res.unwrap_err();

//     assert!(err.is_timeout());
//     assert_eq!(err.url().map(|u| u.as_str()), Some(url.as_str()));
// }

// #[lunatic::test]
// fn request_timeout() {
//     let _ = ensure_server();

//     let client = nightfly::Client::builder().build().unwrap();

//     let url = format!("http://{}/slow", ADDR);

//     let res = client.get(&url).timeout(Duration::from_millis(500)).send();

//     let err = res.unwrap_err();

//     assert!(err.is_timeout());
//     assert_eq!(err.url().map(|u| u.as_str()), Some(url.as_str()));
// }

// #[lunatic::test]
// fn connect_timeout() {
//     let client = nightfly::Client::builder()
//         .connect_timeout(Duration::from_millis(100))
//         .build()
//         .unwrap();

//     let url = "http://10.255.255.1:81/slow";

//     let res = client.get(url).timeout(Duration::from_millis(1000)).send();

//     let err = res.unwrap_err();

//     assert!(err.is_timeout());
// }

// // #[lunatic::test]
// // fn response_timeout() {
// //     let _ = ensure_server();

// //     let server = server::http(move |_req| {
// //         async {
// //             // immediate response, but delayed body
// //             lunatic::sleep(Duration::from_secs(2));
// //             let body = Ok::<_, std::convert::Infallible>("Hello");

// //             http::Response::new(body)
// //         }
// //     });

// //     let client = nightfly::Client::builder()
// //         .timeout(Duration::from_millis(500))
// //         .no_proxy()
// //         .build()
// //         .unwrap();

// //     let url = format!("http://{}/slow", ADDR);
// //     let res = client.get(&url).send().expect("Failed to get");
// //     let body = res.text();

// //     let err = body.unwrap_err();

// //     assert!(err.is_timeout());
// // }

// // /// Tests that internal client future cancels when the oneshot channel
// // /// is canceled.
// // #[test]
// // fn timeout_closes_connection() {
// //     let _ = env_logger::try_init();

// //     // Make Client drop *after* the Server, so the background doesn't
// //     // close too early.
// //     let client = nightfly::blocking::Client::builder()
// //         .timeout(Duration::from_millis(500))
// //         .build()
// //         .unwrap();

// //     let server = server::http(move |_req| {
// //         async {
// //             // delay returning the response
// //             lunatic::time::sleep(Duration::from_secs(2));
// //             http::Response::default()
// //         }
// //     });

// //     let url = format!("http://{}/closes", ADDR);
// //     let err = client.get(&url).send().unwrap_err();

// //     assert!(err.is_timeout());
// //     assert_eq!(err.url().map(|u| u.as_str()), Some(url.as_str()));
// // }

// #[cfg(feature = "blocking")]
// #[test]
// fn timeout_blocking_request() {
//     let _ = env_logger::try_init();

//     // Make Client drop *after* the Server, so the background doesn't
//     // close too early.
//     let client = nightfly::blocking::Client::builder().build().unwrap();

//     let server = server::http(move |_req| {
//         async {
//             // delay returning the response
//             lunatic::time::sleep(Duration::from_secs(2));
//             http::Response::default()
//         }
//     });

//     let url = format!("http://{}/closes", ADDR);
//     let err = client
//         .get(&url)
//         .timeout(Duration::from_millis(500))
//         .send()
//         .unwrap_err();

//     assert!(err.is_timeout());
//     assert_eq!(err.url().map(|u| u.as_str()), Some(url.as_str()));
// }

// #[cfg(feature = "blocking")]
// #[test]
// fn blocking_request_timeout_body() {
//     let _ = env_logger::try_init();

//     let client = nightfly::blocking::Client::builder()
//         // this should be overridden
//         .connect_timeout(Duration::from_millis(200))
//         // this should be overridden
//         .timeout(Duration::from_millis(200))
//         .build()
//         .unwrap();

//     let server = server::http(move |_req| {
//         async {
//             // immediate response, but delayed body
//             let body = hyper::Body::wrap_stream(futures_util::stream::once(async {
//                 lunatic::time::sleep(Duration::from_secs(1));
//                 Ok::<_, std::convert::Infallible>("Hello")
//             }));

//             http::Response::new(body)
//         }
//     });

//     let url = format!("http://{}/closes", ADDR);
//     let res = client
//         .get(&url)
//         // longer than client timeout
//         .timeout(Duration::from_secs(5))
//         .send()
//         .expect("get response");

//     let text = res.text().unwrap();
//     assert_eq!(text, "Hello");
// }

// #[cfg(feature = "blocking")]
// #[test]
// fn write_timeout_large_body() {
//     let _ = env_logger::try_init();
//     let body = vec![b'x'; 20_000];
//     let len = 8192;

//     // Make Client drop *after* the Server, so the background doesn't
//     // close too early.
//     let client = nightfly::blocking::Client::builder()
//         .timeout(Duration::from_millis(500))
//         .build()
//         .unwrap();

//     let server = server::http(move |_req| {
//         async {
//             // delay returning the response
//             lunatic::time::sleep(Duration::from_secs(2));
//             http::Response::default()
//         }
//     });

//     let cursor = std::io::Cursor::new(body);
//     let url = format!("http://{}/write-timeout", ADDR);
//     let err = client
//         .post(&url)
//         .body(nightfly::blocking::Body::sized(cursor, len as u64))
//         .send()
//         .unwrap_err();

//     assert!(err.is_timeout());
//     assert_eq!(err.url().map(|u| u.as_str()), Some(url.as_str()));
// }
