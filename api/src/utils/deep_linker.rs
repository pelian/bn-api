use branch_rs::prelude::*;
use errors::*;

pub trait DeepLinker {
    fn create_deep_link(&self, raw_link: &str) -> Result<String, BigNeonError>;
}

pub struct BranchDeepLinker {
    client: BranchClient,
}
impl BranchDeepLinker {
    pub fn new(url: String, branch_key: String) -> BranchDeepLinker {
        BranchDeepLinker {
            client: BranchClient::new(url, branch_key),
        }
    }
}

impl DeepLinker for BranchDeepLinker {
    fn create_deep_link(&self, raw_link: &str) -> Result<String, BigNeonError> {
        Ok(self.client.links.create(DeepLink {
            data: DeepLinkData {
                desktop_url: Some(raw_link.to_string()),
                ..Default::default()
            },
            ..Default::default()
        })?)
    }
}
