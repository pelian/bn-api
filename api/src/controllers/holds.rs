use actix_web::{HttpResponse, Path};
use auth::user::User;
use bigneon_db::models::*;
use chrono::prelude::*;
use db::Connection;
use errors::BigNeonError;
use extractors::*;
use helpers::application;
use models::PathParameters;
use serde_with::rust::double_option;
use uuid::Uuid;

#[derive(Deserialize, Serialize)]
pub struct CreateHoldRequest {
    pub name: String,
    pub redemption_code: String,
    pub discount_in_cents: Option<u32>,
    pub hold_type: HoldTypes,
    pub quantity: u32,
    pub ticket_type_id: Uuid,
    pub end_at: Option<NaiveDateTime>,
    pub max_per_order: Option<u32>,
}

#[derive(Default, Deserialize, Serialize)]
pub struct UpdateHoldRequest {
    pub name: Option<String>,
    pub redemption_code: Option<String>,
    pub hold_type: Option<HoldTypes>,
    pub quantity: Option<u32>,
    #[serde(default, deserialize_with = "double_option::deserialize")]
    pub discount_in_cents: Option<Option<i64>>,
    #[serde(default, deserialize_with = "double_option_deserialize_unless_blank")]
    pub email: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option_deserialize_unless_blank")]
    pub phone: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option::deserialize")]
    pub end_at: Option<Option<NaiveDateTime>>,
    #[serde(default, deserialize_with = "double_option::deserialize")]
    pub max_per_order: Option<Option<i64>>,
}

impl From<UpdateHoldRequest> for UpdateHoldAttributes {
    fn from(attributes: UpdateHoldRequest) -> Self {
        UpdateHoldAttributes {
            name: attributes.name,
            hold_type: attributes.hold_type.and_then(|hold_type| Some(hold_type)),
            discount_in_cents: attributes.discount_in_cents,
            email: attributes.email,
            phone: attributes.phone,
            end_at: attributes.end_at,
            max_per_order: attributes.max_per_order,
            redemption_code: attributes.redemption_code,
        }
    }
}

// add update fields in here as well

pub fn create(
    (conn, req, path, user): (
        Connection,
        Json<CreateHoldRequest>,
        Path<PathParameters>,
        User,
    ),
) -> Result<HttpResponse, BigNeonError> {
    let conn = conn.get();
    let event = Event::find(path.id, conn)?;
    user.requires_scope_for_organization_event(
        Scopes::HoldWrite,
        &event.organization(conn)?,
        &event,
        conn,
    )?;

    let hold = Hold::create_hold(
        req.name.clone(),
        path.id,
        req.redemption_code.clone(),
        req.discount_in_cents,
        req.end_at,
        req.max_per_order,
        req.hold_type,
        req.ticket_type_id,
    )
    .commit(conn)?;

    hold.set_quantity(req.quantity, conn)?;

    #[derive(Serialize)]
    struct R {
        pub id: Uuid,
        pub name: String,
        pub event_id: Uuid,
        pub redemption_code: String,
        pub discount_in_cents: Option<i64>,
        pub end_at: Option<NaiveDateTime>,
        pub max_per_order: Option<i64>,
        pub hold_type: HoldTypes,
        pub ticket_type_id: Uuid,
        pub available: u32,
        pub quantity: u32,
    }

    let (quantity, available) = hold.quantity(conn)?;

    let r = R {
        id: hold.id,
        name: hold.name,
        event_id: hold.event_id,
        redemption_code: hold.redemption_code,
        discount_in_cents: hold.discount_in_cents,
        end_at: hold.end_at,
        max_per_order: hold.max_per_order,
        hold_type: hold.hold_type,
        ticket_type_id: hold.ticket_type_id,
        available,
        quantity,
    };

    application::created(json!(r))
}

pub fn update(
    (conn, req, path, user): (
        Connection,
        Json<UpdateHoldRequest>,
        Path<PathParameters>,
        User,
    ),
) -> Result<HttpResponse, BigNeonError> {
    let conn = conn.get();

    let hold = Hold::find(path.id, conn)?;
    user.requires_scope_for_organization_event(
        Scopes::HoldWrite,
        &hold.organization(conn)?,
        &hold.event(conn)?,
        conn,
    )?;
    let quantity = req.quantity;
    let hold = hold.update(req.into_inner().into(), conn)?;
    if let Some(quantity) = quantity {
        hold.set_quantity(quantity, conn)?;
    }

    Ok(HttpResponse::Ok().json(hold))
}

pub fn show(
    (conn, path, user): (Connection, Path<PathParameters>, User),
) -> Result<HttpResponse, BigNeonError> {
    let conn = conn.get();
    let hold = Hold::find(path.id, conn)?;
    user.requires_scope_for_organization_event(
        Scopes::HoldRead,
        &hold.organization(conn)?,
        &hold.event(conn)?,
        conn,
    )?;

    #[derive(Serialize)]
    struct R {
        pub id: Uuid,
        pub name: String,
        pub event_id: Uuid,
        pub redemption_code: String,
        pub discount_in_cents: Option<i64>,
        pub end_at: Option<NaiveDateTime>,
        pub max_per_order: Option<i64>,
        pub hold_type: HoldTypes,
        pub ticket_type_id: Uuid,
        pub available: u32,
        pub quantity: u32,
    }

    let (quantity, available) = hold.quantity(conn)?;

    let r = R {
        id: hold.id,
        name: hold.name,
        event_id: hold.event_id,
        redemption_code: hold.redemption_code,
        discount_in_cents: hold.discount_in_cents,
        end_at: hold.end_at,
        max_per_order: hold.max_per_order,
        hold_type: hold.hold_type,
        ticket_type_id: hold.ticket_type_id,
        available,
        quantity,
    };

    Ok(HttpResponse::Ok().json(r))
}

#[derive(Deserialize)]
pub struct SetQuantityRequest {
    pub quantity: u32,
}

#[derive(Deserialize, Serialize)]
pub struct SplitHoldRequest {
    pub name: String,
    pub redemption_code: String,
    pub discount_in_cents: Option<u32>,

    pub hold_type: HoldTypes,
    pub quantity: u32,
    pub end_at: Option<NaiveDateTime>,
    pub max_per_order: Option<u32>,
}

pub fn split(
    (conn, req, path, user): (
        Connection,
        Json<SplitHoldRequest>,
        Path<PathParameters>,
        User,
    ),
) -> Result<HttpResponse, BigNeonError> {
    let conn = conn.get();
    let hold = Hold::find(path.id, conn)?;
    user.requires_scope_for_organization_event(
        Scopes::HoldWrite,
        &hold.organization(conn)?,
        &hold.event(conn)?,
        conn,
    )?;
    let new_hold = hold.split(
        req.name.clone(),
        req.redemption_code.clone(),
        req.quantity,
        req.discount_in_cents,
        req.hold_type,
        req.end_at,
        req.max_per_order,
        conn,
    )?;
    Ok(HttpResponse::Created().json(new_hold))
}

pub fn destroy(
    (conn, path, user): (Connection, Path<PathParameters>, User),
) -> Result<HttpResponse, BigNeonError> {
    let conn = conn.get();
    let hold = Hold::find(path.id, conn)?;
    user.requires_scope_for_organization_event(
        Scopes::HoldWrite,
        &hold.organization(conn)?,
        &hold.event(conn)?,
        conn,
    )?;
    hold.destroy(conn)?;
    Ok(HttpResponse::Ok().finish())
}
