use crate::tifuknn::types::{DiscretisedItemVector, DISCRETISATION_FACTOR};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum Change {
    Insert,
    Update,
    Delete
}

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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EgoNetwork {
    pub(crate) vertices: Vec<usize>,
    pub(crate) edges: Vec<(usize, usize)>,
    pub(crate) vertices_with_sensitive_items: Vec<usize>,
    pub(crate) top_aisles: Vec<(usize, f64)>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Neighborhood {
    pub(crate) user_id: usize,
    pub(crate) adjacent: Vec<(usize, f64)>,
    pub(crate) incident: Vec<(usize, f64)>,
    pub(crate) top_aisles: Vec<(usize, f64)>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeletionImpact {
    pub(crate) user_id: usize,
    pub(crate) item_id: usize,
    pub(crate) deletion_query: String,
    pub(crate) basket_ids: Vec<usize>,
    pub(crate) embedding_difference: Vec<(usize, f64)>,
    pub(crate) recommendation_difference: Vec<(usize, f64, Change)>,
    pub(crate) adjacent_difference: Vec<(usize, f64, Change)>,
    pub(crate) incident_difference: Vec<(usize, f64, Change)>,
    pub(crate) top_aisle_difference: Vec<(usize, f64, Change)>,
    pub(crate) database_update_duration: u128,
    pub(crate) embedding_update_duration: u128,
    pub(crate) topk_index_update_duration: u128,
    pub(crate) num_inspected_neighbors: usize,
    pub(crate) num_updated_neighbors: usize,
}
