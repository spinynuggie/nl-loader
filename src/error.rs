use thiserror::Error;

#[derive(Debug, Error)]
pub enum LauncherError {
    #[error("API client error: {0}")]
    Reqwest(String),
    #[error("I/O error: {0}")]
    Io(String),
    #[error("JSON parsing error: {0}")]
    SerdeJson(String),
    #[error("MessagePack decoding error: {0}")]
    MessagePack(String),
    #[error("System path / segment error: {0}")]
    System(String),
    #[error("Verification error: {0}")]
    Validation(String),
}

impl From<reqwest::Error> for LauncherError {
    fn from(err: reqwest::Error) -> Self {
        Self::Reqwest(err.to_string())
    }
}

impl From<std::io::Error> for LauncherError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err.to_string())
    }
}

impl From<serde_json::Error> for LauncherError {
    fn from(err: serde_json::Error) -> Self {
        Self::SerdeJson(err.to_string())
    }
}

impl From<rmpv::decode::Error> for LauncherError {
    fn from(err: rmpv::decode::Error) -> Self {
        Self::MessagePack(err.to_string())
    }
}

impl From<base64::DecodeError> for LauncherError {
    fn from(err: base64::DecodeError) -> Self {
        Self::Validation(format!("base64 decode failed: {err}"))
    }
}
