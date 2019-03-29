use bigneon_db::dev::TestProject;
use bigneon_db::models::*;
use chrono::prelude::*;
use chrono::Duration;
use uuid::Uuid;

#[test]
fn create() {
    let project = TestProject::new();
    let venue = project.create_venue().finish();
    let user = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .finish();
    let event = project
        .create_event()
        .with_name("NewEvent".into())
        .with_organization(&organization)
        .with_venue(&venue)
        .finish();

    assert_eq!(event.venue_id, Some(venue.id));
    assert_eq!(event.organization_id, organization.id);
    assert_eq!(event.id.to_string().is_empty(), false);
}

#[test]
fn update() {
    //create event
    let project = TestProject::new();
    let venue = project.create_venue().finish();

    let user = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .finish();
    let event = project
        .create_event()
        .with_name("NewEvent".into())
        .with_organization(&organization)
        .with_venue(&venue)
        .finish();
    //Edit event
    let parameters = EventEditableAttributes {
        door_time: Some(NaiveDate::from_ymd(2016, 7, 8).and_hms(4, 10, 11)),
        ..Default::default()
    };
    let event = event
        .update(None, parameters, project.get_connection())
        .unwrap();
    assert_eq!(
        event.door_time,
        Some(NaiveDate::from_ymd(2016, 7, 8).and_hms(4, 10, 11))
    );
}

#[test]
fn guest_list() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let creator = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_fee_schedule(&project.create_fee_schedule().finish(creator.id))
        .finish();
    let venue = project.create_venue().finish();
    let event = project
        .create_event()
        .with_venue(&venue)
        .with_organization(&organization)
        .with_ticket_pricing()
        .finish();
    let user = project.create_user().finish();
    let user2 = project.create_user().finish();
    let user3 = project.create_user().finish();
    let user4 = project.create_user().finish();

    // 1 normal order, 2 orders made on behalf of users by box office user 2
    project
        .create_order()
        .for_event(&event)
        .for_user(&user)
        .quantity(1)
        .is_paid()
        .finish();
    project
        .create_order()
        .for_event(&event)
        .for_user(&user2)
        .on_behalf_of_user(&user3)
        .quantity(1)
        .is_paid()
        .finish();
    project
        .create_order()
        .for_event(&event)
        .for_user(&user2)
        .on_behalf_of_user(&user4)
        .quantity(1)
        .is_paid()
        .finish();
    let guest_list = event.guest_list("", connection).unwrap();
    assert_eq!(3, guest_list.len());
    let guest_ids = guest_list
        .iter()
        .map(|r| r.ticket.user_id)
        .collect::<Vec<Option<Uuid>>>();
    assert!(guest_ids.contains(&Some(user.id)));
    assert!(!guest_ids.contains(&Some(user2.id)));
    assert!(guest_ids.contains(&Some(user3.id)));
    assert!(guest_ids.contains(&Some(user4.id)));

    // User 2 (the box office user) purchases a ticket for themselves
    project
        .create_order()
        .for_event(&event)
        .for_user(&user2)
        .quantity(1)
        .is_paid()
        .finish();
    let guest_list = event.guest_list("", connection).unwrap();
    assert_eq!(4, guest_list.len());
    let guest_ids = guest_list
        .iter()
        .map(|r| r.ticket.user_id)
        .collect::<Vec<Option<Uuid>>>();
    assert!(guest_ids.contains(&Some(user.id)));
    assert!(guest_ids.contains(&Some(user2.id)));
    assert!(guest_ids.contains(&Some(user3.id)));
    assert!(guest_ids.contains(&Some(user4.id)));
}

#[test]
fn publish() {
    //create event
    let project = TestProject::new();
    let venue = project.create_venue().finish();

    let user = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .finish();
    let event = project
        .create_event()
        .with_status(EventStatus::Draft)
        .with_name("NewEvent".into())
        .with_organization(&organization)
        .with_venue(&venue)
        .finish();

    assert_eq!(event.status, EventStatus::Draft);

    let event = event.publish(None, project.get_connection()).unwrap();

    assert_eq!(event.status, EventStatus::Published);
    assert!(event.publish_date.is_some());
}

#[test]
fn publish_in_future() {
    //create event
    let project = TestProject::new();
    let venue = project.create_venue().finish();

    let user = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .finish();
    let event = project
        .create_event()
        .with_status(EventStatus::Draft)
        .with_name("NewEvent".into())
        .with_organization(&organization)
        .with_venue(&venue)
        .finish();

    assert_eq!(event.status, EventStatus::Draft);

    let parameters = EventEditableAttributes {
        publish_date: Some(Some(NaiveDate::from_ymd(2054, 7, 8).and_hms(4, 10, 11))),
        ..Default::default()
    };
    let event = event
        .update(None, parameters, project.get_connection())
        .unwrap();

    let event = event.publish(None, project.get_connection()).unwrap();

    assert_eq!(event.status, EventStatus::Published);
    assert_eq!(
        event.publish_date,
        Some(NaiveDate::from_ymd(2054, 7, 8).and_hms(4, 10, 11))
    );
}

