#![allow(dead_code)]

use crate::domain::entities::Document;
use anyhow::Result;

pub trait DocumentRepository: Send + Sync {
    fn create(&self, document: Document) -> Result<Document>;
    fn find_by_id(&self, id: i64) -> Result<Document>;
    fn find_all(&self) -> Result<Vec<Document>>;
    fn update(&self, document: Document) -> Result<Document>;
    fn delete(&self, id: i64) -> Result<()>;
}
