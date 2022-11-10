#![deny(warnings)]
extern crate nightfly;

use lunatic::{
    abstract_process,
    process::{ProcessRef, StartProcess},
    supervisor::{Supervisor, SupervisorStrategy},
    Mailbox, Tag,
};
use nightfly::Client;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct UuidResponse {
    uuid: String,
}

struct HttpDataProvider {
    client: Client,
    client_id: u32,
}

#[abstract_process]
impl HttpDataProvider {
    #[init]
    fn init(_: ProcessRef<Self>, client_id: u32) -> Self {
        Self {
            client: Client::new(),
            client_id,
        }
    }

    #[terminate]
    fn terminate(self) {
        println!("Shutdown process");
    }

    #[handle_link_trapped]
    fn handle_link_trapped(&self, _: Tag) {
        println!("Link trapped");
    }

    // a request that calls an external service to get a UUID
    #[handle_request]
    fn get_uuid(&mut self) -> String {
        let res: UuidResponse = self
            .client
            .get("http://eu.httpbin.org/uuid")
            .header("accept", "application/json")
            .send()
            .unwrap()
            .json()
            .unwrap();
        res.uuid
    }

    // a request that gets the internal client id for logging/debuggin
    #[handle_request]
    fn client_id(&self) -> u32 {
        self.client_id
    }
}

// contains a cursor that points to the active client
struct HttpClientSup;

impl Supervisor for HttpClientSup {
    type Arg = ();
    // create a pool of size 2
    type Children = (HttpDataProvider, HttpDataProvider);

    fn init(config: &mut lunatic::supervisor::SupervisorConfig<Self>, _: Self::Arg) {
        // If a child fails, just restart it.
        config.set_strategy(SupervisorStrategy::OneForOne);
        // Start Two `HttpDataProvider`s
        config.children_args(((0, None), (1, None)));
    }
}

#[derive(Serialize, Deserialize)]
struct HttpClientPool(usize, ProcessRef<HttpClientSup>);

impl HttpClientPool {
    pub fn new() -> Self {
        let sup = HttpClientSup::start_link((), None);
        Self(0, sup)
    }

    pub fn get_client(&mut self) -> ProcessRef<HttpDataProvider> {
        match self.0 {
            0 => {
                self.0 = 1;
                self.1.children().0
            }
            1 => {
                self.0 = 0;
                self.1.children().1
            }
            _ => panic!("This cannot happen"),
        }
    }
}

// This is using the `lunatic` runtime.
//
#[lunatic::main]
fn main(_: Mailbox<()>) -> () {
    // first, start the client pool
    let mut pool = HttpClientPool::new();
    for _ in 1..5 {
        let client = pool.get_client();
        println!(
            "Getting UUID {:?} from client {:?}",
            client.get_uuid(),
            client.client_id()
        );
    }
}
