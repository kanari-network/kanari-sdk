use serde::{Deserialize, Serialize};

/// Move VM Event representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub key: Vec<u8>,
    pub sequence_number: u64,
    pub type_tag: String,
    pub event_data: Vec<u8>,
}
