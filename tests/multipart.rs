// mod support;
// use support::*;

// #[lunatic::test]
// fn text_part() {
//     let _ = env_logger::try_init();

//     let form = nightfly::multipart::Form::new().text("foo", "bar");

//     let expected_body = format!(
//         "\
//          --{0}\r\n\
//          Content-Disposition: form-data; name=\"foo\"\r\n\r\n\
//          bar\r\n\
//          --{0}--\r\n\
//          ",
//         form.boundary()
//     );

//     let ct = format!("multipart/form-data; boundary={}", form.boundary());

//     let server = server::http(move |mut req| {
//         let ct = ct.clone();
//         let expected_body = expected_body.clone();
//         async move {
//             assert_eq!(req.method(), "POST");
//             assert_eq!(req.headers()["content-type"], ct);
//             assert_eq!(
//                 req.headers()["content-length"],
//                 expected_body.len().to_string()
//             );

//             let mut full: Vec<u8> = Vec::new();
//             while let Some(item) = req.body_mut().next() {
//                 full.extend(&*item.unwrap());
//             }

//             assert_eq!(full, expected_body.as_bytes());

//             http::Response::default()
//         }
//     });

//     let url = format!("http://{}/multipart/1", server.addr());

//     let res = nightfly::Client::new()
//         .post(&url)
//         .multipart(form)
//         .send()
//         .unwrap();

//     assert_eq!(res.url().as_str(), &url);
//     assert_eq!(res.status(), nightfly::StatusCode::OK);
// }

// #[cfg(feature = "blocking")]
// #[test]
// fn blocking_file_part() {
//     let _ = env_logger::try_init();

//     let form = nightfly::blocking::multipart::Form::new()
//         .file("foo", "Cargo.lock")
//         .unwrap();

//     let fcontents = std::fs::read_to_string("Cargo.lock").unwrap();

//     let expected_body = format!(
//         "\
//          --{0}\r\n\
//          Content-Disposition: form-data; name=\"foo\"; filename=\"Cargo.lock\"\r\n\
//          Content-Type: application/octet-stream\r\n\r\n\
//          {1}\r\n\
//          --{0}--\r\n\
//          ",
//         form.boundary(),
//         fcontents
//     );

//     let ct = format!("multipart/form-data; boundary={}", form.boundary());

//     let server = server::http(move |mut req| {
//         let ct = ct.clone();
//         let expected_body = expected_body.clone();
//         async move {
//             assert_eq!(req.method(), "POST");
//             assert_eq!(req.headers()["content-type"], ct);
//             // files know their exact size
//             assert_eq!(
//                 req.headers()["content-length"],
//                 expected_body.len().to_string()
//             );

//             let mut full: Vec<u8> = Vec::new();
//             while let Some(item) = req.body_mut().next() {
//                 full.extend(&*item.unwrap());
//             }

//             assert_eq!(full, expected_body.as_bytes());

//             http::Response::default()
//         }
//     });

//     let url = format!("http://{}/multipart/2", server.addr());

//     let res = nightfly::blocking::Client::new()
//         .post(&url)
//         .multipart(form)
//         .send()
//         .unwrap();

//     assert_eq!(res.url().as_str(), &url);
//     assert_eq!(res.status(), nightfly::StatusCode::OK);
// }
