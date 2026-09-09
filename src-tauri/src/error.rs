use serde::Serialize;

/// Safe, user-facing error codes. Never carries journal text, stack traces,
/// or file paths — those are logged locally (see `tracing` setup in
/// `main.rs`) via safe codes only, per the brief's "no raw private text in
/// logs" requirement.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SafeErrorCode {
    NotFound,
    InvalidInput,
    DatabaseLocked,
    DatabaseError,
    RuntimeStopped,
    ModelMissing,
    ModelIncompatible,
    LocalOnlyUnverified,
    Cancelled,
    InsufficientDisk,
    MalformedImport,
    ConsentRequired,
    VaultGenerationStale,
    EmbeddingSpaceMismatch,
    Unexpected,
}

#[derive(Debug, Serialize)]
pub struct AnchorError {
    pub code: SafeErrorCode,
    pub message: String,
}

impl AnchorError {
    pub fn new(code: SafeErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for AnchorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.code, self.message)
    }
}

impl std::error::Error for AnchorError {}

impl From<rusqlite::Error> for AnchorError {
    fn from(e: rusqlite::Error) -> Self {
        let code = match &e {
            rusqlite::Error::SqliteFailure(err, _)
                if err.code == rusqlite::ErrorCode::DatabaseBusy
                    || err.code == rusqlite::ErrorCode::DatabaseLocked =>
            {
                SafeErrorCode::DatabaseLocked
            }
            rusqlite::Error::QueryReturnedNoRows => SafeErrorCode::NotFound,
            _ => SafeErrorCode::DatabaseError,
        };
        // Deliberately do not include `e` in the user-facing message: rusqlite
        // error text can echo back bound values in rare cases. Full detail
        // goes to tracing only, tagged with the safe code.
        tracing::warn!(code = ?code, "sqlite error");
        AnchorError::new(code, "A local database error occurred.")
    }
}

impl From<r2d2::Error> for AnchorError {
    fn from(_e: r2d2::Error) -> Self {
        AnchorError::new(SafeErrorCode::DatabaseLocked, "Could not obtain a database connection.")
    }
}

pub type AnchorResult<T> = Result<T, AnchorError>;
