use std::rc::Rc;

use differential_dataflow::operators::arrange::TraceAgent;
use differential_dataflow::trace::implementations::spine_fueled::Spine;
use differential_dataflow::trace::implementations::ord::OrdValBatch;

//TODO This should not be here!
pub type Trace<K, V> = TraceAgent<Spine<K, V, usize, isize, Rc<OrdValBatch<K, V, usize, isize>>>>;

#[derive(Debug, Serialize, Deserialize)]
pub enum Requests {
    Purchases(PurchasesRequest),
    Recommendations(RecommendationsRequest),
    ModelState(ModelStateRequest),
    PurchaseDeletion(PurchaseDeletionRequest),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PurchasesRequest {
    pub user_id: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Scenario {
    Alcohol,
    Obesity,
    Carbon
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelStateRequest {
    pub user_id: usize,
    pub scenario: Scenario,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RecommendationsRequest {
    pub user_id: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PurchaseDeletionRequest {
    pub user_id: usize,
    pub item_id: usize,
}