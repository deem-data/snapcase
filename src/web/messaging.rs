//use ws::Message;
//use std::cmp::Ordering;

#[derive(Debug, Serialize, Deserialize)]
pub enum Requests {
    UserFocus(UserFocusRequest),
    PurchaseDeletion(PurchaseDeletionRequest),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserFocusRequest {
    pub user_id: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PurchaseDeletionRequest {
    pub user_id: usize,
    pub item_id: usize,
}

/*
#[derive(Eq, PartialEq)]
pub struct ChangeMessage {
    pub change: isize,
    pub message: Message,
}


impl ChangeMessage {
    pub fn new(change: isize, message: Message) -> Self {
        ChangeMessage { change, message }
    }
}

impl PartialOrd for ChangeMessage {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.change.cmp(&other.change))
    }
}

impl Ord for ChangeMessage {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(&other).unwrap()
    }
}*/