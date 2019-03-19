use actix_web::{HttpResponse, State};
use auth::user::User as AuthUser;
use auth::TokenResponse;
use bigneon_db::models::{ExternalLogin, User, FACEBOOK_SITE};
use db::Connection;
use errors::*;
use extractors::*;
use facebook::prelude::FacebookClient;
use itertools::Itertools;
use models::FacebookWebLoginToken;
use reqwest;
use serde_json;
use server::AppState;

const FACEBOOK_GRAPH_URL: &str = "https://graph.facebook.com";

#[derive(Deserialize)]
struct FacebookGraphResponse {
    id: String,
    first_name: String,
    last_name: String,
    email: String,
}

// TODO: Not covered by tests
pub fn web_login(
    (state, connection, auth_token): (State<AppState>, Connection, Json<FacebookWebLoginToken>),
) -> Result<HttpResponse, BigNeonError> {
    info!("Finding user");
    let url = format!(
        "{}/me?fields=id,email,first_name,last_name",
        FACEBOOK_GRAPH_URL
    );
    let connection = connection.get();
    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header(
            "Authorization",
            format!("Bearer {}", auth_token.access_token),
        )
        .send()?
        .text()?;

    let facebook_graph_response: FacebookGraphResponse = serde_json::from_str(&response)?;

    let existing_user =
        ExternalLogin::find_user(&facebook_graph_response.id, "facebook.com", connection)?;
    let user = match existing_user {
        Some(u) => {
            info!("Found existing user with id: {}", &u.user_id);
            User::find(u.user_id, connection)?
        }
        None => {
            info!("User not found for external id");

            // Link account if email exists
            match User::find_by_email(&facebook_graph_response.email.clone(), connection) {
                Ok(user) => {
                    info!("User has existing account, linking external service");
                    user.add_external_login(
                        facebook_graph_response.id.clone(),
                        FACEBOOK_SITE.to_string(),
                        auth_token.access_token.clone(),
                        connection,
                    )?;
                    user
                }
                Err(e) => {
                    match e.code {
                        // Not found
                        2000 => {
                            info!("Creating new user");
                            User::create_from_external_login(
                                facebook_graph_response.id.clone(),
                                facebook_graph_response.first_name.clone(),
                                facebook_graph_response.last_name.clone(),
                                facebook_graph_response.email.clone(),
                                FACEBOOK_SITE.to_string(),
                                auth_token.access_token.clone(),
                                connection,
                            )?
                        }
                        _ => return Err(e.into()),
                    }
                }
            }
        }
    };
    info!("Saving access token");
    let response = TokenResponse::create_from_user(
        &state.config.token_secret,
        &state.config.token_issuer,
        &state.config.jwt_expiry_time,
        &user,
    )?;
    Ok(HttpResponse::Ok().json(response))
}

pub fn request_manage_page_access(
    (connection, state, user): (Connection, State<AppState>, AuthUser),
) -> Result<HttpResponse, BigNeonError> {
    // TODO Sign/encrypt the user id passed through so that we can verify it has not been spoofed
    let redirect_url = FacebookClient::get_login_url(
        state.config.facebook_app_id.as_ref().ok_or_else(|| {
            ApplicationError::new_with_type(
                ApplicationErrorType::Unprocessable,
                "Facebook App ID has not been configured".to_string(),
            )
        })?,
        None,
        &user.id().to_string(),
        &["manage-pages"],
    );

    #[derive(Serialize)]
    struct R {
        redirect_url: String,
    };

    let r = R { redirect_url };

    Ok(HttpResponse::Ok().json(r))
}

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct AuthCallbackPathParameters {
    code: Option<String>,
    state: Option<String>,
    error_reason: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

/// Callback for converting an FB code into an access token
pub fn auth_callback((query, state, connection):(Query<AuthCallbackPathParameters>, State<AppState>, Connection)) -> Result<HttpResponse, BigNeonError> {
    info!("Auth callback received");
    if query.error.is_some() {
        return ApplicationError::new_with_type(ApplicationErrorType::, "Facebook login failed".to_string());
    }

    let app_id = state.config.facebook_app_id.as_ref().ok_or_else(|| {
        ApplicationError::new_with_type(
            ApplicationErrorType::ServerConfigError,
            "Facebook App ID has not been configured".to_string(),
        )
    })?;


    let app_secret = state.config.facebook_app_secret.as_ref().ok_or_else(|| {
        ApplicationError::new_with_type(
            ApplicationErrorType::ServerConfigError,
            "Facebook App secret has not been configured".to_string(),
        )
    })?;

    let conn = connection.get();

    let user= match query.state {
        Some(user_id) => {
            // TODO check signature of state to make sure it was sent from us
            User::find(user_id.parse()?, conn)?}
        _ => {
            return ApplicationError::new_with_type(ApplicationErrorType::BadRequest, "State was not provided from Facebook".to_string());
        }
    };

    // Note this must be the same as the redirect url used to in the original call.
    let redirect_url = None;

    FacebookClient::get_access_token(app_id, app_secret, redirect_url, query.code.ok_or_else(|| ApplicationError::new_with_type(ApplicationErrorType::Internal, "Code was not provided from Facebook".to_string()))?);


    user.add_external_login()

}

/// Returns a list of pages that a user has access to manage
pub fn pages((connection, user): (Connection, AuthUser)) -> Result<HttpResponse, BigNeonError> {
    let conn = connection.get();
    let db_user = user.user;

    let client = FacebookClient::from_access_token(
        db_user.find_external_login("facebook", conn)?.access_token,
    );
    let pages = client
        .me
        .accounts
        .list()?
        .into_iter()
        .map(|p| FacebookPage {
            id: p.id,
            name: p.name,
        })
        .collect_vec();
    Ok(HttpResponse::Ok().json(pages))
}

#[derive(Serialize)]
pub struct FacebookPage {
    pub id: String,
    pub name: String,
}

