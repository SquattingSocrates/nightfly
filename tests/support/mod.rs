pub mod server;

// TODO: remove once done converting to new support server?
#[allow(unused)]
pub static DEFAULT_USER_AGENT: &str =
    concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

// fn ensure_server(server: Fn(req: RequestContext) -> submillisecond::response::Response) {
//     if let Some(_) = Process::<Process<()>>::lookup("__server__") {
//         return;
//     }
//     ServerSup::start("__server__".to_owned(), None);
// }
