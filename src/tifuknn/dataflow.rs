use crate::tifuknn::types::{Basket, DiscretisedItemVector, HyperParams};
use crate::tifuknn::aggregation::{group_vector, user_vector};

use timely::dataflow::Scope;
use differential_dataflow::lattice::Lattice;
use differential_dataflow::Collection;
use differential_dataflow::operators::Reduce;

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
