use bigneon_db::models::PaymentProviders;
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
}

impl ServiceLocator {
    pub fn new(config: &Config) -> ServiceLocator {
        ServiceLocator {
            stripe_secret_key: config.stripe_secret_key.clone(),
            globee_api_key: config.globee_api_key.clone(),
            globee_base_url: config.globee_base_url.clone(),
            branch_io_base_url: config.branch_io_base_url.clone(),
            branch_io_branch_key: config.branch_io_branch_key.clone(),
        }
    }

    pub fn create_payment_processor(
        &self,
        provider: PaymentProviders,
    ) -> Result<Box<PaymentProcessor>, BigNeonError> {
        match provider {
            PaymentProviders::Stripe => Ok(Box::new(StripePaymentProcessor::new(
                self.stripe_secret_key.clone(),
            ))),
            PaymentProviders::Globee => Ok(Box::new(GlobeePaymentProcessor::new(
                self.globee_api_key.clone(),
                self.globee_base_url.clone(),
            ))),
            // External is not valid for service locator
            PaymentProviders::External => {
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
