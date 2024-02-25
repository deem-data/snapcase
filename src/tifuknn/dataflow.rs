use crate::tifuknn::types::{Basket, DiscretisedItemVector, HyperParams};
use crate::tifuknn::aggregation::{group_vector, user_vector};

use timely::dataflow::{Scope, ScopeParent};
use differential_dataflow::lattice::Lattice;
use differential_dataflow::Collection;
use differential_dataflow::operators::{Join, Reduce, Threshold};
use differential_dataflow::operators::arrange::{ArrangeByKey, Arranged, TraceAgent};
use differential_dataflow::trace::implementations::ord::OrdValSpine;

pub fn tifu_model<G: Scope>(
    baskets: &Collection<G, (usize, usize), isize>,
    basket_items: &Collection<G, (usize, usize), isize>,
    params: HyperParams,
) -> (Arranged<G, TraceAgent<OrdValSpine<usize, DiscretisedItemVector, <G as ScopeParent>::Timestamp, isize>>>,
      Arranged<G, TraceAgent<OrdValSpine<usize, usize, <G as ScopeParent>::Timestamp, isize>>>)
    where G::Timestamp: Lattice+Ord
{
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

    let user_embeddings = user_vectors(&baskets_with_items, params);

    let arranged_items_by_user = basket_items_by_basket_and_user
        .map(|((user_id, _basket_id), item_id)| (user_id, item_id))
        .distinct()
        .arrange_by_key();

    let arranged_user_embeddings = user_embeddings
        .arrange_by_key();

    (arranged_user_embeddings, arranged_items_by_user)
}

pub fn user_vectors<G: Scope>(
    baskets: &Collection<G, (usize, Basket), isize>,
    params: HyperParams,
) -> Collection<G, (usize, DiscretisedItemVector), isize>
    where G::Timestamp: Lattice+Ord
{
    let group_vectors = baskets
        .reduce(move |_user, baskets_and_multiplicities, out| {
            for (basket, multiplicity) in baskets_and_multiplicities {
                // TODO write a test for this...
                let group = if *multiplicity % params.group_size == 0 {
                    *multiplicity / params.group_size
                } else {
                    (*multiplicity + (params.group_size - (*multiplicity % params.group_size)))
                        / params.group_size
                };

                assert_ne!(group, 0);

                out.push(((group, (*basket).clone()), *multiplicity));
            }
        })
        .map(|(user, (group, basket))| ((user, group), basket))
        .reduce(move |(_user, group), baskets_and_multiplicities, out| {
            let group_vector = group_vector(
                *group as usize,
                baskets_and_multiplicities,
                params.group_size,
                params.r_basket,
            );

            out.push((group_vector, *group));
        })
        .map(|((user, _), group_vector)| (user, group_vector));
    //.inspect(|x| println!("Group vector {:?}", x));

    let user_vectors = group_vectors
        .reduce(move |user, vectors_and_multiplicities, out| {
            let user_vector = user_vector(*user, vectors_and_multiplicities, params.r_group);
            //println!("USER-{}-{}", user, user_vector.print());
            out.push((user_vector, 1))
        });

    user_vectors
}