#[test]
fn publish_change_publish_date() {
    //create event
    let project = TestProject::new();
    let venue = project.create_venue().finish();
    let now = Utc::now().naive_utc();

    let user = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .finish();
    let event = project
        .create_event()
        .with_status(EventStatus::Draft)
        .with_name("NewEvent".into())
        .with_organization(&organization)
        .with_venue(&venue)
        .finish();

    assert_eq!(event.status, EventStatus::Draft);

    let parameters = EventEditableAttributes {
        publish_date: Some(Some(NaiveDate::from_ymd(2054, 7, 8).and_hms(4, 10, 11))),
        ..Default::default()
    };
    let event = event
        .update(None, parameters, project.get_connection())
        .unwrap();

    let event = event.publish(None, project.get_connection()).unwrap();

    assert_eq!(event.status, EventStatus::Published);

    let parameters = EventEditableAttributes {
        publish_date: Some(Some(NaiveDate::from_ymd(2041, 7, 8).and_hms(4, 10, 11))),
        ..Default::default()
    };

    let event = event
        .update(None, parameters, project.get_connection())
        .unwrap();

    assert_eq!(
        event.publish_date,
        Some(NaiveDate::from_ymd(2041, 7, 8).and_hms(4, 10, 11))
    );

    let parameters = EventEditableAttributes {
        publish_date: Some(None),
        ..Default::default()
    };

    let event = event
        .update(None, parameters, project.get_connection())
        .unwrap();

    assert!(event.publish_date.unwrap() > now);

    assert!(event.publish_date.unwrap() < Utc::now().naive_utc());
}

#[test]
fn unpublish() {
    //create event
    let project = TestProject::new();
    let venue = project.create_venue().finish();

    let user = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .finish();
    let event = project
        .create_event()
        .with_status(EventStatus::Draft)
        .with_name("NewEvent".into())
        .with_organization(&organization)
        .with_venue(&venue)
        .finish();

    assert_eq!(event.status, EventStatus::Draft);

    let event = event.publish(None, project.get_connection()).unwrap();

    assert_eq!(event.status, EventStatus::Published);
    assert!(event.publish_date.is_some());

    let event = event.unpublish(None, project.get_connection()).unwrap();
    assert_eq!(event.status, EventStatus::Draft);
    assert!(event.publish_date.is_none());
}

#[test]
fn cannot_unpublish_unpublished_event() {
    //create event
    let project = TestProject::new();
    let venue = project.create_venue().finish();

    let user = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .finish();
    let event = project
        .create_event()
        .with_status(EventStatus::Draft)
        .with_name("NewEvent".into())
        .with_organization(&organization)
        .with_venue(&venue)
        .finish();

    assert_eq!(event.status, EventStatus::Draft);
    assert!(event.unpublish(None, project.get_connection()).is_err());
}

#[test]
fn cancel() {
    //create event
    let project = TestProject::new();
    let venue = project.create_venue().finish();

    let user = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .finish();
    let event = project
        .create_event()
        .with_name("NewEvent".into())
        .with_organization(&organization)
        .with_venue(&venue)
        .finish();

    let event = event.cancel(None, &project.get_connection()).unwrap();
    assert!(!event.cancelled_at.is_none());
}

#[test]
fn get_sales_by_date_range() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let creator = project.create_user().finish();

    let organization = project
        .create_organization()
        .with_fee_schedule(&project.create_fee_schedule().finish(creator.id))
        .finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_event_start(NaiveDate::from_ymd(2016, 7, 8).and_hms(9, 10, 11))
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];

    let user = project.create_user().finish();
    let user2 = project.create_user().finish();

    // user purchases 10 tickets
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    cart.update_quantities(
        user.id,
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
    cart.add_external_payment(
        Some("test".to_string()),
        ExternalPaymentType::CreditCard,
        user.id,
        1700,
        connection,
    )
    .unwrap();
    assert_eq!(cart.status, OrderStatus::Paid);

    // Other user does not checkout
    let mut cart2 = Order::find_or_create_cart(&user2, connection).unwrap();
    cart2
        .update_quantities(
            user2.id,
            &[UpdateOrderItem {
                ticket_type_id: ticket_type.id,
                quantity: 5,
                redemption_code: None,
            }],
            false,
            false,
            connection,
        )
        .unwrap();

    // A day ago to today
    let start_utc = Utc::now().naive_utc().date() - Duration::days(1);
    let end_utc = Utc::now().naive_utc().date();
    let results = event
        .get_sales_by_date_range(start_utc, end_utc, connection)
        .unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(
        results,
        vec![
            DayStats {
                date: start_utc,
                revenue_in_cents: 0,
                ticket_sales: 0,
            },
            DayStats {
                date: end_utc,
                revenue_in_cents: 1500,
                ticket_sales: 10,
            }
        ]
    );

    // Just today
    let start_utc = Utc::now().naive_utc().date();
    let end_utc = Utc::now().naive_utc().date();
    let results = event
        .get_sales_by_date_range(start_utc, end_utc, connection)
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(
        results,
        vec![DayStats {
            date: start_utc,
            revenue_in_cents: 1500,
            ticket_sales: 10,
        }]
    );
    // Two days ago to yesterday
    let start_utc = Utc::now().naive_utc().date() - Duration::days(2);
    let end_utc = Utc::now().naive_utc().date() - Duration::days(1);
    let results = event
        .get_sales_by_date_range(start_utc, end_utc, connection)
        .unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(
        results,
        vec![
            DayStats {
                date: start_utc,
                revenue_in_cents: 0,
                ticket_sales: 0,
            },
            DayStats {
                date: end_utc,
                revenue_in_cents: 0,
                ticket_sales: 0,
            }
        ]
    );

    // Error as start date is not earlier than end date
    let results = event.get_sales_by_date_range(end_utc, start_utc, connection);
    assert!(results.is_err());
    assert_eq!(
        "Sales data start date must come before end date",
        results.unwrap_err().cause.unwrap().to_string()
    );
}

