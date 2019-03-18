#[derive(Serialize, Debug)]
pub struct FacebookRequest<'a, T> {
    pub access_token: &'a str,
    #[serde(flatten)]
    pub data: T,
}
