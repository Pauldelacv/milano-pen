use serde::{Serialize, Serializer};

/// Errors surfaced by the core. They cross the IPC boundary as
/// `{ "kind": "...", "message": "..." }` so the UI can react per kind.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("file error: {0}")]
    Io(#[from] std::io::Error),
    #[error("image error: {0}")]
    Image(#[from] image::ImageError),
    #[error("invalid data: {0}")]
    Json(#[from] serde_json::Error),
    #[error("{0} not found")]
    NotFound(String),
    #[error("{0}")]
    Invalid(String),
    #[error("{0}")]
    Conflict(String),
}

impl Error {
    pub fn kind(&self) -> &'static str {
        match self {
            Error::Db(_) => "database",
            Error::Io(_) => "io",
            Error::Image(_) => "image",
            Error::Json(_) => "json",
            Error::NotFound(_) => "notFound",
            Error::Invalid(_) => "invalid",
            Error::Conflict(_) => "conflict",
        }
    }
}

impl Serialize for Error {
    fn serialize<S: Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut st = s.serialize_struct("Error", 2)?;
        st.serialize_field("kind", self.kind())?;
        st.serialize_field("message", &self.to_string())?;
        st.end()
    }
}

pub type Result<T> = std::result::Result<T, Error>;
