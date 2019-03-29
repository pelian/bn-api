use actix_web::{HttpResponse, State};
use auth::TokenResponse;
use bigneon_db::models::{ExternalLogin, User, FACEBOOK_SITE};
use db::Connection;
use errors::*;
use extractors::*;
use log::Level::Debug;
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

    jlog!(Debug, "Facebook Login Response", { "response": &response });

    let facebook_graph_response: FacebookGraphResponse = serde_json::from_str(&response)?;

    let existing_user =
        ExternalLogin::find_user(&facebook_graph_response.id, "facebook.com", connection)?;
    let user = match existing_user {
        Some(u) => {
            jlog!(Debug, "Found existing user with id", {
                "user_id": &u.user_id,
                "facebook_id": &facebook_graph_response.id
            });
            User::find(u.user_id, connection)?
        }
        None => {
            jlog!(Debug, "User not found for external id");

            // Link account if email exists
            match User::find_by_email(&facebook_graph_response.email.clone(), connection) {
                Ok(user) => {
                    jlog!(Debug, "User has existing account, linking external service");
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
                            jlog!(Debug, "Creating new user");
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
    jlog!(Debug, "Saving access token");
    let response = TokenResponse::create_from_user(
        &state.config.token_secret,
        &state.config.token_issuer,
        &state.config.jwt_expiry_time,
        &user,
    )?;
    Ok(HttpResponse::Ok().json(response))
}
