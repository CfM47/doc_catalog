use crate::domain::errors::DomainError;
use crate::domain::utils::is_kebab_case;
use crate::domain::utils::validate_tags;

#[test]
fn test_valid_kebab_case() {
    assert!(is_kebab_case("rust"));
    assert!(is_kebab_case("programming"));
    assert!(is_kebab_case("rust-programming"));
    assert!(is_kebab_case("rust-2024"));
    assert!(is_kebab_case("a"));
    assert!(is_kebab_case("a-b-c"));
}

#[test]
fn test_invalid_kebab_case_uppercase() {
    assert!(!is_kebab_case("Rust"));
    assert!(!is_kebab_case("RUST"));
    assert!(!is_kebab_case("Programming"));
}

#[test]
fn test_invalid_kebab_case_spaces() {
    assert!(!is_kebab_case("rust programming"));
    assert!(!is_kebab_case("rust programming"));
}

#[test]
fn test_invalid_kebab_case_special_chars() {
    assert!(!is_kebab_case("rust_programming"));
    assert!(!is_kebab_case("rust_programming"));
    assert!(!is_kebab_case("rust.programming"));
    assert!(!is_kebab_case("rust!"));
}

#[test]
fn test_invalid_kebab_case_empty() {
    assert!(!is_kebab_case(""));
}

#[test]
fn test_validate_tags_valid() {
    let tags = vec![
        "rust".to_string(),
        "programming".to_string(),
        "rust-2024".to_string(),
    ];
    assert!(validate_tags(&tags).is_ok());
}

#[test]
fn test_validate_tags_invalid() {
    let tags = vec!["Rust".to_string(), "programming".to_string()];
    let result = validate_tags(&tags);
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        DomainError::InvalidTagFormat("Rust".to_string())
    );
}

#[test]
fn test_validate_tags_empty_list() {
    let tags: Vec<String> = vec![];
    assert!(validate_tags(&tags).is_ok());
}
