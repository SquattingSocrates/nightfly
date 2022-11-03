// mod support;
// use support::*;

// #[lunatic::test]
// fn http_upgrade() {
//     let server = server::http(move |req| {
//         assert_eq!(req.method(), "GET");
//         assert_eq!(req.headers()["connection"], "upgrade");
//         assert_eq!(req.headers()["upgrade"], "foobar");

//         lunatic::spawn(async move {
//             let mut upgraded = hyper::upgrade::on(req).unwrap();

//             let mut buf = vec![0; 7];
//             upgraded.read_exact(&mut buf).unwrap();
//             assert_eq!(buf, b"foo=bar");

//             upgraded.write_all(b"bar=foo").unwrap();
//         });

//         async {
//             http::Response::builder()
//                 .status(http::StatusCode::SWITCHING_PROTOCOLS)
//                 .header(http::header::CONNECTION, "upgrade")
//                 .header(http::header::UPGRADE, "foobar")
//                 .body(hyper::Body::empty())
//                 .unwrap()
//         }
//     });

//     let res = nightfly::Client::builder()
//         .build()
//         .unwrap()
//         .get(format!("http://{}", server.addr()))
//         .header(http::header::CONNECTION, "upgrade")
//         .header(http::header::UPGRADE, "foobar")
//         .send()
//         .unwrap();

//     assert_eq!(res.status(), http::StatusCode::SWITCHING_PROTOCOLS);
//     let mut upgraded = res.upgrade().unwrap();

//     upgraded.write_all(b"foo=bar").unwrap();

//     let mut buf = vec![];
//     upgraded.read_to_end(&mut buf).unwrap();
//     assert_eq!(buf, b"bar=foo");
// }
