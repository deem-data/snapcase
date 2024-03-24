extern crate timely;
extern crate ws;
extern crate serde_derive;
extern crate serde;
extern crate serde_json;

extern crate snapcase;

use std::cell::RefCell;
use std::rc::Rc;
use std::fs::File;
use std::io::Read;

use serde_json::json;
use ws::listen;

use timely::worker::Worker;
use timely::worker::Config;

use timely::communication::allocator::thread::Thread;

use snapcase::web::types::Requests;
use snapcase::demo::database::PurchaseDatabase;
use snapcase::tifuknn::hyperparams::PARAMS_INSTACART;

use serde_json::Result as SerdeResult;
use ws::{Handler, Message, Request, Response, Result, Sender};
use snapcase::materialised::TifuView;

fn main() {
    let mut worker = Worker::new(Config::default(), Thread::new());
    demo(worker.clone());
    while worker.step_or_park(None) { }
}

fn read_local(file: &str) -> std::io::Result<Vec<u8>> {
    let mut data = Vec::new();
    let mut file = File::open(file)?;
    file.read_to_end(&mut data)?;
    Ok(data)
}

fn demo(worker: Worker<Thread>) {

    let database = Rc::new(RefCell::new(PurchaseDatabase::new()));
    let tifu_view = Rc::new(RefCell::new(TifuView::new(worker, database.clone(), PARAMS_INSTACART)));

    listen("127.0.0.1:8080", |out| {
        Server {
            out,
            database: database.clone(),
            tifu_view: tifu_view.clone(),
        }
    }).unwrap();
}

pub struct Server {
    out: Sender,
    database: Rc<RefCell<PurchaseDatabase>>,
    tifu_view: Rc<RefCell<TifuView>>,
}


impl Server {

    fn broadcast(&self, message: Message) {
        self.out.broadcast(message).expect("Unable to send message");
    }

    fn broadcast_json(&self, json: serde_json::Value) {
        self.broadcast(Message::text(json.to_string()));
    }
}


impl Handler for Server {

    fn on_message(&mut self, msg: Message) -> Result<()> {

        let message_as_string = &msg.into_text().unwrap();

        let parsed_request: SerdeResult<Requests> =
            serde_json::from_slice(&message_as_string.as_bytes());

        match parsed_request {
            Ok(request) => {
                eprintln!("Received request: {:?}", request);

                match request {
                    Requests::Purchases(purchase_request) => {
                        let purchases = self.database.borrow().purchases(purchase_request.user_id);
                        self.broadcast_json(json!({"response_type": "purchases",
                            "payload": purchases}));
                    },

                    Requests::ModelState(model_state_request) => {

                        let user_id = model_state_request.user_id;
                        let tifu_view = self.tifu_view.borrow();

                        let embedding = tifu_view.user_embedding(user_id);
                        let ego_network = tifu_view.ego_network(
                            206210,
                            user_id,
                            model_state_request.scenario
                        );

                        self.broadcast_json(json!({"response_type": "model_state",
                            "payload": {
                                "embedding": embedding,
                                "ego_network": ego_network,
                            }}));
                    },

                    Requests::Recommendations(recommendations_request) => {
                        // TODO should alpha be a parameter?
                        let recommendations = self.tifu_view.borrow()
                            .recommendations_for(recommendations_request.user_id, 0.1);
                        self.broadcast_json(json!({"response_type": "recommendations",
                            "payload": recommendations}));
                    },

                    Requests::PurchaseDeletion(purchase_deletion) => {
                        eprintln!("Purchase deletion");
                        let deletion_impact = self.tifu_view.borrow_mut().forget_purchase(
                            purchase_deletion.user_id,
                            purchase_deletion.item_id,
                        );

                        self.broadcast_json(json!({"response_type": "deletion_impact",
                            "payload": deletion_impact}));
                    }
                }
            },
            Err(e) => println!("Error parsing request:\n{:?}\n\n{:?}\n", &message_as_string, e),
        }

        Ok(())
    }

    fn on_request(&mut self, req: &Request) -> Result<Response> {
        match req.resource() {
            "/ws" => Response::from_request(req),
            "/" => Ok(Response::new(200, "OK", read_local("html/snapcase.html").unwrap())),
            path if path.ends_with(".html") || path.ends_with(".png") || path.ends_with(".css") || path.ends_with(".js") => {
                // TODO we should return the correct content type too here...
                serve_or_404(&format!("html{}", path))
            },
            _ => Ok(Response::new(404, "Not Found", b"404 - Not Found".to_vec())),
        }
    }
}

fn serve_or_404(path: &str) -> ws::Result<Response> {
    match read_local(path) {
        Ok(contents) => Ok(Response::new(200, "OK", contents)),
        _ => Ok(Response::new(404, "Not Found", b"404 - Not Found".to_vec())),
    }
}