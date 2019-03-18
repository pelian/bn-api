#[derive(Error, Debug)]
pub enum FacebookError {
    HttpError(reqwest::Error),
    DeserializationError(serde_json::Error),
}
