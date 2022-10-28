mod support;

use lunatic::{
    abstract_process,
    process::{ProcessRef, StartProcess},
    spawn_link,
    supervisor::{Supervisor, SupervisorStrategy},
    Process, Tag,
};
use submillisecond::{response::Response as SubmsResponse, router, Application, RequestContext};

struct ServerSup;

struct ServerProcess(Process<()>);

#[abstract_process]
impl ServerProcess {
    #[init]
    fn init(_: ProcessRef<Self>, _: ()) -> Self {
        Self(spawn_link!(|| {
            start_server().unwrap();
        }))
    }

    #[terminate]
    fn terminate(self) {
        println!("Shutdown process");
    }

    #[handle_link_trapped]
    fn handle_link_trapped(&self, _: Tag) {
        println!("Link trapped");
    }
}

impl Supervisor for ServerSup {
    type Arg = String;
    type Children = ServerProcess;

    fn init(config: &mut lunatic::supervisor::SupervisorConfig<Self>, name: Self::Arg) {
        // If a child fails, just restart it.
        config.set_strategy(SupervisorStrategy::OneForOne);
        // Start One `ServerProcess`
        config.children_args(((), Some(name)));
    }
}

fn brotli(req: RequestContext) -> SubmsResponse {
    assert_eq!(req.method(), "HEAD");

    SubmsResponse::builder()
        .header("content-encoding", "br")
        .header("content-length", 100)
        .body(vec![])
        .unwrap()
}

fn accept(req: RequestContext) -> SubmsResponse {
    assert_eq!(req.headers()["accept"], "application/json");
    assert!(req.headers()["accept-encoding"]
        .to_str()
        .unwrap()
        .contains("br"));
    SubmsResponse::default()
}

fn accept_encoding(req: RequestContext) -> SubmsResponse {
    assert_eq!(req.headers()["accept"], "*/*");
    assert_eq!(req.headers()["accept-encoding"], "identity");
    SubmsResponse::default()
}

fn start_server() -> std::io::Result<()> {
    Application::new(router! {
        HEAD "/brotli" => brotli
        GET "/accept" => accept
        GET "/accept-encoding" => accept_encoding
    })
    .serve(ADDR)
}

static ADDR: &'static str = "0.0.0.0:3000";

fn ensure_server() {
    if let Some(_) = Process::<Process<()>>::lookup("__brotli__") {
        return;
    }
    ServerSup::start("__brotli__".to_owned(), None);
}

// ====================================
// Test cases
// ====================================

#[lunatic::test]
fn test_brotli_empty_body() {
    let _ = ensure_server();

    let client = nightfly::Client::new();
    let res = client
        .head(&format!("http://{}/brotli", ADDR))
        .send()
        .unwrap();

    let body = res.text().unwrap();

    assert_eq!(body, "");
}

#[lunatic::test]
fn test_accept_header_is_not_changed_if_set() {
    let _ = ensure_server();

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
    let _ = ensure_server();

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
