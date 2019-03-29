use actix_web::{HttpResponse, Path};
use auth::user::User;
use bigneon_db::models::*;
use chrono::prelude::*;
use db::Connection;
use errors::{ApplicationError, BigNeonError};
use extractors::*;
use helpers::application;
use models::PathParameters;
use serde_with::rust::double_option;
use uuid::Uuid;

#[derive(Deserialize, Serialize)]
pub struct CreateCodeRequest {
    pub name: String,
    pub redemption_codes: Vec<String>,
    pub code_type: CodeTypes,
    pub max_uses: u32,
    pub discount_in_cents: Option<u32>,
    pub discount_as_percentage: Option<u32>,
    pub start_date: NaiveDateTime,
    pub end_date: NaiveDateTime,
    pub max_tickets_per_user: Option<u32>,
    pub ticket_type_ids: Vec<Uuid>,
}

#[derive(Clone, Default, Deserialize, Serialize)]
pub struct UpdateCodeRequest {
    pub name: Option<String>,
    pub redemption_code: Option<String>,
    pub max_uses: Option<i64>,
    #[serde(default, deserialize_with = "double_option::deserialize")]
    pub discount_in_cents: Option<Option<u32>>,
    #[serde(default, deserialize_with = "double_option::deserialize")]
    pub discount_as_percentage: Option<Option<u32>>,
    pub start_date: Option<NaiveDateTime>,
    pub end_date: Option<NaiveDateTime>,
    #[serde(default, deserialize_with = "double_option::deserialize")]
    pub max_tickets_per_user: Option<Option<u32>>,
    pub ticket_type_ids: Option<Vec<Uuid>>,
}

impl From<UpdateCodeRequest> for UpdateCodeAttributes {
    fn from(attributes: UpdateCodeRequest) -> Self {
        UpdateCodeAttributes {
            name: attributes.name,
            redemption_code: attributes.redemption_code,
            max_uses: attributes.max_uses.map(|m| m as i64),
            discount_in_cents: attributes.discount_in_cents.map(|d| d.map(|d2| d2 as i64)),
            discount_as_percentage: attributes
                .discount_as_percentage
                .map(|d| d.map(|d2| d2 as i64)),
            start_date: attributes.start_date,
            end_date: attributes.end_date,
            max_tickets_per_user: attributes
                .max_tickets_per_user
                .map(|m| m.map(|m2| m2 as i64)),
        }
    }
}

pub fn show(
    (conn, path, user): (Connection, Path<PathParameters>, User),
) -> Result<HttpResponse, BigNeonError> {
    let conn = conn.get();
    let code = Code::find(path.id, conn)?;
    user.requires_scope_for_organization_event(
        Scopes::CodeRead,
        &code.organization(conn)?,
        &code.event(conn)?,
        conn,
    )?;

    Ok(HttpResponse::Ok().json(code.for_display(conn)?))
}

pub fn create(
    (conn, req, path, user): (
        Connection,
        Json<CreateCodeRequest>,
        Path<PathParameters>,
        User,
    ),
) -> Result<HttpResponse, BigNeonError> {
    let conn = conn.get();
    let event = Event::find(path.id, conn)?;
    user.requires_scope_for_organization_event(
        Scopes::CodeWrite,
        &event.organization(conn)?,
        &event,
        conn,
    )?;

    if req.redemption_codes.len() != 1 {
        return application::unprocessable("Only one code allowed at this time");
    }

    let code = Code::create(
        req.name.clone(),
        path.id,
        req.code_type,
        req.redemption_codes
            .iter()
            .map(|s| s.to_uppercase())
            .next()
            .ok_or_else(|| ApplicationError::new("Code is required".to_string()))?
            .to_string(),
        req.max_uses,
        req.discount_in_cents,
        req.discount_as_percentage,
        req.start_date,
        req.end_date,
        req.max_tickets_per_user,
    )
    .commit(conn)?;

    code.update_ticket_types(req.ticket_type_ids.clone(), conn)?;
    application::created(json!(code.for_display(conn)?))
}

pub fn update(
    (conn, req, path, user): (
        Connection,
        Json<UpdateCodeRequest>,
        Path<PathParameters>,
        User,
    ),
) -> Result<HttpResponse, BigNeonError> {
    let conn = conn.get();

    let code = Code::find(path.id, conn)?;
    user.requires_scope_for_organization_event(
        Scopes::CodeWrite,
        &code.organization(conn)?,
        &code.event(conn)?,
        conn,
    )?;

    let code = code.update(req.clone().into(), conn)?;

    if let Some(ref ticket_type_ids) = req.ticket_type_ids {
        code.update_ticket_types(ticket_type_ids.clone(), conn)?;
    }

    Ok(HttpResponse::Ok().json(code.for_display(conn)?))
}

pub fn destroy(
    (conn, path, user): (Connection, Path<PathParameters>, User),
) -> Result<HttpResponse, BigNeonError> {
    let conn = conn.get();
    let code = Code::find(path.id, conn)?;
    user.requires_scope_for_organization_event(
        Scopes::CodeWrite,
        &code.organization(conn)?,
        &code.event(conn)?,
        conn,
    )?;

    code.destroy(&*conn)?;
    Ok(HttpResponse::Ok().json(json!({})))
}
