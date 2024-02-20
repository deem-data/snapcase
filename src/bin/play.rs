
extern crate timely;
extern crate differential_dataflow;

use std::collections::HashMap;
use differential_dataflow::input::InputSession;
use differential_dataflow::operators::{Join, Reduce, Threshold};
use differential_dataflow::operators::arrange::ArrangeByKey;
use differential_dataflow::trace::{Cursor, TraceReader};

use timely::dataflow::operators::Probe;
use timely::dataflow::operators::probe::Handle;
//use timely::progress::frontier::AntichainRef;

use sprs::TriMat;
use snapcase::caboose::sparse_topk_index::SparseTopKIndex;
use snapcase::demo::from_query;

use snapcase::tifuknn::types::{Basket, DiscretisedItemVector};
use snapcase::tifuknn::hyperparams::PARAMS_INSTACART;
use snapcase::tifuknn::dataflow::user_vectors;
use snapcase::tifuknn::types::Trace;

type UserId = usize;
type BasketId = usize;
type ItemId = usize;

/*
#5,marinades meat preparation
#95,canned meat seafood
#96,lunch meat
#15,packaged seafood
#33,kosher foods
#34,frozen meat seafood
#35,poultry counter
#49,packaged poultry
#106,hot dogs bacon sausage
#122,meat counter

#27,beers coolers
#28,red wines
#62,white wines
#124,spirits
#134,specialty wines champagnes


baby_aisles = [82, 92, 102, 56]
meat_aisles = [5, 15, 33, 34, 35, 49, 95, 96, 106, 122]
alcohol_aisles = [27, 28, 62, 124, 134]
*/

fn main() {

    let _ = timely::execute_from_args(std::env::args(), move |worker| {

        let mut baskets_input = InputSession::new();
        baskets_input.advance_to(0);
        let mut basket_items_input = InputSession::new();
        basket_items_input.advance_to(0);

        let (user_embeddings_probe, mut user_embeddings_trace,
            items_by_user_probe, mut items_by_user_trace) =
            worker.dataflow(|scope| {

            let mut user_embeddings_probe = Handle::new();
            let mut items_by_user_probe = Handle::new();

            let baskets = baskets_input.to_collection(scope);
            let basket_items = basket_items_input.to_collection(scope);

            let basket_items_by_basket_and_user = baskets
                .join_map(&basket_items,  |basket_id, &user_id, &item_id| {
                    ((user_id, *basket_id), item_id)
                });

            let baskets_with_items = basket_items_by_basket_and_user
                .reduce(|(_user_id, basket_id), item_ids_with_multiplicities, out| {
                    let item_ids: Vec<usize> = item_ids_with_multiplicities.iter()
                        .map(|(item_id, _)| **item_id)
                        .collect();
                    let basket = Basket::new(*basket_id, item_ids);

                    out.push((basket, 1))
                })
                .map(|((user_id, _basket_id), basket)| (user_id, basket));

            let user_embeddings = user_vectors(&baskets_with_items, PARAMS_INSTACART);

            let arranged_items_by_user = basket_items_by_basket_and_user
                .map(|((user_id, _basket_id), item_id)| (user_id, item_id))
                .distinct()
                .arrange_by_key();

            arranged_items_by_user.stream.probe_with(&mut items_by_user_probe);

            let arranged_user_embeddings = user_embeddings
                .arrange_by_key();

            arranged_user_embeddings.stream.probe_with(&mut user_embeddings_probe);

            (user_embeddings_probe, arranged_user_embeddings.trace,
             items_by_user_probe, arranged_items_by_user.trace)
        });



        snapcase::demo::from_query(
           "SELECT order_id, user_id FROM 'datasets/instacart/orders.parquet';",
           |row| baskets_input.insert((row.get(0).unwrap(), row.get(1).unwrap()))
        );

        snapcase::demo::from_query(
            "SELECT order_id, product_id FROM 'datasets/instacart/order_products.parquet';",
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

        //let mut topk_index = SparseTopKIndex::new(interactions.to_csr(), 50);
        //snapcase::caboose::serialize::serialize_to_file(topk_index, "__instacart-index.bin");
        let mut topk_index = snapcase::caboose::serialize::deserialize_from(num_users, num_items,
                                                                            "__instacart-index.bin");

        from_query(r#"
                SELECT    op.order_id, op.product_id
                  FROM    'datasets/instacart/products.parquet' p
                  JOIN    'datasets/instacart/order_products.parquet' op
                    ON    p.product_id = op.product_id
                  JOIN    'datasets/instacart/orders.parquet' o
                    ON    o.order_id = op.order_id
                 WHERE    p.aisle_id IN (27, 28, 62, 124, 134)
                   AND    o.user_id = 40058;
            "#,
            |row| basket_items_input.remove((row.get(0).unwrap(), row.get(1).unwrap()))
        );

        eprintln!("Removing alcolhol items user 40058");
        /*
        eprintln!("{:?}", topk_index.neighbors(1));
        eprintln!("Deleting item 196 for user 1");
        basket_items_input.remove((1187899, 196));
        basket_items_input.remove((431534, 196));
        basket_items_input.remove((550135, 196));
        basket_items_input.remove((2295261, 196));
        basket_items_input.remove((473747, 196));
        basket_items_input.remove((2550362, 196));
        basket_items_input.remove((3367565, 196));
        basket_items_input.remove((2539329, 196));
        basket_items_input.remove((2398795, 196));
        basket_items_input.remove((2254736, 196));
        basket_items_input.remove((3108588, 196));*/

        baskets_input.advance_to(2);
        basket_items_input.advance_to(2);
        baskets_input.flush();
        basket_items_input.flush();

        worker.step_while(||
            user_embeddings_probe.less_than(baskets_input.time())
            || user_embeddings_probe.less_than(basket_items_input.time())
            || items_by_user_probe.less_than(baskets_input.time())
            || items_by_user_probe.less_than(basket_items_input.time())
        );

        let num_changed = update_user_embeddings(2, &mut user_embeddings_trace,
                                                  &mut user_embeddings);
        update_topk_index(2, &mut items_by_user_trace, &mut topk_index);

        eprintln!("{:?} embeddings changed when moving to time 2", num_changed);
        eprintln!("{:?}", user_embeddings.get(&1).unwrap());
    });
}

fn update_topk_index(
    time_of_interest: usize,
    items_by_user_trace: &mut Trace<UserId, usize, usize, isize>,
    sparse_topk_index: &mut SparseTopKIndex,
) {
    let time_to_check = time_of_interest - 1;
    // TODO optimise to use internal batches and skip non-relevant ones
    let (mut cursor, storage) = items_by_user_trace.cursor();
    while cursor.key_valid(&storage) {
        let user_id = cursor.key(&storage);
        while cursor.val_valid(&storage) {
            let item_id = cursor.val(&storage);
            cursor.map_times(&storage, |time, diff| {
                if *user_id == 1 {
                    eprintln!("item: {:?}, time:  {:?} diff: {:?}", item_id, time, diff);
                }
                // This codes makes some strong assumptions about the changes we encounter...
                if *time == time_to_check && *diff == -1 {
                    // The assumption here is that we only see deletions
                    sparse_topk_index.forget(*user_id, *item_id);
                }
            });
            cursor.step_val(&storage);
        }
        cursor.step_key(&storage);
    }
}

// https://github.com/TimelyDataflow/differential-dataflow/issues/104
fn update_user_embeddings(
    time_of_interest: usize,
    user_embeddings_trace: &mut Trace<UserId, DiscretisedItemVector, usize, isize>,
    user_embeddings: &mut HashMap<UserId, DiscretisedItemVector>,
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
