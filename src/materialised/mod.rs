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
use std::time::Instant;
use differential_dataflow::trace::{Cursor, TraceReader};
use crate::caboose::sparse_topk_index::SparseTopKIndex;

use crate::web::types::Trace;
use crate::demo::database::PurchaseDatabase;
use crate::tifuknn::dataflow::tifu_model;
use crate::tifuknn::types::{DiscretisedItemVector, HyperParams};

use crate::tifuknn::types::DISCRETISATION_FACTOR;

use sprs::SpIndex;

use crate::materialised::types::{DeletionImpact, Neighborhood, UserEmbedding};

pub struct TifuView {
    database: Rc<RefCell<PurchaseDatabase>>,
    worker: Worker<Thread>,
    current_time: usize,
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

        let (mut user_embeddings_trace, items_by_user_trace) =
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

        let topk_index = crate::caboose::serialize::deserialize_from(num_users, num_items,
                                                                            "__instacart-index.bin");
        eprintln!("Loaded precomputed index...");

        let baskets_input = Rc::new(RefCell::new(baskets_input));
        let basket_items_input = Rc::new(RefCell::new(basket_items_input));

        Self {
            database,
            worker,
            current_time: 1,
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

    pub fn neighborhood(&self, user_id: usize) -> Neighborhood {

        let adjacent = self.neighbors_of(user_id);
        //TODO the index should know this number...
        let incident = self.in_neighbors_of(206210, user_id);

        let all_neighbor_ids = adjacent
            .iter().map(|(id, _)| id.to_string())
            .chain(incident.iter().map(|(id, _)| id.to_string()))
            .collect::<Vec<String>>()
            .join(",");

        let mut top_aisles = Vec::new();

        self.database.borrow().from_query(&format!(r#"
            SELECT    p.aisle_id, COUNT(*) * 1.0 / SUM(COUNT(*)) OVER () AS normalized_count
              FROM    products p
              JOIN    order_products op
                ON    p.product_id = op.product_id
              JOIN    orders o
                ON    o.order_id = op.order_id
             WHERE    o.user_id IN ({})
            GROUP BY  p.aisle_id
            ORDER BY  normalized_count DESC
            LIMIT 10;
            "#, all_neighbor_ids),
            |row| {
                let aisle_id: usize = row.get(0).unwrap();
                let percentage: f32 = row.get(1).unwrap();
                top_aisles.push((aisle_id, percentage))
            });



        Neighborhood { user_id, adjacent, incident, top_aisles }
    }

    fn neighbors_of(&self, user_id: usize) -> Vec<(usize, f32)> {
        self.topk_index.neighbors(user_id)
            .map(|row| (row.row as usize, row.similarity))
            .collect()
    }

    fn in_neighbors_of(&self, num_users: usize, user_id: usize) -> Vec<(usize, f32)> {

        let mut incident_to = Vec::new();

        for other_user_id in 0..num_users {
            for row in self.topk_index.neighbors(other_user_id) {
                if row.row as usize == user_id {
                    incident_to.push((other_user_id, row.similarity));
                }
            }
        }

        incident_to
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

    pub fn forget_purchase(&mut self, user_id: usize, item_id: usize) -> DeletionImpact {

        let database_update_start = Instant::now();
        // TODO would be much nicer to have real CDC
        let mut basket_ids: Vec<usize> = Vec::new();
        self.database.borrow().from_query(&format!(r#"
            SELECT  op.order_id
            FROM    order_products op
            JOIN    orders o ON o.order_id = op.order_id
            WHERE   o.user_id = {user_id} AND op.product_id = {item_id};"#),
            |row| basket_ids.push(row.get(0).unwrap())
        );

        let baskets_list = basket_ids.iter()
            .map(|basket_id| basket_id.to_string())
            .collect::<Vec<_>>()
            .join(", ");

        let deletion_query = format!(r#"
            DELETE FROM order_products
                  WHERE product_id = {item_id}
                    AND order_id IN ({baskets_list});"#
        );

        self.database.borrow().execute(&deletion_query);
        let database_update_duration = database_update_start.elapsed().as_millis();

        let old_embedding = self.user_embeddings.get(&user_id).unwrap().clone();

        let embedding_update_start = Instant::now();
        // Scoping needed for mutable borrows
        {
            let basket_items_input = &mut self.basket_items_input.borrow_mut();
            let baskets_input = &mut self.baskets_input.borrow_mut();

            for basket_id in &basket_ids {
                basket_items_input.remove((*basket_id, item_id));
            }

            self.current_time += 1;
            baskets_input.advance_to(self.current_time);
            basket_items_input.advance_to(self.current_time);
            baskets_input.flush();
            basket_items_input.flush();

            eprintln!("Moving to time {} for purchase deletion", self.current_time);
            self.worker.step_while(||
                self.user_embeddings_probe.less_than(baskets_input.time())
                    || self.user_embeddings_probe.less_than(basket_items_input.time())
                    || self.items_by_user_probe.less_than(baskets_input.time())
                    || self.items_by_user_probe.less_than(basket_items_input.time())
            );
        }
        eprintln!("Done with {}", self.current_time);
        let _ = self.update_user_embeddings();
        let embedding_update_duration = embedding_update_start.elapsed().as_millis();

        let updated_embedding = self.user_embeddings.get(&user_id).unwrap();

        let new_weights: HashMap<usize, f64> = updated_embedding.indices.iter().zip(updated_embedding.data.iter())
            .map(|(item_id, weight)| (*item_id, *weight  as f64 / DISCRETISATION_FACTOR))
            .collect();

        let embedding_difference: Vec<(usize, f64)> = old_embedding.indices.iter().zip(old_embedding.data.iter())
            .filter_map(|(index, weight)| {
                let new_weight = if new_weights.contains_key(index) {
                    *new_weights.get(&index).unwrap()
                } else {
                    0.0_f64
                };
                let weight_diff = new_weight - (*weight as f64 / DISCRETISATION_FACTOR);

                if weight_diff != 0.0 {
                    Some((*index, weight_diff))
                } else {
                    None
                }
            })
            .collect();



        let topk_index_update_start = Instant::now();
        let (count_nochange, count_update, count_recompute) = self.update_topk_index();
        let topk_index_update_duration = topk_index_update_start.elapsed().as_millis();

        DeletionImpact {
            user_id,
            item_id,
            deletion_query,
            basket_ids,
            embedding_difference,
            database_update_duration,
            embedding_update_duration,
            topk_index_update_duration,
            num_inspected_neighbors: count_nochange as usize,
            num_updated_neighbors: (count_update + count_recompute) as usize
        }
    }

    fn update_topk_index(&mut self) -> (i32, i32, i32) {
        // TODO this is ugly...
        let (mut count_nochange, mut count_update, mut count_recompute) = (0, 0, 0);
        let time_to_check = self.current_time - 1;
        // TODO optimise to use internal batches and skip non-relevant ones
        let (mut cursor, storage) = self.items_by_user_trace.cursor();
        while cursor.key_valid(&storage) {
            let user_id = cursor.key(&storage);
            let mut item_ids = Vec::new();
            while cursor.val_valid(&storage) {
                let item_id = cursor.val(&storage);
                cursor.map_times(&storage, |time, diff| {
                    // This codes makes some strong assumptions about the changes we encounter...
                    if *time == time_to_check && *diff == -1 {
                        // The assumption here is that we only see deletions
                        item_ids.push(*item_id);
                    }
                });
                cursor.step_val(&storage);
            }
            if !item_ids.is_empty() {
                (count_nochange, count_update, count_recompute) =
                    self.topk_index.forget_multiple(*user_id, &item_ids);
            }
            cursor.step_key(&storage);
        }

        (count_nochange, count_update, count_recompute)
    }

    // https://github.com/TimelyDataflow/differential-dataflow/issues/104
    fn update_user_embeddings(&mut self) -> usize {
        let time_to_check = self.current_time - 1;
        let mut num_changed_embeddings = 0;
        // TODO optimise to use internal batches and skip non-relevant ones
        let (mut cursor, storage) = self.user_embeddings_trace.cursor();
        while cursor.key_valid(&storage) {
            let user_id = cursor.key(&storage);
            while cursor.val_valid(&storage) {
                let embedding = cursor.val(&storage);
                cursor.map_times(&storage, |time, diff| {
                    // This codes makes some strong assumptions about the changes we encounter...
                    if *time == time_to_check && *diff == 1 {
                        self.user_embeddings.insert(*user_id, embedding.clone());
                        num_changed_embeddings += 1;
                    }
                });
                cursor.step_val(&storage);
            }
            cursor.step_key(&storage);
        }
        num_changed_embeddings
    }
}


//TODO remove duplication
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