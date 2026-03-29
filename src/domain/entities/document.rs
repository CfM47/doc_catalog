use super::{BookMetadata, DocumentType, NotesMetadata};
use crate::domain::errors::DomainError;

#[allow(dead_code)]
const MIN_YEAR: i32 = 1000;
#[allow(dead_code)]
const MAX_YEAR: i32 = 2100;
#[allow(dead_code)]
const MAX_TAGS: usize = 50;
#[allow(dead_code)]
const MAX_TAG_LENGTH: usize = 30;

#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    pub id: i64,
    pub title: String,
    pub doc_type: DocumentType,
    pub year: Option<i32>,
    pub source: Option<String>,
    pub url: Option<String>,
    pub tags: Vec<String>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl Document {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: i64,
        title: String,
        doc_type: DocumentType,
        year: Option<i32>,
        source: Option<String>,
        url: Option<String>,
        tags: Vec<String>,
        notes: Option<String>,
        created_at: String,
        updated_at: String,
    ) -> Result<Self, DomainError> {
        let doc = Self {
            id,
            title,
            doc_type,
            year,
            source,
            url,
            tags,
            notes,
            created_at,
            updated_at,
        };
        doc.validate()?;
        Ok(doc)
    }

    pub fn validate(&self) -> Result<(), DomainError> {
        if self.title.trim().is_empty() {
            return Err(DomainError::EmptyTitle);
        }

        if let Some(year) = self.year
            && !(MIN_YEAR..=MAX_YEAR).contains(&year)
        {
            return Err(DomainError::InvalidYear(year));
        }

        if self.tags.len() > MAX_TAGS {
            return Err(DomainError::TooManyTags(self.tags.len()));
        }

        for tag in &self.tags {
            if tag.len() > MAX_TAG_LENGTH {
                return Err(DomainError::TagTooLong(tag.clone()));
            }
        }

        Ok(())
    }

    pub fn validate_metadata_key(&self, key: &str) -> bool {
        self.doc_type.metadata_keys().contains(&key)
    }
}

impl Default for Document {
    fn default() -> Self {
        Self {
            id: 0,
            title: String::new(),
            doc_type: DocumentType::Notes(NotesMetadata {}),
            year: None,
            source: None,
            url: None,
            tags: Vec::new(),
            notes: None,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }
}
