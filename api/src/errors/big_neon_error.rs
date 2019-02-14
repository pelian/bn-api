use actix_web::HttpResponse;
use actix_web::ResponseError;
use bigneon_db::utils::errors::*;
use branch_rs::BranchError;
use diesel::result::Error as DieselError;
use errors::AuthError;
use errors::*;
use globee::GlobeeError;
use jwt::errors::Error as JwtError;
use lettre::smtp::error::Error as SmtpError;
use lettre_email::error::Error as EmailBuilderError;
use payments::PaymentProcessorError;
use r2d2;
use reqwest::header::ToStrError as ReqwestToStrError;
use reqwest::Error as ReqwestError;
use serde_json::Error as SerdeError;
use std::error::Error;
use std::fmt;
use tari_client::TariError;
use uuid::ParseError as UuidParseError;

#[derive(Debug)]
pub struct BigNeonError(Box<ConvertToWebError + Send + Sync>);

macro_rules! error_conversion {
    ($e: ty) => {
        impl From<$e> for BigNeonError {
            fn from(e: $e) -> Self {
                BigNeonError(Box::new(e))
            }
        }
    };
}

error_conversion!(ApplicationError);
error_conversion!(AuthError);
error_conversion!(DatabaseError);
error_conversion!(r2d2::Error);
error_conversion!(DieselError);
error_conversion!(EmailBuilderError);
error_conversion!(EnumParseError);
error_conversion!(JwtError);
error_conversion!(PaymentProcessorError);
error_conversion!(ReqwestError);
error_conversion!(ReqwestToStrError);
error_conversion!(SerdeError);
error_conversion!(SmtpError);
error_conversion!(TariError);
error_conversion!(UuidParseError);
error_conversion!(GlobeeError);
error_conversion!(BranchError);

impl fmt::Display for BigNeonError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&*self.0.to_string())
    }
}

impl Error for BigNeonError {
    fn description(&self) -> &str {
        self.0.description()
    }
}

impl ResponseError for BigNeonError {
    fn error_response(&self) -> HttpResponse {
        self.0.to_response()
    }
}

impl BigNeonError {
    pub fn new(inner: Box<ConvertToWebError + Send + Sync>) -> BigNeonError {
        BigNeonError(inner)
    }

    pub fn into_inner(&self) -> &ConvertToWebError {
        self.0.as_ref()
    }
}
