use diesel::result::Error as DieselError;
use std::fmt::Display;
use thiserror::Error;

/// Error type for database pool initialization
#[derive(Debug, Error)]
pub enum ErrorKind {
    #[error("cannot init database pool : {0}")]
    Pool(String),
    #[error("cannot find environment variable for database init : {0}")]
    Variable(String),
    #[error("database init error : {0}")]
    GenericInit(String),
}

/// Unified database error type with context for runtime operations
#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("Failed to get connection from pool for operation '{operation}': {message}")]
    PoolError { operation: String, message: String },

    #[error("Database interaction failed for operation '{operation}': {message}")]
    InteractionError { operation: String, message: String },

    #[error("Record not found in operation '{operation}'")]
    NotFound { operation: String },

    #[error("Database query error in operation '{operation}': {message}")]
    QueryError { operation: String, message: String },

    #[error("Unique constraint violation in operation '{operation}': {message}")]
    UniqueViolation { operation: String, message: String },

    #[error("Foreign key constraint violation in operation '{operation}': {message}")]
    ForeignKeyViolation { operation: String, message: String },
}

impl DatabaseError {
    /// Create a `NotFound` error with operation context
    pub fn not_found(operation: impl Display) -> Self {
        Self::NotFound {
            operation: operation.to_string(),
        }
    }

    /// Create a `QueryError` with operation context
    pub fn query_error(operation: impl Display, message: impl Display) -> Self {
        Self::QueryError {
            operation: operation.to_string(),
            message: message.to_string(),
        }
    }

    /// Check if this error is a `NotFound` variant
    pub const fn is_not_found(&self) -> bool {
        matches!(self, Self::NotFound { .. })
    }

    /// Extract the operation context from the error
    pub fn operation(&self) -> &str {
        match self {
            Self::PoolError { operation, .. }
            | Self::InteractionError { operation, .. }
            | Self::NotFound { operation }
            | Self::QueryError { operation, .. }
            | Self::UniqueViolation { operation, .. }
            | Self::ForeignKeyViolation { operation, .. } => operation,
        }
    }
}

impl From<DieselError> for DatabaseError {
    fn from(err: DieselError) -> Self {
        match err {
            DieselError::NotFound => Self::NotFound {
                operation: "unknown".to_string(),
            },
            DieselError::DatabaseError(
                diesel::result::DatabaseErrorKind::UniqueViolation,
                info,
            ) => Self::UniqueViolation {
                operation: "unknown".to_string(),
                message: info.message().to_string(),
            },
            DieselError::DatabaseError(
                diesel::result::DatabaseErrorKind::ForeignKeyViolation,
                info,
            ) => Self::ForeignKeyViolation {
                operation: "unknown".to_string(),
                message: info.message().to_string(),
            },
            other => Self::QueryError {
                operation: "unknown".to_string(),
                message: other.to_string(),
            },
        }
    }
}
