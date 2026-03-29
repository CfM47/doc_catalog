#![allow(dead_code, unused_imports)]

mod document;
mod document_type;
mod metadata;

pub use document::Document;
#[allow(unused_imports)]
pub use document_type::{
    BookMetadata, DocumentType, LectureMetadata, NotesMetadata, PaperMetadata,
};
#[allow(unused_imports)]
pub use metadata::Metadata;
