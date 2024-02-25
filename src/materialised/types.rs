use crate::tifuknn::types::{DiscretisedItemVector, DISCRETISATION_FACTOR};

#[derive(Debug, Serialize, Deserialize)]
pub struct UserEmbedding {
    user_id: usize,
    weights_per_item: Vec<(usize, f64)>
}

impl UserEmbedding {
    pub fn from_discretised_item_vector(vector: &DiscretisedItemVector) -> Self {
        let weights_per_item: Vec<(usize, f64)> = vector.indices.iter().zip(vector.data.iter())
            .map(|(index, value)| (*index, *value as f64 / DISCRETISATION_FACTOR ))
            .collect();

        Self { user_id: vector.id, weights_per_item }
    }
}