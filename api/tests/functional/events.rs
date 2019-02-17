use actix_web::Query;
use actix_web::{http::StatusCode, FromRequest, HttpResponse, Path};
use bigneon_api::controllers::events;
use bigneon_api::controllers::events::*;
use bigneon_api::extractors::*;
use bigneon_api::models::*;
use bigneon_db::models::*;
use chrono::prelude::*;
use diesel::PgConnection;
use functional::base;
use serde_json;
use serde_json::Value;
use std::collections::HashMap;
use support;
use support::database::TestDatabase;
use support::test_request::TestRequest;
use uuid::Uuid;

#[test]
pub fn index() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let organization = database.create_organization().finish();
    let venue = database.create_venue().finish();
    let event = database
        .create_event()
        .with_name("NewEvent1".to_string())
        .with_organization(&organization)
        .with_venue(&venue)
        .with_event_start(NaiveDate::from_ymd(2022, 7, 8).and_hms(9, 10, 11))
        .with_event_end(NaiveDate::from_ymd(2022, 7, 9).and_hms(9, 10, 11))
        .finish();
    let event2 = database
        .create_event()
        .with_name("NewEvent2".to_string())
        .with_organization(&organization)
        .with_venue(&venue)
        .with_event_start(NaiveDate::from_ymd(2022, 7, 8).and_hms(9, 10, 11))
        .with_event_end(NaiveDate::from_ymd(2022, 7, 9).and_hms(9, 10, 11))
        .finish();

    let expected_results = vec![
        event_venue_entry(&event, &venue, None, &*connection),
        event_venue_entry(&event2, &venue, None, &*connection),
    ];

    let test_request = TestRequest::create_with_uri("/events?query=New");
    let parameters = Query::<SearchParameters>::extract(&test_request.request).unwrap();
    let response: HttpResponse = events::index((
        test_request.extract_state(),
        database.connection.clone().into(),
        parameters,
        OptionalUser(None),
    ))
    .into();

    let body = support::unwrap_body_to_string(&response).unwrap();

    let mut expected_tags: HashMap<String, Value> = HashMap::new();
    expected_tags.insert("query".to_string(), json!("New"));
    let wrapped_expected_events = Payload {
        data: expected_results,
        paging: Paging {
            page: 0,
            limit: 100,
            sort: "".to_string(),
            dir: SortingDir::Asc,
            total: 2,
            tags: expected_tags,
        },
    };
    let expected_json = serde_json::to_string(&wrapped_expected_events).unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body, expected_json);
}

#[test]
pub fn index_for_user() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let organization = database.create_organization().finish();
    let user = database.create_user().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);
    let venue = database.create_venue().finish();
    let event = database
        .create_event()
        .with_name("NewEvent1".to_string())
        .with_organization(&organization)
        .with_venue(&venue)
        .with_event_start(NaiveDate::from_ymd(2022, 7, 8).and_hms(9, 10, 11))
        .with_event_end(NaiveDate::from_ymd(2022, 7, 9).and_hms(9, 10, 11))
        .finish();
    let _event_interest = EventInterest::create(event.id, user.id).commit(connection);
    let event2 = database
        .create_event()
        .with_name("NewEvent2".to_string())
        .with_organization(&organization)
        .with_venue(&venue)
        .with_event_start(NaiveDate::from_ymd(2022, 7, 8).and_hms(9, 10, 11))
        .with_event_end(NaiveDate::from_ymd(2022, 7, 9).and_hms(9, 10, 11))
        .finish();

    let expected_results = vec![
        event_venue_entry(&event, &venue, Some(user.clone()), &*connection),
        event_venue_entry(&event2, &venue, Some(user), &*connection),
    ];

    let test_request = TestRequest::create_with_uri("/events?query=New");
    let parameters = Query::<SearchParameters>::extract(&test_request.request).unwrap();
    let response: HttpResponse = events::index((
        test_request.extract_state(),
        database.connection.clone().into(),
        parameters,
        OptionalUser(Some(auth_user)),
    ))
    .into();

    let body = support::unwrap_body_to_string(&response).unwrap();

    let mut expected_tags: HashMap<String, Value> = HashMap::new();
    expected_tags.insert("query".to_string(), json!("New"));
    let wrapped_expected_events = Payload {
        data: expected_results,
        paging: Paging {
            page: 0,
            limit: 100,
            sort: "".to_string(),
            dir: SortingDir::Asc,
            total: 2,
            tags: expected_tags,
        },
    };
    let expected_json = serde_json::to_string(&wrapped_expected_events).unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body, expected_json);
}

