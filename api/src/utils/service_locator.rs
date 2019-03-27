use bigneon_db::prelude::*;
use config::Config;
use errors::*;
use payments::globee::GlobeePaymentProcessor;
use payments::stripe::StripePaymentProcessor;
use payments::PaymentProcessor;
use utils::deep_linker::BranchDeepLinker;
use utils::deep_linker::DeepLinker;

pub struct ServiceLocator {
    stripe_secret_key: String,
    globee_api_key: String,
    globee_base_url: String,
    branch_io_base_url: String,
    branch_io_branch_key: String,
    api_keys_encryption_key: String,
}

impl ServiceLocator {
    pub fn new(config: &Config) -> ServiceLocator {
        ServiceLocator {
            stripe_secret_key: config.stripe_secret_key.clone(),
            globee_api_key: config.globee_api_key.clone(),
            globee_base_url: config.globee_base_url.clone(),
            branch_io_base_url: config.branch_io_base_url.clone(),
            branch_io_branch_key: config.branch_io_branch_key.clone(),
            api_keys_encryption_key: config.api_keys_encryption_key.clone(),
        }
    }

    pub fn create_payment_processor(
        &self,
        provider: PaymentProviders,
        organization: &Organization,
    ) -> Result<Box<PaymentProcessor>, BigNeonError> {
        match provider {
            PaymentProviders::Stripe => Ok(Box::new(StripePaymentProcessor::new(
                self.stripe_secret_key.clone(),
            ))),
            PaymentProviders::Globee => {
                let mut org = organization.clone();
                org.decrypt(&self.api_keys_encryption_key)?;

                let api_key = org.globee_api_key.as_ref().unwrap_or(&self.globee_api_key);
                Ok(Box::new(GlobeePaymentProcessor::new(
                    api_key.to_string(),
                    self.globee_base_url.clone(),
                )))
            }
            // External is not valid for service locator
            PaymentProviders::Free | PaymentProviders::External => {
                return Err(ApplicationError::new("Unknown payment provider".into()).into());
            }
        }
    }

    pub fn create_deep_linker(&self) -> Result<Box<DeepLinker>, BigNeonError> {
        Ok(Box::new(BranchDeepLinker::new(
            self.branch_io_base_url.clone(),
            self.branch_io_branch_key.clone(),
        )))
    }

    pub fn is_refund_supported(provider: String) -> bool {
        match provider.to_lowercase().as_str() {
            "stripe" => true,
            "globee" => false,
            "external" => false,
            _ => false,
        }
    }
}
