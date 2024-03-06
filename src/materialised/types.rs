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

#[derive(Debug, Serialize, Deserialize)]
pub struct Neighborhood {
    pub(crate) user_id: usize,
    pub(crate) adjacent: Vec<(usize, f32)>,
    pub(crate) incident: Vec<(usize, f32)>,
    pub(crate) top_aisles: Vec<(usize, f32)>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeletionImpact {
    pub(crate) user_id: usize,
    pub(crate) item_id: usize,
    pub(crate) deletion_query: String,
    pub(crate) basket_ids: Vec<usize>,
    pub(crate) embedding_difference: Vec<(usize, f64)>,
    pub(crate) database_update_duration: u128,
    pub(crate) embedding_update_duration: u128,
    pub(crate) topk_index_update_duration: u128,
    pub(crate) num_inspected_neighbors: usize,
    pub(crate) num_updated_neighbors: usize,
}