#[test]
fn find_by_ids() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project.create_event().with_name("Event1".into()).finish();
    let event2 = project.create_event().with_name("Event2".into()).finish();

    assert_eq!(
        vec![event.clone()],
        Event::find_by_ids(vec![event.id], connection).unwrap()
    );
    assert_eq!(
        vec![event2.clone()],
        Event::find_by_ids(vec![event2.id], connection).unwrap()
    );
    assert_equiv!(
        Event::find_by_ids(vec![event.id, event2.id], connection).unwrap(),
        vec![event, event2]
    );
}

#[test]
fn find_by_order_item_ids() {
    let project = TestProject::new();
    let creator = project.create_user().finish();
    let connection = project.get_connection();
    let organization = project
        .create_organization()
        .with_name("Organization1".into())
        .with_fee_schedule(&project.create_fee_schedule().finish(creator.id))
        .finish();
    let organization2 = project
        .create_organization()
        .with_name("Organization2".into())
        .with_fee_schedule(&project.create_fee_schedule().finish(creator.id))
        .finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let event2 = project
        .create_event()
        .with_organization(&organization2)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let user = project.create_user().finish();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
    let ticket_type2 = &event2.ticket_types(true, None, connection).unwrap()[0];
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    cart.update_quantities(
        user.id,
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
    let total = cart.calculate_total(connection).unwrap();
    cart.add_external_payment(
        Some("Test".to_string()),
        ExternalPaymentType::CreditCard,
        user.id,
        total,
        connection,
    )
    .unwrap();

    let mut cart2 = Order::find_or_create_cart(&user, connection).unwrap();
    cart2
        .update_quantities(
            user.id,
            &[UpdateOrderItem {
                ticket_type_id: ticket_type2.id,
                quantity: 10,
                redemption_code: None,
            }],
            false,
            false,
            connection,
        )
        .unwrap();

    let items = cart.items(&connection).unwrap();
    let items2 = cart2.items(&connection).unwrap();
    let order_item = items
        .iter()
        .find(|i| i.ticket_type_id == Some(ticket_type.id))
        .unwrap();
    let order_item2 = items2
        .iter()
        .find(|i| i.ticket_type_id == Some(ticket_type2.id))
        .unwrap();

    // Ticket belonging to only first event / organization
    let events = Event::find_by_order_item_ids(&vec![order_item.id], connection).unwrap();
    assert_eq!(events, vec![event.clone()]);

    // Ticket belonging to only second event / organization
    let events = Event::find_by_order_item_ids(&vec![order_item2.id], connection).unwrap();
    assert_eq!(events, vec![event2.clone()]);

    // Ticket belonging to both events
    let events =
        Event::find_by_order_item_ids(&vec![order_item.id, order_item2.id], connection).unwrap();
    assert_equiv!(events, vec![event, event2]);
}

#[test]
fn find_individuals() {
    let project = TestProject::new();
    let venue = project.create_venue().finish();
    let event = project
        .create_event()
        .with_name("NewEvent".into())
        .with_event_start(
            NaiveDateTime::parse_from_str("2014-03-05 12:00:00.000", "%Y-%m-%d %H:%M:%S%.f")
                .unwrap(),
        )
        .with_event_end(
            NaiveDateTime::parse_from_str("2014-03-06 12:00:00.000", "%Y-%m-%d %H:%M:%S%.f")
                .unwrap(),
        )
        .with_venue(&venue)
        .finish();

    let parameters = EventEditableAttributes {
        door_time: Some(NaiveDate::from_ymd(2016, 7, 8).and_hms(4, 10, 11)),
        ..Default::default()
    };
    let event = event
        .update(None, parameters, project.get_connection())
        .unwrap();

    let found_event = Event::find(event.id, project.get_connection()).unwrap();
    assert_eq!(found_event, event);

    //find event via venue
    let found_event_via_venue =
        Event::find_all_active_events_for_venue(&event.venue_id.unwrap(), project.get_connection())
            .unwrap();
    assert_eq!(found_event_via_venue[0], event);
}

#[test]
fn find_all_events_for_organization() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().finish();
    let past_event = project
        .create_event()
        .with_name("Event1".into())
        .with_event_start(
            NaiveDateTime::parse_from_str("2014-03-05 12:00:00.000", "%Y-%m-%d %H:%M:%S%.f")
                .unwrap(),
        )
        .with_event_end(
            NaiveDateTime::parse_from_str("2014-03-06 12:00:00.000", "%Y-%m-%d %H:%M:%S%.f")
                .unwrap(),
        )
        .with_organization(&organization)
        .finish();
    let current_event = project
        .create_event()
        .with_name("Event2".into())
        .with_event_start(
            NaiveDateTime::parse_from_str("2018-03-05 12:00:00.000", "%Y-%m-%d %H:%M:%S%.f")
                .unwrap(),
        )
        .with_event_end(
            NaiveDateTime::parse_from_str("2814-03-06 12:00:00.000", "%Y-%m-%d %H:%M:%S%.f")
                .unwrap(),
        )
        .with_organization(&organization)
        .finish();
    let future_event = project
        .create_event()
        .with_name("Event3".into())
        .with_event_start(
            NaiveDateTime::parse_from_str("2918-03-05 12:00:00.000", "%Y-%m-%d %H:%M:%S%.f")
                .unwrap(),
        )
        .with_event_end(
            NaiveDateTime::parse_from_str("2919-03-06 12:00:00.000", "%Y-%m-%d %H:%M:%S%.f")
                .unwrap(),
        )
        .with_organization(&organization)
        .finish();

    // Past events
    let events = Event::find_all_events_for_organization(
        organization.id,
        PastOrUpcoming::Past,
        None,
        0,
        100,
        connection,
    )
    .unwrap();
    assert_eq!(events.data, vec![past_event.summary(connection).unwrap()]);
    assert_eq!(events.paging.total, 1);

    // Upcoming (current, future) events
    let events = Event::find_all_events_for_organization(
        organization.id,
        PastOrUpcoming::Upcoming,
        None,
        0,
        100,
        connection,
    )
    .unwrap();
    assert_eq!(
        events.data,
        vec![
            current_event.summary(connection).unwrap(),
            future_event.summary(connection).unwrap()
        ]
    );
    assert_eq!(events.paging.total, 2);
}

