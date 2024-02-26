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

use snapcase::web::messaging::{Requests, ChangeMessage};
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

fn read_local(file: &str) -> Vec<u8> {
    let mut data = Vec::new();

    let mut file = File::open(file).expect("Unable to read file!");
    file.read_to_end(&mut data).expect("Unable to read file!");

    data
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

    fn broadcast_in_order(&self, mut changes: Vec<ChangeMessage>) {
        changes.sort();
        changes.into_iter()
            .for_each(|change| {
                println!("\t{}", change.message.as_text().unwrap());
                self.broadcast(change.message)
            });
    }

    fn broadcast_json(&self, json: serde_json::Value) {
        self.broadcast(Message::text(json.to_string()));
    }

}




impl Handler for Server {

    fn on_message(&mut self, msg: Message) -> Result<()> {

        // We assume we always get valid utf-8
        let message_as_string = &msg.into_text().unwrap();

        let parsed_request: SerdeResult<Requests> =
            serde_json::from_slice(&message_as_string.as_bytes());

        match parsed_request {
            Ok(request) => {

                println!("Received request: {:?}", request);

                match request {
                    Requests::UserFocus(user_focus_request) => {
                        let user_id = user_focus_request.user_id;
                        let purchases = self.database.borrow().purchases(user_id);
                        self.broadcast_json(json!({"response_type": "purchases", "payload": purchases}));

                        let embedding = self.tifu_view.borrow().user_embedding(user_id);
                        self.broadcast_json(json!({"response_type": "embedding", "payload": embedding}));

                        let recommendations = self.tifu_view.borrow().recommendations_for(user_id, 0.9);
                        self.broadcast_json(json!({"response_type": "recommendations", "payload": recommendations}));

                        let neighbors = self.tifu_view.borrow().neighbors_of(user_id);
                        self.broadcast_json(json!({"response_type": "neighbors", "payload": neighbors}));
                    },
                    Requests::PurchaseDeletion(purchase_deletion) => {
                        eprintln!("Purchase deletion");
                        let deletion_impact = self.tifu_view.borrow_mut().forget_purchase(
                            purchase_deletion.user_id, purchase_deletion.item_id);

                        self.broadcast_json(json!({"response_type": "deletion_impact", "payload": deletion_impact}));
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
            //"/style.css" => Ok(Response::new(200, "OK", read_local("html/style.css"))),
            "/products.js" => Ok(Response::new(200, "OK", read_local("html/products.js"))),
            "/aisles.js" => Ok(Response::new(200, "OK", read_local("html/aisles.js"))),
            "/" => Ok(Response::new(200, "OK", read_local("html/index.html"))),
            _ => Ok(Response::new(404, "Not Found", b"404 - Not Found".to_vec())),
        }
    }
}