#[derive(Deserialize)]
pub struct Account {
    pub category: String,
    pub category_list: Vec<CategoryListItem>,
    pub name: String,
    pub access_token: String,
    pub id: String,
    pub tasks: Vec<String>,
}

#[derive(Deserialize)]
pub struct CategoryListItem {
    pub id: String,
    pub name: String,
}