#[test]
pub fn index_with_draft_for_organization_user() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let auth_user = support::create_auth_user_from_user(
        &user,
        Roles::OrgMember,
        Some(&organization),
        &database,
    );
    let venue = database.create_venue().finish();
    let event = database
        .create_event()
        .with_name("NewEvent1".to_string())
        .with_organization(&organization)
        .with_venue(&venue)
        .with_event_start(NaiveDate::from_ymd(2022, 7, 8).and_hms(9, 10, 11))
        .with_event_end(NaiveDate::from_ymd(2022, 7, 9).and_hms(9, 10, 11))
        .finish();
    let event2 = database
        .create_event()
        .with_name("NewEvent2".to_string())
        .with_status(EventStatus::Draft)
        .with_organization(&organization)
        .with_venue(&venue)
        .with_event_start(NaiveDate::from_ymd(2022, 7, 8).and_hms(9, 10, 11))
        .with_event_end(NaiveDate::from_ymd(2022, 7, 9).and_hms(9, 10, 11))
        .finish();

    let expected_results = vec![
        event_venue_entry(&event, &venue, None, &*connection),
        event_venue_entry(&event2, &venue, None, &*connection),
    ];

    let test_request = TestRequest::create_with_uri("/events?query=New");
    let parameters = Query::<SearchParameters>::extract(&test_request.request).unwrap();
    let response: HttpResponse = events::index((
        test_request.extract_state(),
        database.connection.clone().into(),
        parameters,
        OptionalUser(Some(auth_user)),
    ))
    .into();

    let body = support::unwrap_body_to_string(&response).unwrap();

    let mut expected_tags: HashMap<String, Value> = HashMap::new();
    expected_tags.insert("query".to_string(), json!("New"));
    let wrapped_expected_events = Payload {
        data: expected_results,
        paging: Paging {
            page: 0,
            limit: 100,
            sort: "".to_string(),
            dir: SortingDir::Asc,
            total: 2,
            tags: expected_tags,
        },
    };
    let expected_json = serde_json::to_string(&wrapped_expected_events).unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body, expected_json);
}

#[test]
pub fn index_with_draft_for_user_ignores_drafts() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);
    let organization = database.create_organization().finish();
    let venue = database.create_venue().finish();
    let event = database
        .create_event()
        .with_name("NewEvent1".to_string())
        .with_organization(&organization)
        .with_venue(&venue)
        .with_event_start(NaiveDate::from_ymd(2022, 7, 8).and_hms(9, 10, 11))
        .with_event_end(NaiveDate::from_ymd(2022, 7, 9).and_hms(9, 10, 11))
        .finish();
    let _event2 = database
        .create_event()
        .with_name("NewEvent2".to_string())
        .with_status(EventStatus::Draft)
        .with_organization(&organization)
        .with_venue(&venue)
        .with_event_start(NaiveDate::from_ymd(2022, 7, 8).and_hms(9, 10, 11))
        .with_event_end(NaiveDate::from_ymd(2022, 7, 9).and_hms(9, 10, 11))
        .finish();

    let expected_results = vec![event_venue_entry(&event, &venue, None, &*connection)];

    let test_request = TestRequest::create_with_uri("/events?query=New");
    let parameters = Query::<SearchParameters>::extract(&test_request.request).unwrap();
    let response: HttpResponse = events::index((
        test_request.extract_state(),
        database.connection.clone().into(),
        parameters,
        OptionalUser(Some(auth_user)),
    ))
    .into();

    let body = support::unwrap_body_to_string(&response).unwrap();
    let mut expected_tags: HashMap<String, Value> = HashMap::new();
    expected_tags.insert("query".to_string(), json!("New"));
    let wrapped_expected_events = Payload {
        data: expected_results,
        paging: Paging {
            page: 0,
            limit: 100,
            sort: "".to_string(),
            dir: SortingDir::Asc,
            total: 1,
            tags: expected_tags,
        },
    };
    let expected_json = serde_json::to_string(&wrapped_expected_events).unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body, expected_json);
}

#[test]
pub fn index_search_with_filter() {
    let database = TestDatabase::new();
    let organization = database.create_organization().finish();
    let event = database
        .create_event()
        .with_name("NewEvent1".to_string())
        .with_organization(&organization)
        .with_event_start(NaiveDate::from_ymd(2022, 7, 8).and_hms(9, 10, 11))
        .with_event_end(NaiveDate::from_ymd(2022, 7, 9).and_hms(9, 10, 11))
        .finish();
    database
        .create_event()
        .with_name("NewEvent2".to_string())
        .with_organization(&organization)
        .with_event_start(NaiveDate::from_ymd(2022, 7, 8).and_hms(9, 10, 11))
        .with_event_end(NaiveDate::from_ymd(2022, 7, 9).and_hms(9, 10, 11))
        .finish();

    let localized_times = event.get_all_localized_time_strings(&None);
    let expected_events = vec![EventVenueEntry {
        id: event.id,
        name: event.name,
        organization_id: event.organization_id,
        venue_id: event.venue_id,
        created_at: event.created_at,
        event_start: event.event_start,
        door_time: event.door_time,
        status: event.status,
        publish_date: event.publish_date,
        promo_image_url: event.promo_image_url,
        additional_info: event.additional_info,
        top_line_info: event.top_line_info,
        age_limit: event.age_limit,
        cancelled_at: event.cancelled_at,
        venue: None,
        min_ticket_price: None,
        max_ticket_price: None,
        is_external: false,
        external_url: None,
        user_is_interested: false,
        localized_times,
        tracking_keys: TrackingKeys {
            ..Default::default()
        },
        event_type: EventTypes::Music,
    }];

    let test_request = TestRequest::create_with_uri("/events?query=NewEvent1");
    let parameters = Query::<SearchParameters>::extract(&test_request.request).unwrap();
    let response: HttpResponse = events::index((
        test_request.extract_state(),
        database.connection.into(),
        parameters,
        OptionalUser(None),
    ))
    .into();

    let body = support::unwrap_body_to_string(&response).unwrap();
    let mut expected_tags: HashMap<String, Value> = HashMap::new();
    expected_tags.insert("query".to_string(), json!("NewEvent1"));
    let wrapped_expected_events = Payload {
        data: expected_events,
        paging: Paging {
            page: 0,
            limit: 100,
            sort: "".to_string(),
            dir: SortingDir::Asc,
            total: 1,
            tags: expected_tags,
        },
    };
    let expected_json = serde_json::to_string(&wrapped_expected_events).unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body, expected_json);
}

