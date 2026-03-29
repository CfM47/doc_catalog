#[derive(Debug, Clone, PartialEq)]
pub enum DocumentType {
    Book(BookMetadata),
    Paper(PaperMetadata),
    Lecture(LectureMetadata),
    Notes(NotesMetadata),
}

impl DocumentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            DocumentType::Book(_) => "book",
            DocumentType::Paper(_) => "paper",
            DocumentType::Lecture(_) => "lecture",
            DocumentType::Notes(_) => "notes",
        }
    }

    pub fn metadata_keys(&self) -> &'static [&'static str] {
        match self {
            DocumentType::Book(_) => BookMetadata::keys(),
            DocumentType::Paper(_) => PaperMetadata::keys(),
            DocumentType::Lecture(_) => LectureMetadata::keys(),
            DocumentType::Notes(_) => NotesMetadata::keys(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct BookMetadata {
    pub authors: Option<Vec<String>>,
    pub edition: Option<String>,
    pub publisher: Option<String>,
    pub isbn: Option<String>,
}

impl BookMetadata {
    pub fn keys() -> &'static [&'static str] {
        &["authors", "edition", "publisher", "isbn"]
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PaperMetadata {
    pub authors: Option<Vec<String>>,
    pub journal: Option<String>,
    pub volume: Option<String>,
    pub issue: Option<String>,
    pub doi: Option<String>,
}

impl PaperMetadata {
    pub fn keys() -> &'static [&'static str] {
        &["authors", "journal", "volume", "issue", "doi"]
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct LectureMetadata {
    pub event: Option<String>,
    pub institution: Option<String>,
    pub location: Option<String>,
    pub topic: Option<String>,
}

impl LectureMetadata {
    pub fn keys() -> &'static [&'static str] {
        &["event", "institution", "location", "topic"]
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct NotesMetadata {}

impl NotesMetadata {
    pub fn keys() -> &'static [&'static str] {
        &[]
    }
}
