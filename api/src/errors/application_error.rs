use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum ApplicationErrorType {
    Unprocessable,
    Internal,
    BadRequest,
    ServerConfigError,
}

#[derive(Debug)]
pub struct ApplicationError {
    pub reason: String,
    pub error_type: ApplicationErrorType,
}

impl ApplicationError {
    pub fn new(reason: String) -> ApplicationError {
        ApplicationError {
            reason,
            error_type: ApplicationErrorType::Internal,
        }
    }

    pub fn new_with_type(error_type: ApplicationErrorType, reason: String) -> ApplicationError {
        ApplicationError { reason, error_type }
    }
}

impl fmt::Display for ApplicationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", self.reason)
    }
}
impl Error for ApplicationError {}