#[test]
fn search() {
    //create event
    let project = TestProject::new();
    let region1 = project.create_region().finish();
    let region2 = project.create_region().finish();
    let venue1 = project
        .create_venue()
        .with_name("Venue1".into())
        .with_region(&region1)
        .finish();
    let venue2 = project
        .create_venue()
        .with_name("Venue2".into())
        .with_region(&region2)
        .finish();
    let artist1 = project.create_artist().with_name("Artist1".into()).finish();
    let artist2 = project.create_artist().with_name("Artist2".into()).finish();
    let organization_owner = project.create_user().finish();
    let organization_user = project.create_user().finish();
    let user = project.create_user().finish();
    let admin = project
        .create_user()
        .finish()
        .add_role(Roles::Admin, project.get_connection())
        .unwrap();
    let organization = project
        .create_organization()
        .with_member(&organization_owner, Roles::OrgOwner)
        .with_member(&organization_user, Roles::OrgMember)
        .finish();
    let event = project
        .create_event()
        .with_status(EventStatus::Published)
        .with_name("OldEvent".into())
        .with_organization(&organization)
        .with_venue(&venue1)
        .with_event_start(NaiveDate::from_ymd(2016, 7, 8).and_hms(9, 10, 11))
        .with_event_end(NaiveDate::from_ymd(2016, 7, 9).and_hms(9, 10, 11))
        .with_publish_date(NaiveDate::from_ymd(2016, 7, 8).and_hms(9, 10, 11))
        .finish();

    event
        .add_artist(None, artist1.id, project.get_connection())
        .unwrap();
    event
        .add_artist(None, artist2.id, project.get_connection())
        .unwrap();

    //find more than one event
    let event2 = project
        .create_event()
        .with_status(EventStatus::Closed)
        .with_name("NewEvent".into())
        .with_organization(&organization)
        .with_venue(&venue2)
        .with_event_start(NaiveDate::from_ymd(2017, 7, 8).and_hms(9, 10, 11))
        .with_event_end(NaiveDate::from_ymd(2017, 7, 9).and_hms(9, 10, 11))
        .finish();

    event2
        .add_artist(None, artist1.id, project.get_connection())
        .unwrap();

    let event3 = project
        .create_event()
        .with_name("NewEvent2".into())
        .with_status(EventStatus::Offline)
        .with_organization(&organization)
        .with_venue(&venue2)
        .with_event_start(NaiveDate::from_ymd(2017, 7, 8).and_hms(9, 10, 11))
        .with_event_end(NaiveDate::from_ymd(2017, 7, 9).and_hms(9, 10, 11))
        .finish();

    // Event draft, not returned except for organization user or owner
    let event4 = project
        .create_event()
        .with_name("NewEventDraft".into())
        .with_status(EventStatus::Draft)
        .with_organization(&organization)
        .with_venue(&venue2)
        .with_event_start(NaiveDate::from_ymd(2017, 7, 8).and_hms(9, 10, 11))
        .with_event_end(NaiveDate::from_ymd(2017, 7, 9).and_hms(9, 10, 11))
        .finish();

    // Event draft belonging to other organization
    let event5 = project
        .create_event()
        .with_name("NewEventDraft2".into())
        .with_status(EventStatus::Draft)
        .with_event_start(NaiveDate::from_ymd(2017, 7, 8).and_hms(9, 10, 11))
        .with_event_end(NaiveDate::from_ymd(2017, 7, 9).and_hms(9, 10, 11))
        .finish();

    // Event with publish date in the future, not returned except for organization user or owner
    let event6 = project
        .create_event()
        .with_name("NewEventFuturePublish".into())
        .with_status(EventStatus::Published)
        .with_event_start(NaiveDate::from_ymd(2017, 7, 8).and_hms(9, 10, 11))
        .with_event_end(NaiveDate::from_ymd(2017, 7, 9).and_hms(9, 10, 11))
        .with_publish_date(NaiveDate::from_ymd(2999, 7, 8).and_hms(9, 10, 11))
        .finish();

    let all_events = vec![event, event2, event3];
    let mut all_events_for_organization = all_events.clone();
    all_events_for_organization.push(event4);
    let mut all_events_for_admin = all_events_for_organization.clone();
    all_events_for_admin.push(event5);
    all_events_for_admin.push(event6);

    // All events unauthorized user
    let all_found_events = Event::search(
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        project.get_connection(),
    )
    .unwrap();
    assert_equiv!(all_events, all_found_events);

    // All events organization owner
    let all_found_events = Event::search(
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        Some(organization_owner),
        PastOrUpcoming::Past,
        project.get_connection(),
    )
    .unwrap();
    assert_equiv!(all_events_for_organization, all_found_events);

    // All events organization user
    let all_found_events = Event::search(
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        Some(organization_user),
        PastOrUpcoming::Past,
        project.get_connection(),
    )
    .unwrap();
    assert_equiv!(all_events_for_organization, all_found_events);

    // All events normal user not part of event organization
    let all_found_events = Event::search(
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        Some(user),
        PastOrUpcoming::Past,
        project.get_connection(),
    )
    .unwrap();
    assert_equiv!(all_events, all_found_events);

    // All events for admin
    let all_found_events = Event::search(
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        Some(admin),
        PastOrUpcoming::Past,
        project.get_connection(),
    )
    .unwrap();
    assert_equiv!(all_events_for_admin, all_found_events);

    // No name specified
    let all_found_events = Event::search(
        Some("".to_string()),
        None,
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        project.get_connection(),
    )
    .unwrap();
    assert_equiv!(all_events, all_found_events);

    // Limited by publicly accessible and specific to an organization
    let all_found_events = Event::search(
        None,
        None,
        Some(organization.id),
        None,
        None,
        None,
        Some(vec![EventStatus::Published]),
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        project.get_connection(),
    )
    .unwrap();
    assert_eq!(all_found_events.len(), 1);
    assert_eq!(all_events[0], all_found_events[0]);

    // Limited by just Published and Offline events
    let all_found_events = Event::search(
        Some("".to_string()),
        None,
        None,
        None,
        None,
        None,
        Some(vec![EventStatus::Published, EventStatus::Offline]),
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        project.get_connection(),
    )
    .unwrap();
    assert_eq!(all_found_events.len(), 2);
    assert_eq!(all_events[0], all_found_events[0]);
    assert_eq!(all_events[2], all_found_events[1]);

    // Limited by just Closed events
    let all_found_events = Event::search(
        Some("".to_string()),
        None,
        None,
        None,
        None,
        None,
        Some(vec![EventStatus::Closed]),
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        project.get_connection(),
    )
    .unwrap();
    assert_eq!(all_found_events.len(), 1);
    assert_eq!(all_events[1], all_found_events[0]);

    // Event name search
    let all_found_events = Event::search(
        Some("New".to_string()),
        None,
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        project.get_connection(),
    )
    .unwrap();
    assert_eq!(all_found_events.len(), 2);
    assert_eq!(all_events[1], all_found_events[0]);
    assert_eq!(all_events[2], all_found_events[1]);

    // Venue name search
    let all_found_events = Event::search(
        Some("Venue1".to_string()),
        None,
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        project.get_connection(),
    )
    .unwrap();
    assert_eq!(all_found_events.len(), 1);
    assert_eq!(all_events[0], all_found_events[0]);

    // Venue id  search
    let all_found_events = Event::search(
        None,
        None,
        None,
        Some(venue1.id),
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        project.get_connection(),
    )
    .unwrap();
    assert_eq!(all_found_events.len(), 1);
    assert_eq!(all_events[0], all_found_events[0]);

    // Artist name search for artist in both events
    let all_found_events = Event::search(
        Some("Artist1".to_string()),
        None,
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        project.get_connection(),
    )
    .unwrap();
    assert_eq!(all_found_events.len(), 2);
    assert_eq!(all_events[0], all_found_events[0]);
    assert_eq!(all_events[1], all_found_events[1]);

    // Artist name search for artist at only one event
    let all_found_events = Event::search(
        Some("Artist2".to_string()),
        None,
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        project.get_connection(),
    )
    .unwrap();
    assert_eq!(all_found_events.len(), 1);
    assert_eq!(all_events[0], all_found_events[0]);

    // Match names Venue2 and Artist2 returning all events
    let all_found_events = Event::search(
        Some("2".to_string()),
        None,
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        project.get_connection(),
    )
    .unwrap();
    assert_equiv!(all_events, all_found_events);

    // Match events belonging to given region
    let all_found_events = Event::search(
        None,
        Some(region1.id.into()),
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        project.get_connection(),
    )
    .unwrap();
    assert_eq!(all_found_events.len(), 1);
    assert_eq!(all_events[0], all_found_events[0]);

    // Match events belonging to other region
    let all_found_events = Event::search(
        None,
        Some(region2.id.into()),
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        project.get_connection(),
    )
    .unwrap();
    assert_eq!(all_found_events.len(), 2);
    assert_eq!(all_events[1], all_found_events[0]);
    assert_eq!(all_events[2], all_found_events[1]);

    // Combination of query and region resulting in no records
    let all_found_events = Event::search(
        Some("Artist2".to_string()),
        Some(region2.id.into()),
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        project.get_connection(),
    )
    .unwrap();
    assert_eq!(all_found_events.len(), 0);

    // Combination of query and region resulting in records
    let all_found_events = Event::search(
        Some("Artist2".to_string()),
        Some(region1.id.into()),
        None,
        None,
        None,
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        project.get_connection(),
    )
    .unwrap();
    assert_eq!(all_found_events.len(), 1);
    assert_eq!(all_events[0], all_found_events[0]);

    let all_found_events = Event::search(
        None,
        None,
        None,
        None,
        Some(NaiveDate::from_ymd(2017, 7, 8).and_hms(9, 0, 11)),
        None,
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        project.get_connection(),
    )
    .unwrap();
    assert_eq!(all_found_events.len(), 2);
    assert_eq!(all_events[1], all_found_events[0]);
    assert_eq!(all_events[2], all_found_events[1]);

    let all_found_events = Event::search(
        None,
        None,
        None,
        None,
        None,
        Some(NaiveDate::from_ymd(2017, 7, 8).and_hms(9, 0, 11)),
        None,
        EventSearchSortField::EventStart,
        SortingDir::Asc,
        None,
        PastOrUpcoming::Past,
        project.get_connection(),
    )
    .unwrap();
    assert_eq!(all_found_events.len(), 1);
    assert_eq!(all_events[0], all_found_events[0]);
}

