pub mod types;

use std::cell::RefCell;
use std::rc::Rc;
use itertools::Itertools;

use timely::dataflow::ProbeHandle;
use timely::worker::Worker;

use differential_dataflow::input::InputSession;
use timely::communication::allocator::thread::Thread;
use timely::dataflow::operators::Probe;
use timely::dataflow::operators::probe::Handle;

use std::collections::HashMap;
use differential_dataflow::trace::{Cursor, TraceReader};
use crate::caboose::sparse_topk_index::SparseTopKIndex;

use crate::web::types::Trace;
use crate::demo::database::PurchaseDatabase;
use crate::tifuknn::dataflow::tifu_model;
use crate::tifuknn::types::{DiscretisedItemVector, HyperParams};

use crate::tifuknn::types::DISCRETISATION_FACTOR;

use sprs::{SpIndex, TriMat};

use crate::materialised::types::UserEmbedding;

pub struct TifuView {
    database: Rc<RefCell<PurchaseDatabase>>,
    worker: Worker<Thread>,
    baskets_input: Rc<RefCell<InputSession<usize, (usize, usize), isize>>>,
    basket_items_input: Rc<RefCell<InputSession<usize, (usize, usize), isize>>>,
    user_embeddings_probe: ProbeHandle<usize>,
    user_embeddings_trace: Trace<usize, DiscretisedItemVector>,
    items_by_user_probe: ProbeHandle<usize>,
    items_by_user_trace: Trace<usize, usize>,
    user_embeddings: HashMap<usize, DiscretisedItemVector>,
    topk_index: SparseTopKIndex,
}

impl TifuView {
    pub fn new(
        mut worker: Worker<Thread>,
        database: Rc<RefCell<PurchaseDatabase>>,
        params: HyperParams,
    ) -> Self {

        let mut baskets_input = InputSession::new();
        let mut basket_items_input = InputSession::new();

        baskets_input.advance_to(0);
        basket_items_input.advance_to(0);

        let mut user_embeddings_probe = Handle::new();
        let mut items_by_user_probe = Handle::new();

        let (mut user_embeddings_trace, mut items_by_user_trace) =
            worker.dataflow(|scope| {

                let baskets = baskets_input.to_collection(scope);
                let basket_items = basket_items_input.to_collection(scope);

                let (arranged_user_embeddings, arranged_items_by_user) =
                    tifu_model(&baskets, &basket_items, params);

                arranged_user_embeddings.stream.probe_with(&mut user_embeddings_probe);
                arranged_items_by_user.stream.probe_with(&mut items_by_user_probe);

                (arranged_user_embeddings.trace, arranged_items_by_user.trace)
            });

        eprintln!("Inserting purchase data...");
        database.borrow().from_query(
            "SELECT order_id, user_id FROM orders;",
            |row| baskets_input.insert((row.get(0).unwrap(), row.get(1).unwrap()))
        );

        database.borrow().from_query(
            "SELECT order_id, product_id FROM order_products;",
            |row| basket_items_input.insert((row.get(0).unwrap(), row.get(1).unwrap()))
        );

        let mut user_embeddings = HashMap::new();

        baskets_input.advance_to(1);
        basket_items_input.advance_to(1);
        baskets_input.flush();
        basket_items_input.flush();

        eprintln!("Initial execution");
        worker.step_while(||
            user_embeddings_probe.less_than(baskets_input.time())
                || user_embeddings_probe.less_than(basket_items_input.time())
                || items_by_user_probe.less_than(baskets_input.time())
                || items_by_user_probe.less_than(basket_items_input.time())
        );

        let num_changed = update_user_embeddings(1, &mut user_embeddings_trace,
                                                 &mut user_embeddings);
        eprintln!("{:?} embeddings changed when moving to time 1", num_changed);

        let num_users = 206210;
        let num_items = 49689;
        /*
        let mut interactions = TriMat::new((num_users, num_items));

        let (mut cursor, storage) = items_by_user_trace.cursor();
        while cursor.key_valid(&storage) {
            let user = cursor.key(&storage);
            while cursor.val_valid(&storage) {
                let item = cursor.val(&storage);
                interactions.add_triplet(*user, *item, 1.0);
                cursor.step_val(&storage);
            }
            cursor.step_key(&storage);
        }

        let mut topk_index = SparseTopKIndex::new(interactions.to_csr(), 50);
        snapcase::caboose::serialize::serialize_to_file(topk_index, "__instacart-index.bin");*/
        eprintln!("Loading precomputed index...");
        let mut topk_index = crate::caboose::serialize::deserialize_from(num_users, num_items,
                                                                            "__instacart-index.bin");


        let baskets_input = Rc::new(RefCell::new(baskets_input));
        let basket_items_input = Rc::new(RefCell::new(basket_items_input));

        Self {
            database,
            worker,
            baskets_input,
            basket_items_input,
            user_embeddings_probe,
            user_embeddings_trace,
            items_by_user_probe,
            items_by_user_trace,
            user_embeddings,
            topk_index
        }
    }

