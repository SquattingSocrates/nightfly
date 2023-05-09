use lunatic::Tag;
use serde::{Deserialize, Serialize};

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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DummyError;
pub struct DummyProcess;

#[lunatic::abstract_process]
impl DummyProcess {
    #[init]
    fn init(_: lunatic::ap::Config<Self>, _: ()) -> Result<Self, DummyError> {
        Ok(Self)
    }

    #[terminate]
    fn terminate(self) {
        println!("Shutdown process");
    }

    #[handle_link_death]
    fn handle_link_trapped(&self, _: Tag) {
        println!("Link trapped");
    }
}

#[macro_export]
macro_rules! wrap_server {
    ($name:ident, $router:ident, $addr:ident) => {
        mod $name {

            use lunatic::{
                abstract_process, spawn_link,
                supervisor::{Supervisor, SupervisorStrategy},
                AbstractProcess, Process, Tag,
            };
            use submillisecond::Application;

            struct ServerProcess(Process<()>);
            struct ServerSup;

            #[abstract_process]
            impl ServerProcess {
                #[init]
                fn init(
                    _: lunatic::ap::Config<Self>,
                    _: (),
                ) -> Result<Self, crate::support::DummyError> {
                    Ok(Self(spawn_link!(|| {
                        Application::new(super::$router)
                            .serve(super::$addr)
                            .unwrap();
                    })))
                }

                #[terminate]
                fn terminate(self) {
                    println!("Shutdown process");
                }

                #[handle_link_death]
                fn handle_link_trapped(&self, _: Tag) {
                    println!("Link trapped");
                }
            }

            impl Supervisor for ServerSup {
                type Arg = String;
                type Children = (ServerProcess, crate::support::DummyProcess);

                fn init(config: &mut lunatic::supervisor::SupervisorConfig<Self>, name: Self::Arg) {
                    // If a child fails, just restart it.
                    config.set_strategy(SupervisorStrategy::OneForOne);
                    // Start One `ServerProcess`
                    config.children_args((((), None), ((), None)));
                }
            }

            pub fn ensure_server() {
                let name = format!("__{}__", stringify!($name));
                if let Some(_) = Process::<Process<()>>::lookup(&name) {
                    return;
                }
                ServerSup::start(name.to_owned()).expect("should have started server");
            }
        }
    };
}

pub type RouterFn =
    fn() -> fn(req: ::submillisecond::RequestContext) -> ::submillisecond::response::Response;