#[test]
fn current_ticket_pricing_range() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project
        .create_event()
        .with_tickets()
        .with_ticket_type_count(2)
        .finish();

    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
    let ticket_type2 = &event.ticket_types(true, None, connection).unwrap()[1];

    // No current pricing set
    let (min_ticket_price, max_ticket_price) = event
        .current_ticket_pricing_range(false, connection)
        .unwrap();
    assert_eq!(None, min_ticket_price);
    assert_eq!(None, max_ticket_price);

    // Future pricing
    ticket_type
        .add_ticket_pricing(
            "Pricing1".into(),
            NaiveDate::from_ymd(2055, 7, 8).and_hms(7, 8, 10),
            NaiveDate::from_ymd(9999, 7, 8).and_hms(7, 8, 10),
            3000,
            false,
            None,
            connection,
        )
        .unwrap();

    let (min_ticket_price, max_ticket_price) = event
        .current_ticket_pricing_range(false, connection)
        .unwrap();
    assert_eq!(None, min_ticket_price);
    assert_eq!(None, max_ticket_price);

    // Current pricing
    ticket_type
        .add_ticket_pricing(
            "Pricing2".into(),
            NaiveDate::from_ymd(2016, 7, 8).and_hms(7, 8, 10),
            NaiveDate::from_ymd(9999, 7, 8).and_hms(7, 8, 10),
            8000,
            false,
            None,
            connection,
        )
        .unwrap();

    let (min_ticket_price, max_ticket_price) = event
        .current_ticket_pricing_range(false, connection)
        .unwrap();
    assert_eq!(Some(8000), min_ticket_price);
    assert_eq!(Some(8000), max_ticket_price);

    ticket_type2
        .add_ticket_pricing(
            "Pricing2".into(),
            NaiveDate::from_ymd(2016, 7, 8).and_hms(7, 8, 10),
            NaiveDate::from_ymd(9999, 7, 8).and_hms(7, 8, 10),
            20000,
            false,
            None,
            connection,
        )
        .unwrap();

    let (min_ticket_price, max_ticket_price) = event
        .current_ticket_pricing_range(false, connection)
        .unwrap();
    assert_eq!(Some(8000), min_ticket_price);
    assert_eq!(Some(20000), max_ticket_price);

    // Box office pricing, present
    ticket_type
        .add_ticket_pricing(
            "Box office1".into(),
            NaiveDate::from_ymd(2016, 7, 8).and_hms(7, 8, 10),
            NaiveDate::from_ymd(9999, 7, 8).and_hms(7, 8, 10),
            5000,
            true,
            None,
            connection,
        )
        .unwrap();

    // Box office pricing, date in future so won't activate
    ticket_type2
        .add_ticket_pricing(
            "Box office2".into(),
            NaiveDate::from_ymd(2055, 7, 8).and_hms(7, 8, 10),
            NaiveDate::from_ymd(9999, 7, 8).and_hms(7, 8, 10),
            1000,
            true,
            None,
            connection,
        )
        .unwrap();

    let (min_ticket_price, max_ticket_price) = event
        .current_ticket_pricing_range(true, connection)
        .unwrap();
    assert_eq!(Some(5000), min_ticket_price);
    assert_eq!(Some(20000), max_ticket_price);
}