    pub fn neighbors_of(&self, user_id: usize) -> Vec<(usize, f32)> {
        self.topk_index.neighbors(user_id)
            .map(|row| (row.row as usize, row.similarity))
            .collect()
    }

    pub fn user_embedding(&self, user_id: usize) -> UserEmbedding {
        let raw_vector = self.user_embeddings.get(&user_id).unwrap();
        UserEmbedding::from_discretised_item_vector(raw_vector)
    }

    pub fn recommendations_for(&self, user_id: usize, alpha: f64) -> Vec<(usize, f64)> {

        // TODO make this an attribute
        let mut item_weights = vec![0.0; 49689];
        for similar_user in self.topk_index.neighbors(user_id) {
            let neighbor_id = similar_user.row.index();
            let neighbor_embedding = self.user_embeddings.get(&neighbor_id).unwrap();
            // TODO move to type
            for (index, value) in neighbor_embedding.indices.iter().zip(neighbor_embedding.data.iter()) {
                item_weights[*index] +=
                    (1.0 - alpha)
                        * similar_user.similarity as f64
                        * (*value as f64 / DISCRETISATION_FACTOR);
            }
        }

        let user_embedding = self.user_embeddings.get(&user_id).unwrap();
        // TODO move to type
        for (index, value) in user_embedding.indices.iter().zip(user_embedding.data.iter()) {
            item_weights[*index] +=
                alpha * (*value as f64 / DISCRETISATION_FACTOR);
        }

        let recommended_items: Vec<_> = item_weights.into_iter().enumerate()
            .filter(|(_index, value)| *value > 0.0)
            .sorted_by_key(|(_index, value)| (-1.0 * *value * DISCRETISATION_FACTOR) as isize)
            // TODO make this a parameter
            .take(20)
            .collect();

        recommended_items
    }
}



// https://github.com/TimelyDataflow/differential-dataflow/issues/104
fn update_user_embeddings(
    time_of_interest: usize,
    user_embeddings_trace: &mut Trace<usize, DiscretisedItemVector>,
    user_embeddings: &mut HashMap<usize, DiscretisedItemVector>,
) -> usize {
    let time_to_check = time_of_interest - 1;
    let mut num_changed_embeddings = 0;
    // TODO optimise to use internal batches and skip non-relevant ones
    let (mut cursor, storage) = user_embeddings_trace.cursor();
    while cursor.key_valid(&storage) {
        let user_id = cursor.key(&storage);
        while cursor.val_valid(&storage) {
            let embedding = cursor.val(&storage);
            cursor.map_times(&storage, |time, diff| {
                // This codes makes some strong assumptions about the changes we encounter...
                if *time == time_to_check && *diff == 1 {
                    user_embeddings.insert(*user_id, embedding.clone());
                    num_changed_embeddings += 1;
                }
            });
            cursor.step_val(&storage);
        }
        cursor.step_key(&storage);
    }
    num_changed_embeddings
}