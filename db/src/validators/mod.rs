mod event_ids_belong_to_organization;
mod n_date_before_m_date_validator;
mod number_validators;
mod redemption_code_uniqueness_validator;
mod start_date_before_end_date_validator;
mod url_array_validator;

pub use self::event_ids_belong_to_organization::event_ids_belong_to_organization_validation;
pub use self::n_date_before_m_date_validator::n_date_valid;
pub use self::number_validators::validate_greater_than;
pub use self::redemption_code_uniqueness_validator::redemption_code_unique_per_event_validation;
pub use self::start_date_before_end_date_validator::start_date_valid;
pub use self::url_array_validator::validate_urls;

use std::borrow::Cow;
use validator::*;

pub fn append_validation_error(
    validation_errors: Result<(), ValidationErrors>,
    field: &'static str,
    validation_error: Result<(), ValidationError>,
) -> Result<(), ValidationErrors> {
    if let Err(validation_error) = validation_error {
        let mut validation_errors = match validation_errors {
            Ok(_) => ValidationErrors::new(),
            Err(mut validation_errors) => validation_errors,
        };
        validation_errors.add(field, validation_error);
        Err(validation_errors)
    } else {
        validation_errors
    }
}

pub fn create_validation_error(code: &'static str, message: &'static str) -> ValidationError {
    let mut validation_error = ValidationError::new(code);
    validation_error.message = Some(Cow::from(message));
    validation_error
}