#[test]
fn find_for_organization() {
    //create event
    let project = TestProject::new();
    let venue1 = project.create_venue().finish();
    let venue2 = project.create_venue().finish();
    let user = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .finish();

    let artist1 = project.create_artist().finish();
    let artist2 = project.create_artist().finish();

    let event = project
        .create_event()
        .with_name("Event1".into())
        .with_event_start(
            NaiveDateTime::parse_from_str("2014-03-04 12:00:00.000", "%Y-%m-%d %H:%M:%S%.f")
                .unwrap(),
        )
        .with_event_end(
            NaiveDateTime::parse_from_str("2014-03-05 12:00:00.000", "%Y-%m-%d %H:%M:%S%.f")
                .unwrap(),
        )
        .with_organization(&organization)
        .with_venue(&venue1)
        .finish();
    event
        .add_artist(None, artist1.id, project.get_connection())
        .unwrap();
    event
        .add_artist(None, artist2.id, project.get_connection())
        .unwrap();

    //find more than one event
    let event2 = project
        .create_event()
        .with_name("Event2".into())
        .with_event_start(
            NaiveDateTime::parse_from_str("2014-03-05 12:00:00.000", "%Y-%m-%d %H:%M:%S%.f")
                .unwrap(),
        )
        .with_event_end(
            NaiveDateTime::parse_from_str("2014-03-06 12:00:00.000", "%Y-%m-%d %H:%M:%S%.f")
                .unwrap(),
        )
        .with_organization(&organization)
        .with_venue(&venue2)
        .finish();
    event2
        .add_artist(None, artist1.id, project.get_connection())
        .unwrap();

    let all_events = vec![event2.id, event.id];

    //find all events via organisation
    let found_event_via_organizations = Event::find_all_events_for_organization(
        organization.id,
        PastOrUpcoming::Past,
        None,
        0,
        100,
        project.get_connection(),
    )
    .unwrap();
    assert_equiv!(
        found_event_via_organizations
            .data
            .iter()
            .map(|i| i.id)
            .collect::<Vec<Uuid>>(),
        all_events
    );

    // Restrict to just a subset of events
    let found_event_via_organizations = Event::find_all_events_for_organization(
        organization.id,
        PastOrUpcoming::Past,
        Some(vec![event.id]),
        0,
        100,
        project.get_connection(),
    )
    .unwrap();
    assert_equiv!(
        found_event_via_organizations
            .data
            .iter()
            .map(|i| i.id)
            .collect::<Vec<Uuid>>(),
        vec![event.id]
    );

    // Returning both events
    let found_event_via_organizations = Event::find_all_events_for_organization(
        organization.id,
        PastOrUpcoming::Past,
        Some(vec![event.id, event2.id]),
        0,
        100,
        project.get_connection(),
    )
    .unwrap();
    assert_equiv!(
        found_event_via_organizations
            .data
            .iter()
            .map(|i| i.id)
            .collect::<Vec<Uuid>>(),
        all_events
    );
}