#[test]
fn show() {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);

    let organization = database.create_organization().finish();
    let venue = database.create_venue().finish();
    let event = database
        .create_event()
        .with_name("NewEvent".to_string())
        .with_organization(&organization)
        .with_venue(&venue)
        .with_ticket_pricing()
        .finish();
    let event_id = event.id;

    let artist1 = database.create_artist().finish();
    let artist2 = database.create_artist().finish();
    let conn = database.connection.get();

    event.add_artist(artist1.id, conn).unwrap();
    event.add_artist(artist2.id, conn).unwrap();

    let _event_interest = EventInterest::create(event.id, user.id).commit(conn);
    let test_request = TestRequest::create_with_uri(&format!("/events/{}", event.id));
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    let event_expected_json =
        base::events::expected_show_json(event, organization, venue, false, None, None, conn);
    path.id = event_id;
    let query_parameters = Query::<EventParameters>::extract(&test_request.request).unwrap();

    let response: HttpResponse = events::show((
        test_request.extract_state(),
        database.connection.clone(),
        path,
        query_parameters,
        OptionalUser(Some(auth_user)),
    ))
    .into();
    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body, event_expected_json);
}

#[test]
fn show_with_cancelled_ticket_type() {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);

    let organization = database.create_organization().finish();
    let venue = database.create_venue().finish();
    let event = database
        .create_event()
        .with_name("NewEvent".to_string())
        .with_organization(&organization)
        .with_venue(&venue)
        .with_ticket_pricing()
        .with_ticket_type_count(2)
        .finish();
    let event_id = event.id;

    let artist1 = database.create_artist().finish();
    let artist2 = database.create_artist().finish();
    let conn = database.connection.get();

    event.add_artist(artist1.id, conn).unwrap();
    event.add_artist(artist2.id, conn).unwrap();

    let ticket_type = &event.ticket_types(true, None, conn).unwrap()[0];
    let _cancelled_ticket_type = ticket_type.cancel(conn).unwrap();

    let _event_interest = EventInterest::create(event.id, user.id).commit(conn);
    let test_request = TestRequest::create_with_uri(&format!("/events/{}", event.id));
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    let event_expected_json =
        base::events::expected_show_json(event, organization, venue, false, None, None, conn);
    path.id = event_id;
    let query_parameters = Query::<EventParameters>::extract(&test_request.request).unwrap();

    let response: HttpResponse = events::show((
        test_request.extract_state(),
        database.connection.clone(),
        path,
        query_parameters,
        OptionalUser(Some(auth_user)),
    ))
    .into();
    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body, event_expected_json);
}

#[test]
fn show_with_access_restricted_ticket_type_and_no_code() {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);

    let organization = database.create_organization().finish();
    let venue = database.create_venue().finish();
    let event = database
        .create_event()
        .with_name("NewEvent".to_string())
        .with_organization(&organization)
        .with_venue(&venue)
        .with_ticket_pricing()
        .with_ticket_type_count(2)
        .finish();
    let event_id = event.id;

    let artist1 = database.create_artist().finish();
    let artist2 = database.create_artist().finish();
    let conn = database.connection.get();

    event.add_artist(artist1.id, conn).unwrap();
    event.add_artist(artist2.id, conn).unwrap();

    let ticket_types = &event.ticket_types(true, None, conn).unwrap();
    let ticket_type = &ticket_types[0];
    let ticket_type2 = &ticket_types[1];
    let _code = database
        .create_code()
        .with_code_type(CodeTypes::Access)
        .with_event(&event)
        .for_ticket_type(&ticket_type2)
        .finish();

    let _event_interest = EventInterest::create(event.id, user.id).commit(conn);
    let test_request = TestRequest::create_with_uri(&format!("/events/{}", event.id));
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    let event_expected_json = base::events::expected_show_json(
        event,
        organization,
        venue,
        false,
        None,
        Some(vec![ticket_type.id]),
        conn,
    );
    path.id = event_id;
    let query_parameters = Query::<EventParameters>::extract(&test_request.request).unwrap();
    assert_eq!(query_parameters.redemption_code, None);

    let response: HttpResponse = events::show((
        test_request.extract_state(),
        database.connection.clone(),
        path,
        query_parameters,
        OptionalUser(Some(auth_user)),
    ))
    .into();
    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body, event_expected_json);
}

