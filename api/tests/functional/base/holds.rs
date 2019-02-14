use actix_web::{http::StatusCode, FromRequest, HttpResponse, Path};
use bigneon_api::controllers::holds;
use bigneon_api::controllers::holds::*;
use bigneon_api::extractors::*;
use bigneon_api::models::PathParameters;
use bigneon_db::models::*;
use serde_json;
use support;
use support::database::TestDatabase;
use support::test_request::TestRequest;

pub fn create(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let organization = database.create_organization().finish();
    let user = database.create_user().finish();
    let event = database
        .create_event()
        .with_tickets()
        .with_organization(&organization)
        .finish();
    let auth_user =
        support::create_auth_user_from_user(&user, role, Some(&organization), &database);

    let name = "Hold Example".to_string();
    let redemption_code = "IHAVEACODE".to_string();
    let discount_in_cents = Some(10);
    let hold_type = HoldTypes::Discount;

    let json = Json(CreateHoldRequest {
        name: name.clone(),
        redemption_code: redemption_code.clone(),
        discount_in_cents,
        hold_type,
        end_at: None,
        max_per_order: None,
        quantity: 2,
        ticket_type_id: event
            .ticket_types(true, None, database.connection.get())
            .unwrap()[0]
            .id,
    });

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = event.id;

    let response: HttpResponse =
        holds::create((database.connection.into(), json, path, auth_user)).into();

    if should_test_succeed {
        let body = support::unwrap_body_to_string(&response).unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let hold: DisplayHold = serde_json::from_str(&body).unwrap();
        assert_eq!(hold.name, name);
        assert_eq!(hold.redemption_code, redemption_code);
        assert_eq!(hold.discount_in_cents, Some(10));
    } else {
        support::expects_unauthorized(&response);
    }
}

pub fn update(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let hold = database.create_hold().finish();
    let event = Event::find(hold.event_id, connection).unwrap();
    let organization = event.organization(connection).unwrap();
    let auth_user =
        support::create_auth_user_from_user(&user, role, Some(&organization), &database);
    let name = "New Name";

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = hold.id;

    let json = Json(UpdateHoldRequest {
        name: Some(name.into()),
        quantity: Some(1),
        ..Default::default()
    });

    let response: HttpResponse =
        holds::update((database.connection.clone(), json, path, auth_user)).into();
    let body = support::unwrap_body_to_string(&response).unwrap();

    if should_test_succeed {
        assert_eq!(response.status(), StatusCode::OK);
        let updated_hold: Hold = serde_json::from_str(&body).unwrap();
        assert_eq!(updated_hold.name, name);
        assert_eq!(updated_hold.quantity(&connection).unwrap(), (1, 1));
    } else {
        support::expects_unauthorized(&response);
    }
}
