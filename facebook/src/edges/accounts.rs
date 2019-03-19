#[derive(Deserialize)]
pub struct Account {
    pub category: String,
    pub name: String,
    pub access_token: String,
    pub id: String,
    pub tasks: Vec<String>,
}
