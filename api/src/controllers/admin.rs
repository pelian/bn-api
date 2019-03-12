use actix_web::HttpResponse;
use auth::user::User as AuthUser;
use bigneon_db::models::{DomainAction, Report, Scopes};
use db::Connection;
use errors::*;

pub fn admin_ticket_count(
    (connection, user): (Connection, AuthUser),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    //Check if they have org admin permissions
    user.requires_scope(Scopes::OrgAdmin)?;
    let result = Report::ticket_sales_and_counts(
        None, None, None, None, false, false, false, false, connection,
    )?;
    Ok(HttpResponse::Ok().json(result))
}

pub fn admin_stuck_domain_actions(
    (connection, user): (Connection, AuthUser),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    //Check if they have org admin permissions
    user.requires_scope(Scopes::OrgAdmin)?;
    let result = DomainAction::find_stuck(connection)?;
    Ok(HttpResponse::Ok().json(result))
}
