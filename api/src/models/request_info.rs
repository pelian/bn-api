#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct RequestInfo {
    pub user_agent: Option<String>,
}
