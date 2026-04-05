pub mod dto;
pub mod repositories;
pub mod use_cases;
pub mod utils;

#[cfg(test)]
pub mod tests {
    mod create_document;
    mod delete_document;
    mod document_repository;
    mod list_documents;
    mod search_documents;
    mod update_document;
    pub mod utils;
}
