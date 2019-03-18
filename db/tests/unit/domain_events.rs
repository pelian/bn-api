use bigneon_db::dev::TestProject;
use bigneon_db::prelude::*;
use chrono::Utc;
use serde_json::Value;
use std::str::FromStr;
use uuid::Uuid;

#[test]
fn serialize() {
    let project = TestProject::new();
    let conn = project.get_connection();

    let domain_event = DomainEvent::create(
        DomainEventTypes::EventArtistAdded,
        "Nothing to see here".to_string(),
        Tables::EventArtists,
        None,
        None,
        None,
    )
    .commit(conn)
    .unwrap();

    let json = json!(domain_event).to_string();
    let v: Value = serde_json::from_str(json.as_str()).unwrap();
    assert_eq!(v["event_type"], "EventArtistAdded");
    assert_eq!(v["display_text"], "Nothing to see here");
    assert_eq!(v["main_table"], "EventArtists");
}

#[test]
fn partial_ord() {
    let make_dummy_event = |id| DomainEvent {
        id,
        event_type: DomainEventTypes::EventArtistAdded,
        display_text: "".to_string(),
        event_data: None,
        main_table: Tables::EventArtists,
        main_id: None,
        published_at: None,
        created_at: Utc::now().naive_utc(),
        updated_at: Utc::now().naive_utc(),
        user_id: None,
    };

    let high_id = "e2cf68a4-76bb-49e1-993c-2576a4fc1220";
    let low_id = "e2cf68a4-76bb-49e1-993c-2576a4fc1221";

    // Anti-symmetry
    let a = make_dummy_event(Uuid::from_str(low_id).unwrap());
    let b = make_dummy_event(Uuid::from_str(high_id).unwrap());

    assert!(a > b);
    assert!(!(a < b));

    let a = make_dummy_event(Uuid::from_str(high_id).unwrap());
    let b = make_dummy_event(Uuid::from_str(low_id).unwrap());

    assert!(a < b);
    assert!(!(a > b));
}

#[test]
fn commit() {
    let project = TestProject::new();
    let conn = project.get_connection();

    let domain_event = DomainEvent::create(
        DomainEventTypes::EventArtistAdded,
        "".to_string(),
        Tables::EventArtists,
        None,
        None,
        None,
    );

    let domain_action = domain_event.commit(conn).unwrap();

    assert!(!domain_action.id.is_nil());
}

#[test]
fn find() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let id = Uuid::new_v4();

    // Empty, no events
    assert!(
        DomainEvent::find(Tables::PaymentMethods, Some(id), None, connection)
            .unwrap()
            .is_empty()
    );

    // New events
    let domain_event = DomainEvent::create(
        DomainEventTypes::PaymentMethodCreated,
        "Payment method was created".to_string(),
        Tables::PaymentMethods,
        Some(id),
        Some(user.id),
        Some("".into()),
    )
    .commit(connection)
    .unwrap();

    let domain_event2 = DomainEvent::create(
        DomainEventTypes::PaymentMethodUpdated,
        "Payment method was updated".to_string(),
        Tables::PaymentMethods,
        Some(id),
        Some(user.id),
        Some("".into()),
    )
    .commit(connection)
    .unwrap();

    assert_equiv!(
        DomainEvent::find(Tables::PaymentMethods, Some(id), None, connection).unwrap(),
        [domain_event.clone(), domain_event2.clone()]
    );
    assert!(
        DomainEvent::find(Tables::Payments, Some(id), None, connection)
            .unwrap()
            .is_empty()
    );
    assert!(DomainEvent::find(
        Tables::PaymentMethods,
        Some(Uuid::new_v4()),
        None,
        connection,
    )
    .unwrap()
    .is_empty());

    // Filtered by type
    assert_eq!(
        DomainEvent::find(
            Tables::PaymentMethods,
            Some(id),
            Some(DomainEventTypes::PaymentMethodCreated),
            connection,
        )
        .unwrap(),
        [domain_event]
    );
    assert_eq!(
        DomainEvent::find(
            Tables::PaymentMethods,
            Some(id),
            Some(DomainEventTypes::PaymentMethodUpdated),
            connection,
        )
        .unwrap(),
        [domain_event2]
    );
}

#[test]
pub fn find_unpublished() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let id = Uuid::new_v4();

    let domain_event = DomainEvent::create(
        DomainEventTypes::PaymentMethodCreated,
        "Payment method was created".to_string(),
        Tables::PaymentMethods,
        Some(id),
        Some(user.id),
        Some("".into()),
    )
    .commit(connection)
    .unwrap();

    let mut found_events = DomainEvent::find_unpublished(100, connection).unwrap();

    let db_event = found_events.remove(0);
    assert_eq!(db_event, domain_event);

    let mut publisher = DomainEventPublisher::new();
    publisher.add_subscription(DomainEventTypes::PaymentMethodCreated, |_| None);
    publisher.publish(db_event, connection).unwrap();

    let found_events = DomainEvent::find_unpublished(100, connection).unwrap();
    assert!(found_events.is_empty());
}

#[test]
pub fn mark_as_published() {
    let project = TestProject::new();
    let conn = project.get_connection();

    let user = project.create_user().finish();
    let id = Uuid::new_v4();

    let domain_event = DomainEvent::create(
        DomainEventTypes::PaymentMethodCreated,
        "Payment method was created".to_string(),
        Tables::PaymentMethods,
        Some(id),
        Some(user.id),
        Some("".into()),
    )
    .commit(conn)
    .unwrap();

    domain_event.mark_as_published(conn).unwrap();

    let found_events = DomainEvent::find(Tables::PaymentMethods, Some(id), None, conn).unwrap();

    assert_eq!(1, found_events.len());
    // 60 second leeway
    assert!(
        found_events[0].published_at.unwrap().timestamp() > Utc::now().naive_utc().timestamp() - 60
    );
}
