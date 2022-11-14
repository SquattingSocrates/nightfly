use std::io::{Cursor, Read, Write};

use flate2::{
    read::{GzDecoder, GzEncoder},
    Compression,
};
use lunatic::{spawn_link, Mailbox};
use nightfly::Client;
use submillisecond::{response::Response as SubmsResponse, router, Application, RequestContext};

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

// static GZIP: &str = "eJwLz8zOLEhNyUxUyMxTiCmKyQPh5IzSvOxiPQCZQQqZ";
static GZIP: [u8; 44] = [
    // length = 22; 0x16
    31, 139, 8, 0, 0, 0, 0, 0, 4, 255, 11, 207, 204, 206, 44, 72, 77, 201, 76, 84, 200, 204,
    // length = 21; 0x15
    83, 224, 229, 226, 229, 74, 206, 40, 205, 203, 46, 214, 3, 0, 102, 210, 154, 109, 24, 0, 0,
    // length = 1; 0x1
    0,
];
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

static DEFLATE: [u8; 24] = [
    11, 207, 204, 206, 44, 72, 77, 201, 76, 84, 200, 204, 83, 224, 226, 74, 206, 40, 205, 203, 46,
    214, 3, 0,
];

#[rustfmt::skip]
static DEFLATE_CHUNKED: [u8; 38] = [
    b'1', b'5', b'\r', b'\n',
    11, 207, 204, 206, 44, 72, 77, 201, 76, 84, 200, 204, 83, 224, 226, 74, 206, 40, 205, 203, 46,
    // three bytes in last chunk
    b'3', b'\r', b'\n',
    214, 3, 0, b'\r', b'\n', 
    // zero length chunk
    b'0', b'\r', b'\n', // end of data
    b'\r', b'\n',
];

fn chunked(req: RequestContext) -> SubmsResponse {
    // assert_eq!(req.headers()["accept"], "*/*");
    // assert_eq!(req.headers()["accept-encoding"], "identity");
    SubmsResponse::builder()
        .header("Transfer-Encoding", "chunked")
        .header("Content-Length", "24")
        .body(CHUNKS.join("").to_owned().into_bytes())
        .unwrap()
}

fn chunked_gzip() -> SubmsResponse {
    // let mut decoder = GzDecoder::new(&GZIP_CHUNKED[..]);
    // let mut s = String::new();
    // decoder.read_to_string(&mut s).unwrap();
    // println!("GZIP DECODED {:?}", s);
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

static ROUTER: fn(RequestContext) -> SubmsResponse = router! {
    GET "/chunked" => chunked
    GET "/gzip" => chunked_gzip
    GET "/deflate" => chunked_deflate
};

fn start_server() {
    Application::new(ROUTER).serve(ADDR).unwrap();
}

#[lunatic::main]
fn main(_: Mailbox<()>) -> () {
    spawn_link!(|| { start_server() });
    let mut ret_vec = Vec::new();
    let mut input = "Wikipedia in \r\n\r\nchunks.".to_string().into_bytes();
    let mut gz = GzEncoder::new(&input[..], Compression::fast());
    let count = gz.read_to_end(&mut ret_vec).unwrap();
    println!("ENCODED GZ {:?}", ret_vec);
    // Some simple CLI args requirements...
    let url = match std::env::args().nth(1) {
        Some(url) => url,
        None => {
            println!("No CLI URL provided, using default.");
            "http://0.0.0.0:3001/gzip".into()
        }
    };

    eprintln!("Fetching {:?}...", url);

    // nightfly::get() is a convenience function.
    //
    // In most cases, you should create/build a nightfly::Client and reuse
    // it for all requests.
    let client = Client::new();
    let res = client.get(url).send().unwrap();

    eprintln!("Response: {:?} {}", res.version(), res.status());
    eprintln!("Headers: {:#?}\n", res.headers());

    let body = res.text().unwrap();

    println!("{}", body);

    // second call
    // Some simple CLI args requirements...
    let url = match std::env::args().nth(1) {
        Some(url) => url,
        None => {
            println!("No CLI URL provided, using default.");
            "http://0.0.0.0:3001/gzip".into()
        }
    };

    eprintln!("Fetching {:?}...", url);

    // nightfly::get() is a convenience function.
    //
    // In most cases, you should create/build a nightfly::Client and reuse
    // it for all requests.
    let res = client.get(url).send().unwrap();

    eprintln!("Response: {:?} {}", res.version(), res.status());
    eprintln!("Headers: {:#?}\n", res.headers());

    let body = res.text().unwrap();

    println!("{}", body);

    // let raw_req =

    // Ok(())
}
