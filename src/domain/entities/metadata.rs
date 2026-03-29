#[derive(Debug, Clone, PartialEq)]
pub struct Metadata {
    pub document_id: i64,
    pub key: String,
    pub value: String,
}

impl Metadata {
    pub fn new(document_id: i64, key: String, value: String) -> Self {
        Self {
            document_id,
            key,
            value,
        }
    }
}