#[test]
fn show_with_access_restricted_ticket_type_and_access_code() {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);

    let organization = database.create_organization().finish();
    let venue = database.create_venue().finish();
    let event = database
        .create_event()
        .with_name("NewEvent".to_string())
        .with_organization(&organization)
        .with_venue(&venue)
        .with_ticket_pricing()
        .with_ticket_type_count(2)
        .finish();
    let event_id = event.id;

    let artist1 = database.create_artist().finish();
    let artist2 = database.create_artist().finish();
    let conn = database.connection.get();

    event.add_artist(artist1.id, conn).unwrap();
    event.add_artist(artist2.id, conn).unwrap();

    let ticket_types = &event.ticket_types(true, None, conn).unwrap();
    let ticket_type = &ticket_types[0];
    let ticket_type2 = &ticket_types[1];
    let code = database
        .create_code()
        .with_code_type(CodeTypes::Access)
        .with_event(&event)
        .for_ticket_type(&ticket_type2)
        .finish();

    let _event_interest = EventInterest::create(event.id, user.id).commit(conn);
    let test_request = TestRequest::create_with_uri(&format!(
        "/events/{}?redemption_code={}",
        event.id, code.redemption_code
    ));
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    let event_expected_json = base::events::expected_show_json(
        event,
        organization,
        venue,
        false,
        Some(code.redemption_code.clone()),
        Some(vec![ticket_type.id, ticket_type2.id]),
        conn,
    );
    path.id = event_id;
    let query_parameters = Query::<EventParameters>::extract(&test_request.request).unwrap();
    assert_eq!(query_parameters.redemption_code, Some(code.redemption_code));

    let response: HttpResponse = events::show((
        test_request.extract_state(),
        database.connection.clone(),
        path,
        query_parameters,
        OptionalUser(Some(auth_user)),
    ))
    .into();
    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body, event_expected_json);
}

