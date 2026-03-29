use crate::domain::errors::DomainError;

pub fn is_kebab_case(tag: &str) -> bool {
    if tag.is_empty() {
        return false;
    }
    tag.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

pub fn validate_tags(tags: &[String]) -> Result<(), DomainError> {
    for tag in tags {
        if !is_kebab_case(tag) {
            return Err(DomainError::InvalidTagFormat(tag.clone()));
        }
    }
    Ok(())
}
