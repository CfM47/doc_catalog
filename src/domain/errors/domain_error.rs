#![allow(dead_code)]

#[derive(Debug, PartialEq)]
pub enum DomainError {
    EmptyTitle,
    InvalidYear(i32),
    InvalidDocumentType(String),
    TooManyTags(usize),
    TagTooLong(String),
    InvalidTagFormat(String),
}

impl std::fmt::Display for DomainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DomainError::EmptyTitle => write!(f, "Title cannot be empty"),
            DomainError::InvalidYear(year) => {
                write!(f, "Invalid year: {}. Must be between 1000 and 2100", year)
            }
            DomainError::InvalidDocumentType(dt) => {
                write!(f, "Invalid document type: {}", dt)
            }
            DomainError::TooManyTags(count) => {
                write!(f, "Too many tags: {}. Maximum is 50", count)
            }
            DomainError::TagTooLong(tag) => {
                write!(f, "Tag too long: '{}'. Maximum is 30 characters", tag)
            }
            DomainError::InvalidTagFormat(tag) => {
                write!(
                    f,
                    "Invalid tag format: '{}'. Must be kebab-case (lowercase, numbers, hyphens only)",
                    tag
                )
            }
        }
    }
}

impl std::error::Error for DomainError {}
