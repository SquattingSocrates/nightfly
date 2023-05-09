#[macro_use]
pub mod support;

use submillisecond::{response::Response as SubmsResponse, router, RequestContext};
use support::RouterFn;

static CHUNKS: [&str; 10] = [
    "4\r\n",
    "Wiki\r\n",
    "6\r\n",
    "pedia \r\n",
    "E\r\n",
    "in \r\n",
    "\r\n",
    "chunks.\r\n",
    "0\r\n",
    "\r\n",
];

// original gzip for reference
// static GZIP_STRING: &str = "H4sIAAAAAAAACgvPzM4sSE3JTFTIzFPg4krOKM3LLtYDAFW43D4WAAAA";
// static GZIP: [u8; 44] = [
//     // length = 22; 0x16
//     31, 139, 8, 0, 0, 0, 0, 0, 4, 255, 11, 207, 204, 206, 44, 72, 77, 201, 76, 84, 200, 204,
//     // length = 21; 0x15
//     83, 224, 229, 226, 229, 74, 206, 40, 205, 203, 46, 214, 3, 0, 102, 210, 154, 109, 24, 0, 0,
//     // length = 1; 0x1
//     0,
// ];
#[rustfmt::skip]
static GZIP_CHUNKED: [u8; 66] = [
    b'1', b'6', b'\r', b'\n',
    31, 139, 8, 0, 0, 0, 0, 0, 4, 255, 11, 207, 204, 206, 44, 72, 77, 201, 76, 84, 200, 204, b'\r', b'\n',
    b'1', b'5', b'\r', b'\n',
    83, 224, 229, 226, 229, 74, 206, 40, 205, 203, 46, 214, 3, 0, 102, 210, 154, 109, 24, 0, 0, b'\r', b'\n',
    // single byte in last chunk
    b'1', b'\r', b'\n',
    0, b'\r', b'\n', 
    // zero length chunk
    b'0', b'\r', b'\n',
    // end of data
    b'\r', b'\n',
];

// original buffer for comparison
// static DEFLATE_ORIG: &str = "C8/MzixITclMVMjMU+DiSs4ozcsu1gMA";
// static DEFLATE: [u8; 24] = [
//     11, 207, 204, 206, 44, 72, 77, 201, 76, 84, 200, 204, 83, 224, 226, 74, 206, 40, 205, 203, 46,
//     214, 3, 0,
// ];

#[rustfmt::skip]
static DEFLATE_CHUNKED: [u8; 40] = [
    b'1', b'5', b'\r', b'\n',
    11, 207, 204, 206, 44, 72, 77, 201, 76, 84, 200, 204, 83, 224, 226, 74, 206, 40, 205, 203, 46,  b'\r', b'\n', 
    // three bytes in last chunk
    b'3', b'\r', b'\n',
    214, 3, 0, b'\r', b'\n', 
    // zero length chunk
    b'0', b'\r', b'\n', // end of data
    b'\r', b'\n',
];

fn chunked() -> SubmsResponse {
    SubmsResponse::builder()
        .header("Transfer-Encoding", "chunked")
        .header("Content-Length", "24")
        .body(CHUNKS.join("").to_owned().into_bytes())
        .unwrap()
}

fn chunked_gzip() -> SubmsResponse {
    SubmsResponse::builder()
        .header("Transfer-Encoding", "chunked")
        .header("Content-encoding", "gzip")
        .body(GZIP_CHUNKED.to_vec())
        .unwrap()
}

fn chunked_deflate() -> SubmsResponse {
    SubmsResponse::builder()
        .header("Transfer-Encoding", "chunked")
        .header("Content-encoding", "deflate")
        .body(DEFLATE_CHUNKED.to_vec())
        .unwrap()
}

static ADDR: &'static str = "0.0.0.0:3001";

static ROUTER: RouterFn = router! {
    GET "/chunked" => chunked
    GET "/gzip" => chunked_gzip
    GET "/deflate" => chunked_deflate
};

wrap_server!(chunked_server, ROUTER, ADDR);

#[lunatic::test]
fn test_chunked_uncompressed_body() {
    let _ = chunked_server::ensure_server();

    let client = nightfly::Client::new();
    let res = client
        .get(&format!("http://{}/chunked", ADDR))
        .send()
        .unwrap();

    let body = res.text().unwrap();

    assert_eq!(body, "Wikipedia in \r\n\r\nchunks.");
}

#[lunatic::test]
fn test_chunked_gzip_body() {
    let _ = chunked_server::ensure_server();

    let client = nightfly::Client::new();
    let res = client.get(&format!("http://{}/gzip", ADDR)).send().unwrap();

    let body = res.text().unwrap();

    assert_eq!(body, "Wikipedia in \r\n\r\nchunks.");
}

// #[lunatic::test]
// fn test_chunked_deflate_body() {
//     let _ = chunked_server::ensure_server();

//     let client = nightfly::Client::new();
//     let res = client
//         .get(&format!("http://{}/deflate", ADDR))
//         .send()
//         .unwrap();

//     let body = res.text().unwrap();

//     assert_eq!(body, "Wikipedia in \r\n\r\nchunks.");
// }
