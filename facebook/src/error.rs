#[derive(Error, Debug)]
pub enum FacebookError {
    HttpError(reqwest::Error),
    DeserializationError(serde_json::Error),
    #[error(msg_embedded, no_from, non_std)]
    ParseError(String),
}