#[test]
fn find_active_for_venue() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let venue = project.create_venue().finish();
    let user = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .finish();

    let artist1 = project.create_artist().finish();
    let artist2 = project.create_artist().finish();
    //create two events
    let event = project
        .create_event()
        .with_name("Event1".into())
        .with_event_start(
            NaiveDateTime::parse_from_str("2014-03-04 12:00:00.000", "%Y-%m-%d %H:%M:%S%.f")
                .unwrap(),
        )
        .with_organization(&organization)
        .with_venue(&venue)
        .finish();
    event
        .add_artist(None, artist1.id, project.get_connection())
        .unwrap();
    event
        .add_artist(None, artist2.id, project.get_connection())
        .unwrap();
    let event2 = project
        .create_event()
        .with_name("Event2".into())
        .with_event_start(
            NaiveDateTime::parse_from_str("2014-03-05 12:00:00.000", "%Y-%m-%d %H:%M:%S%.f")
                .unwrap(),
        )
        .with_organization(&organization)
        .with_venue(&venue)
        .finish();
    event2
        .add_artist(None, artist1.id, project.get_connection())
        .unwrap();
    //Cancel first event
    event.cancel(None, connection).unwrap();

    //find all active events via venue
    let found_events =
        Event::find_all_active_events_for_venue(&venue.id, project.get_connection()).unwrap();

    assert_eq!(found_events.len(), 1);
    assert_eq!(found_events[0].id, event2.id);
}

#[test]
fn organization() {
    let project = TestProject::new();
    let user = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .finish();
    let event = project
        .create_event()
        .with_name("NewEvent".into())
        .with_organization(&organization)
        .finish();

    assert_eq!(
        event.organization(project.get_connection()).unwrap(),
        organization
    );
}

#[test]
fn venue() {
    let project = TestProject::new();
    let venue = project.create_venue().finish();
    let event = project
        .create_event()
        .with_name("NewEvent".into())
        .with_venue(&venue)
        .finish();
    assert_eq!(event.venue(project.get_connection()).unwrap(), Some(venue));

    let event = project.create_event().with_name("NewEvent".into()).finish();
    assert_eq!(event.venue(project.get_connection()).unwrap(), None);
}

