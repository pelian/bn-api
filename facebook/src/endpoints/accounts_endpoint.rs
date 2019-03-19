use edges::Account;
use error::FacebookError;
use facebook_client::FacebookClientInner;
use std::rc::Rc;

pub struct AccountsEndpoint {
    pub client: Rc<FacebookClientInner>,
}

impl AccountsEndpoint {
    pub fn list(&self) -> Result<Vec<Account>, FacebookError> {
        let client = reqwest::Client::new();

        // println!("{}", json!(&request));

        //jlog!(Info, "Sending request to Facebook", { "request": &request });

        let mut resp = client
            .get(&format!("{}/me/accounts", &self.client.base_url))
            .header(
                "Authorization",
                format!("Bearer {}", &self.client.app_access_token),
            )
            .send()?;
        let status = resp.status();
        let value: serde_json::Value = resp.json()?;
        println!("{:?}", value);

        let results: Vec<Account> = serde_json::from_value(value)?;
        Ok(results)
    }
}