#[cfg(test)]
mod show_box_office_pricing_tests {
    use super::*;
    #[test]
    fn show_box_office_pricing_org_member() {
        base::events::show_box_office_pricing(Roles::OrgMember, true);
    }
    #[test]
    fn show_box_office_pricing_admin() {
        base::events::show_box_office_pricing(Roles::Admin, true);
    }
    #[test]
    fn show_box_office_pricing_user() {
        base::events::show_box_office_pricing(Roles::User, false);
    }
    #[test]
    fn show_box_office_pricing_org_owner() {
        base::events::show_box_office_pricing(Roles::OrgOwner, true);
    }
    #[test]
    fn show_box_office_pricing_door_person() {
        base::events::show_box_office_pricing(Roles::DoorPerson, false);
    }
    #[test]
    fn show_box_office_pricing_promoter() {
        base::events::show_box_office_pricing(Roles::Promoter, false);
    }
    #[test]
    fn show_box_office_pricing_promoter_read_only() {
        base::events::show_box_office_pricing(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn show_box_office_pricing_org_admin() {
        base::events::show_box_office_pricing(Roles::OrgAdmin, true);
    }
    #[test]
    fn show_box_office_pricing_box_office() {
        base::events::show_box_office_pricing(Roles::OrgBoxOffice, true);
    }
}

#[cfg(test)]
mod dashboard_tests {
    use super::*;
    #[test]
    fn dashboard_org_member() {
        base::events::dashboard(Roles::OrgMember, true);
    }
    #[test]
    fn dashboard_admin() {
        base::events::dashboard(Roles::Admin, true);
    }
    #[test]
    fn dashboard_user() {
        base::events::dashboard(Roles::User, false);
    }
    #[test]
    fn dashboard_org_owner() {
        base::events::dashboard(Roles::OrgOwner, true);
    }
    #[test]
    fn dashboard_door_person() {
        base::events::dashboard(Roles::DoorPerson, false);
    }
    #[test]
    fn dashboard_promoter() {
        base::events::dashboard(Roles::Promoter, true);
    }
    #[test]
    fn dashboard_promoter_read_only() {
        base::events::dashboard(Roles::PromoterReadOnly, true);
    }
    #[test]
    fn dashboard_org_admin() {
        base::events::dashboard(Roles::OrgAdmin, true);
    }
    #[test]
    fn dashboard_box_office() {
        base::events::dashboard(Roles::OrgBoxOffice, true);
    }
}

#[cfg(test)]
mod create_tests {
    use super::*;
    #[test]
    fn create_org_member() {
        base::events::create(Roles::OrgMember, true);
    }
    #[test]
    fn create_admin() {
        base::events::create(Roles::Admin, true);
    }
    #[test]
    fn create_user() {
        base::events::create(Roles::User, false);
    }
    #[test]
    fn create_org_owner() {
        base::events::create(Roles::OrgOwner, true);
    }
    #[test]
    fn create_door_person() {
        base::events::create(Roles::DoorPerson, false);
    }
    #[test]
    fn create_promoter() {
        base::events::create(Roles::Promoter, true);
    }
    #[test]
    fn create_promoter_read_only() {
        base::events::create(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn create_org_admin() {
        base::events::create(Roles::OrgAdmin, true);
    }
    #[test]
    fn create_box_office() {
        base::events::create(Roles::OrgBoxOffice, false);
    }
}

#[cfg(test)]
mod update_tests {
    use super::*;
    #[test]
    fn update_org_member() {
        base::events::update(Roles::OrgMember, true);
    }
    #[test]
    fn update_admin() {
        base::events::update(Roles::Admin, true);
    }
    #[test]
    fn update_user() {
        base::events::update(Roles::User, false);
    }
    #[test]
    fn update_org_owner() {
        base::events::update(Roles::OrgOwner, true);
    }
    #[test]
    fn update_door_person() {
        base::events::update(Roles::DoorPerson, false);
    }
    #[test]
    fn update_promoter() {
        base::events::update(Roles::Promoter, true);
    }
    #[test]
    fn update_promoter_read_only() {
        base::events::update(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn update_org_admin() {
        base::events::update(Roles::OrgAdmin, true);
    }
    #[test]
    fn update_box_office() {
        base::events::update(Roles::OrgBoxOffice, false);
    }
}

#[cfg(test)]
mod cancel_tests {
    use super::*;
    #[test]
    fn cancel_org_member() {
        base::events::cancel(Roles::OrgMember, true);
    }
    #[test]
    fn cancel_admin() {
        base::events::cancel(Roles::Admin, true);
    }
    #[test]
    fn cancel_user() {
        base::events::cancel(Roles::User, false);
    }
    #[test]
    fn cancel_org_owner() {
        base::events::cancel(Roles::OrgOwner, true);
    }
    #[test]
    fn cancel_door_person() {
        base::events::cancel(Roles::DoorPerson, false);
    }
    #[test]
    fn cancel_promoter() {
        base::events::cancel(Roles::Promoter, false);
    }
    #[test]
    fn cancel_promoter_read_only() {
        base::events::cancel(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn cancel_org_admin() {
        base::events::cancel(Roles::OrgAdmin, true);
    }
    #[test]
    fn cancel_box_office() {
        base::events::cancel(Roles::OrgBoxOffice, false);
    }
}

#[cfg(test)]
mod add_artist_tests {
    use super::*;
    #[test]
    fn add_artist_org_member() {
        base::events::add_artist(Roles::OrgMember, true);
    }
    #[test]
    fn add_artist_admin() {
        base::events::add_artist(Roles::Admin, true);
    }
    #[test]
    fn add_artist_user() {
        base::events::add_artist(Roles::User, false);
    }
    #[test]
    fn add_artist_org_owner() {
        base::events::add_artist(Roles::OrgOwner, true);
    }
    #[test]
    fn add_artist_door_person() {
        base::events::add_artist(Roles::DoorPerson, false);
    }
    #[test]
    fn add_artist_promoter() {
        base::events::add_artist(Roles::Promoter, true)
    }
    #[test]
    fn add_artist_promoter_read_only() {
        base::events::add_artist(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn add_artist_org_admin() {
        base::events::add_artist(Roles::OrgAdmin, true);
    }
    #[test]
    fn add_artist_box_office() {
        base::events::add_artist(Roles::OrgBoxOffice, false);
    }
}

#[cfg(test)]
mod list_interested_users_tests {
    use super::*;
    #[test]
    fn list_interested_users_org_member() {
        base::events::list_interested_users(Roles::OrgMember, true);
    }
    #[test]
    fn list_interested_users_admin() {
        base::events::list_interested_users(Roles::Admin, true);
    }
    #[test]
    fn list_interested_users_user() {
        base::events::list_interested_users(Roles::User, true);
    }
    #[test]
    fn list_interested_users_org_owner() {
        base::events::list_interested_users(Roles::OrgOwner, true);
    }
    #[test]
    fn list_interested_users_door_person() {
        base::events::list_interested_users(Roles::DoorPerson, true);
    }
    #[test]
    fn list_interested_users_promoter() {
        base::events::list_interested_users(Roles::Promoter, true);
    }
    #[test]
    fn list_interested_users_promoter_read_only() {
        base::events::list_interested_users(Roles::PromoterReadOnly, true);
    }
    #[test]
    fn list_interested_users_org_admin() {
        base::events::list_interested_users(Roles::OrgAdmin, true);
    }
    #[test]
    fn list_interested_users_box_office() {
        base::events::list_interested_users(Roles::OrgBoxOffice, true);
    }
}

#[cfg(test)]
mod add_interest_tests {
    use super::*;
    #[test]
    fn add_interest_org_member() {
        base::events::add_interest(Roles::OrgMember, true);
    }
    #[test]
    fn add_interest_admin() {
        base::events::add_interest(Roles::Admin, true);
    }
    #[test]
    fn add_interest_user() {
        base::events::add_interest(Roles::User, true);
    }
    #[test]
    fn add_interest_org_owner() {
        base::events::add_interest(Roles::OrgOwner, true);
    }
    #[test]
    fn add_interest_door_person() {
        base::events::add_interest(Roles::DoorPerson, true);
    }
    #[test]
    fn add_interest_promoter() {
        base::events::add_interest(Roles::Promoter, true);
    }
    #[test]
    fn add_interest_promoter_read_only() {
        base::events::add_interest(Roles::PromoterReadOnly, true);
    }
    #[test]
    fn add_interest_org_admin() {
        base::events::add_interest(Roles::OrgAdmin, true);
    }
    #[test]
    fn add_interest_box_office() {
        base::events::add_interest(Roles::OrgBoxOffice, true);
    }
}

#[cfg(test)]
mod remove_interest_tests {
    use super::*;
    #[test]
    fn remove_interest_org_member() {
        base::events::remove_interest(Roles::OrgMember, true);
    }
    #[test]
    fn remove_interest_admin() {
        base::events::remove_interest(Roles::Admin, true);
    }
    #[test]
    fn remove_interest_user() {
        base::events::remove_interest(Roles::User, true);
    }
    #[test]
    fn remove_interest_org_owner() {
        base::events::remove_interest(Roles::OrgOwner, true);
    }
    #[test]
    fn remove_interest_door_person() {
        base::events::remove_interest(Roles::DoorPerson, true);
    }
    #[test]
    fn remove_interest_promoter() {
        base::events::remove_interest(Roles::Promoter, true);
    }
    #[test]
    fn remove_interest_promoter_read_only() {
        base::events::remove_interest(Roles::PromoterReadOnly, true);
    }
    #[test]
    fn remove_interest_org_admin() {
        base::events::remove_interest(Roles::OrgAdmin, true);
    }
    #[test]
    fn remove_interest_box_office() {
        base::events::remove_interest(Roles::OrgBoxOffice, true);
    }
}

#[cfg(test)]
mod update_artists_tests {
    use super::*;
    #[test]
    fn update_artists_org_member() {
        base::events::update_artists(Roles::OrgMember, true);
    }
    #[test]
    fn update_artists_admin() {
        base::events::update_artists(Roles::Admin, true);
    }
    #[test]
    fn update_artists_user() {
        base::events::update_artists(Roles::User, false);
    }
    #[test]
    fn update_artists_org_owner() {
        base::events::update_artists(Roles::OrgOwner, true);
    }
    #[test]
    fn update_artists_door_person() {
        base::events::update_artists(Roles::DoorPerson, false);
    }
    #[test]
    fn update_artists_promoter() {
        base::events::update_artists(Roles::Promoter, true);
    }
    #[test]
    fn update_artists_promoter_read_only() {
        base::events::update_artists(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn update_artists_org_admin() {
        base::events::update_artists(Roles::OrgAdmin, true);
    }
    #[test]
    fn update_artists_box_office() {
        base::events::update_artists(Roles::OrgBoxOffice, false);
    }
}

#[cfg(test)]
mod guest_list_tests {
    use super::*;
    #[test]
    fn guest_list_org_member() {
        base::events::guest_list(Roles::OrgMember, true);
    }
    #[test]
    fn guest_list_admin() {
        base::events::guest_list(Roles::Admin, true);
    }
    #[test]
    fn guest_list_user() {
        base::events::guest_list(Roles::User, false);
    }
    #[test]
    fn guest_list_org_owner() {
        base::events::guest_list(Roles::OrgOwner, true);
    }
    #[test]
    fn guest_list_door_person() {
        base::events::guest_list(Roles::DoorPerson, false);
    }
    #[test]
    fn guest_list_promoter() {
        base::events::guest_list(Roles::Promoter, true);
    }
    #[test]
    fn guest_list_promoter_read_only() {
        base::events::guest_list(Roles::PromoterReadOnly, true);
    }
    #[test]
    fn guest_list_org_admin() {
        base::events::guest_list(Roles::OrgAdmin, true);
    }
    #[test]
    fn guest_list_box_office() {
        base::events::guest_list(Roles::OrgBoxOffice, true);
    }
}

#[cfg(test)]
mod codes_tests {
    use super::*;
    #[test]
    fn codes_org_member() {
        base::events::codes(Roles::OrgMember, true);
    }
    #[test]
    fn codes_admin() {
        base::events::codes(Roles::Admin, true);
    }
    #[test]
    fn codes_user() {
        base::events::codes(Roles::User, false);
    }
    #[test]
    fn codes_org_owner() {
        base::events::codes(Roles::OrgOwner, true);
    }
    #[test]
    fn codes_door_person() {
        base::events::codes(Roles::DoorPerson, false);
    }
    #[test]
    fn codes_promoter() {
        base::events::codes(Roles::Promoter, true);
    }
    #[test]
    fn codes_promoter_read_only() {
        base::events::codes(Roles::PromoterReadOnly, true);
    }
    #[test]
    fn codes_org_admin() {
        base::events::codes(Roles::OrgAdmin, true);
    }
    #[test]
    fn codes_box_office() {
        base::events::codes(Roles::OrgBoxOffice, false);
    }
}

#[cfg(test)]
mod holds_tests {
    use super::*;
    #[test]
    fn holds_org_member() {
        base::events::holds(Roles::OrgMember, true);
    }
    #[test]
    fn holds_admin() {
        base::events::holds(Roles::Admin, true);
    }
    #[test]
    fn holds_user() {
        base::events::holds(Roles::User, false);
    }
    #[test]
    fn holds_org_owner() {
        base::events::holds(Roles::OrgOwner, true);
    }
    #[test]
    fn holds_door_person() {
        base::events::holds(Roles::DoorPerson, true);
    }
    #[test]
    fn holds_promoter() {
        base::events::holds(Roles::Promoter, true);
    }
    #[test]
    fn holds_promoter_read_only() {
        base::events::holds(Roles::PromoterReadOnly, true);
    }
    #[test]
    fn holds_org_admin() {
        base::events::holds(Roles::OrgAdmin, true);
    }
    #[test]
    fn holds_box_office() {
        base::events::holds(Roles::OrgBoxOffice, true);
    }
}

#[test]
fn update_promoter_fails_lacks_event_id() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let event = database
        .create_event()
        .with_organization(&organization)
        .finish();
    let auth_user =
        support::create_auth_user_from_user(&user, Roles::Promoter, Some(&organization), &database);

    // Remove event access
    OrganizationUser::create(organization.id, user.id, vec![Roles::Promoter], vec![])
        .commit(connection)
        .unwrap();

    let new_name = "New Event Name";
    let test_request = TestRequest::create();

    let json = Json(EventEditableAttributes {
        name: Some(new_name.to_string()),
        ..Default::default()
    });
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = event.id;

    let response: HttpResponse = events::update((
        database.connection.clone().into(),
        path,
        json,
        auth_user.clone(),
    ))
    .into();
    support::expects_unauthorized(&response);
}

#[test]
fn dashboard_with_default_range() {
    let database = TestDatabase::new();

    let admin = database.create_user().finish();
    let connection = database.connection.get();
    let org_admin = database.create_user().finish();
    let organization = database
        .create_organization()
        .with_fee_schedule(&database.create_fee_schedule().finish(admin.id))
        .finish();
    let auth_user = support::create_auth_user_from_user(
        &org_admin,
        Roles::OrgOwner,
        Some(&organization),
        &database,
    );
    let event = database
        .create_event()
        .with_organization(&organization)
        .with_event_start(NaiveDate::from_ymd(2016, 7, 8).and_hms(9, 10, 11))
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];

    // org_admin purchases 10 tickets
    let mut cart = Order::find_or_create_cart(&org_admin, connection).unwrap();
    cart.update_quantities(
        org_admin.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 10,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();
    assert_eq!(cart.calculate_total(connection).unwrap(), 1700);
    cart.add_external_payment(Some("test".to_string()), org_admin.id, 1700, connection)
        .unwrap();
    assert_eq!(cart.status, OrderStatus::Paid);

    let test_request = TestRequest::create_with_uri(&format!("/events/{}/dashboard?", event.id));
    let query_parameters = Query::<DashboardParameters>::extract(&test_request.request).unwrap();
    let mut path_parameters = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path_parameters.id = event.id;

    let response: HttpResponse = events::dashboard((
        database.connection.clone().into(),
        path_parameters,
        query_parameters,
        auth_user.clone(),
    ))
    .into();

    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();
    let dashboard_result: DashboardResult = serde_json::from_str(&body).unwrap();
    assert_eq!(dashboard_result.day_stats.len(), 30);
    assert_eq!(
        dashboard_result.day_stats[29],
        DayStats {
            date: Utc::now().naive_utc().date(),
            revenue_in_cents: 1500,
            ticket_sales: 10,
        }
    );
}

#[test]
pub fn show_from_organizations_past() {
    let database = TestDatabase::new();

    let organization = database.create_organization().finish();
    let event = database
        .create_event()
        .with_name("NewEvent".to_string())
        .with_event_start(
            NaiveDateTime::parse_from_str("2014-03-04 12:00:00.000", "%Y-%m-%d %H:%M:%S%.f")
                .unwrap(),
        )
        .with_event_end(
            NaiveDateTime::parse_from_str("2014-03-05 12:00:00.000", "%Y-%m-%d %H:%M:%S%.f")
                .unwrap(),
        )
        .with_organization(&organization)
        .finish();
    let _event2 = database
        .create_event()
        .with_name("NewEvent2".to_string())
        .with_event_start(
            NaiveDateTime::parse_from_str("2059-03-02 12:00:00.000", "%Y-%m-%d %H:%M:%S%.f")
                .unwrap(),
        )
        .with_event_end(
            NaiveDateTime::parse_from_str("2059-03-03 12:00:00.000", "%Y-%m-%d %H:%M:%S%.f")
                .unwrap(),
        )
        .with_organization(&organization)
        .finish();

    let user = database.create_user().finish();
    let auth_user = support::create_auth_user_from_user(
        &user,
        Roles::OrgMember,
        Some(&organization),
        &database,
    );

    let expected_events = vec![event.id];

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = organization.id;
    let test_request = TestRequest::create_with_uri(&format!("/events?past_or_upcoming=Past"));
    let query_parameters = Query::<PagingParameters>::extract(&test_request.request).unwrap();
    let response = events::show_from_organizations((
        database.connection.into(),
        path,
        query_parameters,
        auth_user,
    ))
    .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        expected_events,
        response
            .payload()
            .data
            .iter()
            .map(|i| i.id)
            .collect::<Vec<Uuid>>()
    );
}

#[test]
pub fn show_from_organizations_upcoming() {
    let database = TestDatabase::new();

    let organization = database.create_organization().finish();
    let _event = database
        .create_event()
        .with_name("NewEvent".to_string())
        .with_event_start(
            NaiveDateTime::parse_from_str("2014-03-04 12:00:00.000", "%Y-%m-%d %H:%M:%S%.f")
                .unwrap(),
        )
        .with_event_end(
            NaiveDateTime::parse_from_str("2014-03-05 12:00:00.000", "%Y-%m-%d %H:%M:%S%.f")
                .unwrap(),
        )
        .with_organization(&organization)
        .finish();
    let event2 = database
        .create_event()
        .with_event_start(
            NaiveDateTime::parse_from_str("2059-03-02 12:00:00.000", "%Y-%m-%d %H:%M:%S%.f")
                .unwrap(),
        )
        .with_event_end(
            NaiveDateTime::parse_from_str("2059-03-03 12:00:00.000", "%Y-%m-%d %H:%M:%S%.f")
                .unwrap(),
        )
        .with_name("NewEvent2".to_string())
        .with_organization(&organization)
        .finish();

    let user = database.create_user().finish();
    let auth_user = support::create_auth_user_from_user(
        &user,
        Roles::OrgMember,
        Some(&organization),
        &database,
    );

    let expected_events = vec![event2.id];
    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = organization.id;
    let test_request = TestRequest::create_with_uri(&format!("/events?past_or_upcoming=Upcoming"));
    let query_parameters = Query::<PagingParameters>::extract(&test_request.request).unwrap();
    let response = events::show_from_organizations((
        database.connection.into(),
        path,
        query_parameters,
        auth_user,
    ))
    .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        expected_events,
        response
            .payload()
            .data
            .iter()
            .map(|i| i.id)
            .collect::<Vec<Uuid>>()
    );
}

#[derive(Serialize)]
struct EventVenueEntry {
    id: Uuid,
    name: String,
    organization_id: Uuid,
    venue_id: Option<Uuid>,
    created_at: NaiveDateTime,
    event_start: Option<NaiveDateTime>,
    door_time: Option<NaiveDateTime>,
    status: EventStatus,
    publish_date: Option<NaiveDateTime>,
    promo_image_url: Option<String>,
    additional_info: Option<String>,
    top_line_info: Option<String>,
    age_limit: Option<i32>,
    cancelled_at: Option<NaiveDateTime>,
    venue: Option<Venue>,
    min_ticket_price: Option<i64>,
    max_ticket_price: Option<i64>,
    is_external: bool,
    external_url: Option<String>,
    user_is_interested: bool,
    localized_times: EventLocalizedTimeStrings,
    tracking_keys: TrackingKeys,
    event_type: EventTypes,
}

fn event_venue_entry(
    event: &Event,
    venue: &Venue,
    user: Option<User>,
    connection: &PgConnection,
) -> EventVenueEntry {
    let localized_times = event.get_all_localized_time_strings(&Some(venue.clone()));
    let (min_ticket_price, max_ticket_price) = event
        .current_ticket_pricing_range(false, connection)
        .unwrap();
    EventVenueEntry {
        id: event.id,
        name: event.name.clone(),
        organization_id: event.organization_id,
        venue_id: event.venue_id,
        created_at: event.created_at,
        event_start: event.event_start,
        door_time: event.door_time,
        status: event.status.clone(),
        publish_date: event.publish_date,
        promo_image_url: event.promo_image_url.clone(),
        additional_info: event.additional_info.clone(),
        top_line_info: event.top_line_info.clone(),
        age_limit: event.age_limit,
        cancelled_at: event.cancelled_at,
        venue: Some(venue.clone()),
        min_ticket_price: min_ticket_price,
        max_ticket_price: max_ticket_price,
        is_external: event.is_external.clone(),
        external_url: event.external_url.clone(),
        user_is_interested: user
            .map(|u| EventInterest::user_interest(event.id, u.id, connection).unwrap())
            .unwrap_or(false),
        localized_times,
        tracking_keys: TrackingKeys {
            ..Default::default()
        },
        event_type: event.event_type,
    }
}
