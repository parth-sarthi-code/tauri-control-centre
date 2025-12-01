//! Error types for Control Centre
//! 
//! Provides structured error handling for all system command operations.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Error types for Control Centre operations
#[derive(Error, Debug, Serialize, Deserialize)]
pub enum ControlCentreError {
    #[error("Command execution failed: {0}")]
    CommandFailed(String),
    
    #[error("Command not found: {0}")]
    CommandNotFound(String),
    
    #[error("Parse error: {0}")]
    ParseError(String),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    
    #[error("System error: {0}")]
    SystemError(String),
    
    #[error("Timeout: {0}")]
    Timeout(String),
}

impl From<std::io::Error> for ControlCentreError {
    fn from(err: std::io::Error) -> Self {
        match err.kind() {
            std::io::ErrorKind::NotFound => {
                ControlCentreError::CommandNotFound(err.to_string())
            }
            std::io::ErrorKind::PermissionDenied => {
                ControlCentreError::PermissionDenied(err.to_string())
            }
            std::io::ErrorKind::TimedOut => {
                ControlCentreError::Timeout(err.to_string())
            }
            _ => ControlCentreError::SystemError(err.to_string()),
        }
    }
}

impl From<std::num::ParseIntError> for ControlCentreError {
    fn from(err: std::num::ParseIntError) -> Self {
        ControlCentreError::ParseError(err.to_string())
    }
}

/// Result type alias for Control Centre operations
pub type CCResult<T> = Result<T, ControlCentreError>;

/// Converts ControlCentreError to a string for Tauri command responses
impl From<ControlCentreError> for String {
    fn from(err: ControlCentreError) -> Self {
        err.to_string()
    }
}