#[test]
fn add_ticket_type() {
    let project = TestProject::new();
    let conn = project.get_connection();
    let event = project.create_event().finish();
    let wallet_id = event.issuer_wallet(conn).unwrap().id;
    let sd = NaiveDate::from_ymd(2016, 7, 8).and_hms(4, 10, 11);
    let ed = NaiveDate::from_ymd(2016, 7, 9).and_hms(4, 10, 11);
    let ticket_type = event
        .add_ticket_type(
            "General Admission".to_string(),
            None,
            100,
            sd,
            ed,
            wallet_id,
            None,
            0,
            100,
            SoldOutBehavior::ShowSoldOut,
            false,
            conn,
        )
        .unwrap();

    assert_eq!(ticket_type.event_id, event.id);
    assert_eq!(ticket_type.name, "General Admission".to_string());
}

#[test]
fn ticket_types() {
    let project = TestProject::new();
    let conn = project.get_connection();
    let event = project.create_event().finish();
    let wallet_id = event.issuer_wallet(conn).unwrap().id;
    let sd = NaiveDate::from_ymd(2016, 7, 8).and_hms(4, 10, 11);
    let ed = NaiveDate::from_ymd(2016, 7, 9).and_hms(4, 10, 11);
    let ticket_type_ga = event
        .add_ticket_type(
            "General Admission".to_string(),
            None,
            100,
            sd,
            ed,
            wallet_id,
            None,
            0,
            100,
            SoldOutBehavior::ShowSoldOut,
            false,
            conn,
        )
        .unwrap();
    let ticket_type_vip = event
        .add_ticket_type(
            "VIP".to_string(),
            None,
            100,
            sd,
            ed,
            wallet_id,
            None,
            0,
            100,
            SoldOutBehavior::ShowSoldOut,
            false,
            conn,
        )
        .unwrap();

    let ticket_types = event.ticket_types(true, None, conn).unwrap();

    assert_equiv!(ticket_types, vec![ticket_type_ga, ticket_type_vip]);
}

#[test]
fn localized_time() {
    let utc_time =
        NaiveDateTime::parse_from_str("2019-01-01 12:00:00.000", "%Y-%m-%d %H:%M:%S%.f").unwrap();
    let localized_time =
        Event::localized_time(&Some(utc_time), &Some("Africa/Johannesburg".to_string())).unwrap();
    assert_eq!(
        localized_time.to_rfc2822(),
        "Tue,  1 Jan 2019 14:00:00 +0200"
    );

    let invalid_localized_time =
        Event::localized_time(&None, &Some("Africa/Johannesburg".to_string()));
    assert_eq!(invalid_localized_time, None);

    let invalid_localized_time = Event::localized_time(&Some(utc_time), &None);
    assert_eq!(invalid_localized_time, None);

    let invalid_localized_time = Event::localized_time(&None, &None);
    assert_eq!(invalid_localized_time, None);
}

#[test]
fn get_all_localized_times() {
    let project = TestProject::new();
    //    let conn = project.get_connection();
    let venue = project
        .create_venue()
        .with_timezone("Africa/Johannesburg".to_string())
        .finish();
    let utc_time =
        NaiveDateTime::parse_from_str("2019-01-01 12:00:00.000", "%Y-%m-%d %H:%M:%S%.f").unwrap();
    let event = project
        .create_event()
        .with_event_start(utc_time.clone())
        .with_venue(&venue)
        .finish();

    let localized_times: EventLocalizedTimes = event.get_all_localized_times(&Some(venue));
    println!("{}", localized_times.event_start.unwrap().to_rfc2822());
    assert_eq!(
        localized_times.event_start.unwrap().to_rfc2822(),
        "Tue,  1 Jan 2019 14:00:00 +0200"
    );
    assert_eq!(
        localized_times.event_end.unwrap().to_rfc2822(),
        "Wed,  2 Jan 2019 14:00:00 +0200"
    );
    assert_ne!(
        localized_times.door_time.unwrap().to_rfc2822(),
        "Tue,  1 Jan 2019 13:00:00 +0200"
    );
}

#[test]
fn checked_in_users() {
    let project = TestProject::new();
    let admin = project.create_user().finish();

    let connection = project.get_connection();

    let organization = project
        .create_organization()
        .with_fee_schedule(&project.create_fee_schedule().finish(admin.id))
        .finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_ticket_pricing()
        .finish();
    let user = project.create_user().finish();
    project
        .create_order()
        .for_event(&event)
        .for_user(&user)
        .quantity(1)
        .is_paid()
        .finish();
    let ticket = TicketInstance::find_for_user(user.id, connection)
        .unwrap()
        .remove(0);

    let result2 =
        TicketInstance::redeem_ticket(ticket.id, ticket.redeem_key.unwrap(), admin.id, connection)
            .unwrap();
    assert_eq!(result2, RedeemResults::TicketRedeemSuccess);
    let users = Event::checked_in_users(event.id, connection).unwrap();
    assert_eq!(users[0], user);
    let fans = event
        .search_fans(None, Some(100), Some(0), None, None, connection)
        .unwrap();
    assert_eq!(user.id, fans.0[0].user_id);
}
