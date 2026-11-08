use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    Message(String),
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("file system error: {0}")]
    Io(#[from] std::io::Error),
    #[error("archive error: {0}")]
    Archive(String),
    #[error("image error: {0}")]
    Image(#[from] image::ImageError),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("invalid input: {0}")]
    Invalid(String),
    #[error("cancelled")]
    Cancelled,
}

impl AppError {
    pub fn message(value: impl Into<String>) -> Self {
        AppError::Message(value.into())
    }

    pub fn invalid(value: impl Into<String>) -> Self {
        AppError::Invalid(value.into())
    }

    pub fn not_found(value: impl Into<String>) -> Self {
        AppError::NotFound(value.into())
    }

    pub fn cancelled() -> Self {
        AppError::Cancelled
    }
}

impl From<zip::result::ZipError> for AppError {
    fn from(value: zip::result::ZipError) -> Self {
        AppError::Archive(value.to_string())
    }
}

impl From<quick_xml::Error> for AppError {
    fn from(value: quick_xml::Error) -> Self {
        AppError::Message(format!("xml error: {value}"))
    }
}

#[derive(Serialize)]
struct ErrorPayload {
    kind: String,
    message: String,
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let kind = match self {
            AppError::Message(_) => "message",
            AppError::Database(_) => "database",
            AppError::Io(_) => "io",
            AppError::Archive(_) => "archive",
            AppError::Image(_) => "image",
            AppError::NotFound(_) => "not_found",
            AppError::Invalid(_) => "invalid",
            AppError::Cancelled => "cancelled",
        };
        ErrorPayload {
            kind: kind.to_string(),
            message: self.to_string(),
        }
        .serialize(serializer)
    }
}

pub type AppResult<T> = Result<T, AppError>;
