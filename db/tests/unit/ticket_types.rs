use bigneon_db::dev::TestProject;
use bigneon_db::prelude::*;
use bigneon_db::schema::ticket_instances;
use bigneon_db::utils::errors::ErrorCode::ValidationError;
use chrono::NaiveDate;
use diesel::prelude::*;
use uuid::Uuid;

#[test]
fn create() {
    let project = TestProject::new();
    let conn = project.get_connection();
    let event = project.create_event().finish();
    let wallet_id = event.issuer_wallet(conn).unwrap().id;
    let sd = NaiveDate::from_ymd(2016, 7, 8).and_hms(4, 10, 11);
    let ed = NaiveDate::from_ymd(2016, 7, 9).and_hms(4, 10, 11);
    let ticket_type = event
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
            conn,
        )
        .unwrap();

    assert_eq!(ticket_type.event_id, event.id);
    assert_eq!(ticket_type.name, "VIP".to_string())
}

#[test]
pub fn create_with_validation_errors() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project.create_event().finish();
    let wallet_id = event.issuer_wallet(connection).unwrap().id;
    let start_date = NaiveDate::from_ymd(2016, 7, 8).and_hms(4, 10, 11);
    let end_date = NaiveDate::from_ymd(2016, 6, 9).and_hms(4, 10, 11);
    let result = event.add_ticket_type(
        "VIP".to_string(),
        None,
        100,
        start_date,
        end_date,
        wallet_id,
        None,
        0,
        100,
        connection,
    );
    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("start_date"));
                assert_eq!(errors["start_date"].len(), 1);
                assert_eq!(
                    errors["start_date"][0].code,
                    "start_date_must_be_before_end_date"
                );
                assert_eq!(
                    &errors["start_date"][0]
                        .message
                        .clone()
                        .unwrap()
                        .into_owned(),
                    "Start date must be before end date"
                );
            }
            _ => panic!("Expected validation error"),
        },
    }
}

#[test]
fn valid_unsold_ticket_count() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project.create_event().with_ticket_pricing().finish();
    let ticket_type = event
        .ticket_types(true, None, connection)
        .unwrap()
        .remove(0);
    assert_eq!(
        100,
        ticket_type.valid_unsold_ticket_count(connection).unwrap()
    );

    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    cart.update_quantities(
        user.id,
        &vec![UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 50,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();

    // 50 in cart
    assert_eq!(
        100,
        ticket_type.valid_unsold_ticket_count(connection).unwrap()
    );

    let total = cart.calculate_total(connection).unwrap();
    cart.add_external_payment(Some("test".to_string()), user.id, total, connection)
        .unwrap();

    // 50 paid
    assert_eq!(
        50,
        ticket_type.valid_unsold_ticket_count(connection).unwrap()
    );

    // Add 1 to cart marking it as reserved
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    cart.update_quantities(
        user.id,
        &vec![UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 1,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();

    let items = cart.items(&connection).unwrap();
    let order_item = items
        .iter()
        .find(|i| i.ticket_type_id == Some(ticket_type.id))
        .unwrap();
    let ticket_instance = ticket_instances::table
        .filter(ticket_instances::order_item_id.eq(order_item.id))
        .first::<TicketInstance>(connection)
        .unwrap();
    assert_eq!(TicketInstanceStatus::Reserved, ticket_instance.status);

    // 0 Nullified events
    let domain_events = DomainEvent::find(
        Tables::TicketInstances,
        None,
        Some(DomainEventTypes::TicketInstanceNullified),
        connection,
    )
    .unwrap();
    assert_eq!(0, domain_events.len());

    // Nullify 49
    let asset = Asset::find_by_ticket_type(ticket_type.id, connection).unwrap();
    TicketInstance::nullify_tickets(asset.id, 49, user.id, connection).unwrap();
    assert_eq!(
        1,
        ticket_type.valid_unsold_ticket_count(connection).unwrap()
    );

    // Reload ticket instance, should not nullify
    let ticket_instance = TicketInstance::find(ticket_instance.id, connection).unwrap();
    assert_eq!(TicketInstanceStatus::Reserved, ticket_instance.status);

    // 0 Nullified events for this particular ID
    let domain_events = DomainEvent::find(
        Tables::TicketInstances,
        Some(ticket_instance.id),
        Some(DomainEventTypes::TicketInstanceNullified),
        connection,
    )
    .unwrap();
    assert_eq!(0, domain_events.len());

    // Nullify remaining 1
    TicketInstance::nullify_tickets(asset.id, 1, user.id, connection).unwrap();
    assert_eq!(
        0,
        ticket_type.valid_unsold_ticket_count(connection).unwrap()
    );

    // Reload ticket instance, should nullify
    let ticket_instance = TicketInstance::find(ticket_instance.id, connection).unwrap();
    assert_eq!(TicketInstanceStatus::Nullified, ticket_instance.status);

    // 1 Nullified events for this particular ID
    let domain_events = DomainEvent::find(
        Tables::TicketInstances,
        Some(ticket_instance.id),
        Some(DomainEventTypes::TicketInstanceNullified),
        connection,
    )
    .unwrap();
    assert_eq!(1, domain_events.len());
}

#[test]
fn find_for_code() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project
        .create_event()
        .with_ticket_pricing()
        .with_ticket_type_count(2)
        .finish();
    let ticket_types = event.ticket_types(true, None, &connection).unwrap();
    let ticket_type = &ticket_types[0];
    let ticket_type2 = &ticket_types[1];
    let code = project.create_code().with_event(&event).finish();

    TicketTypeCode::create(ticket_type.id, code.id)
        .commit(connection)
        .unwrap();
    TicketTypeCode::create(ticket_type2.id, code.id)
        .commit(connection)
        .unwrap();

    let found_ticket_types = TicketType::find_for_code(code.id, connection).unwrap();
    assert_eq!(
        found_ticket_types
            .into_iter()
            .map(|tt| tt.id)
            .collect::<Vec<Uuid>>()
            .sort(),
        vec![ticket_type.id, ticket_type2.id].sort()
    );
}

#[test]
fn find_by_event_id() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project
        .create_event()
        .with_ticket_pricing()
        .with_ticket_type_count(2)
        .finish();
    let mut ticket_types = event.ticket_types(true, None, &connection).unwrap();
    let ticket_type = ticket_types.remove(0);
    let ticket_type2 = ticket_types.remove(0);

    // No access codes on file checking with access code filtering
    let results = TicketType::find_by_event_id(event.id, true, None, &connection).unwrap();
    assert_eq!(vec![ticket_type.clone(), ticket_type2.clone()], results);

    // No access codes on file checking without access code filtering
    let mut results = TicketType::find_by_event_id(event.id, false, None, &connection).unwrap();
    results.sort_by_key(|r| r.id);
    let mut expected = vec![ticket_type.clone(), ticket_type2.clone()];
    expected.sort_by_key(|r| r.id);
    assert_eq!(expected, results);

    // Add access code
    let code = project
        .create_code()
        .with_event(&event)
        .for_ticket_type(&ticket_type)
        .with_code_type(CodeTypes::Access)
        .finish();

    // One ticket type is filtered while one is not so filtered only shows when code is present
    let results = TicketType::find_by_event_id(event.id, true, None, &connection).unwrap();
    assert_eq!(vec![ticket_type2.clone()], results);
    let results = TicketType::find_by_event_id(
        event.id,
        true,
        Some(code.redemption_code.clone()),
        &connection,
    )
    .unwrap();
    assert_eq!(vec![ticket_type.clone(), ticket_type2.clone()], results);
    let results = TicketType::find_by_event_id(event.id, false, None, &connection).unwrap();
    assert_eq!(vec![ticket_type.clone(), ticket_type2.clone()], results);

    // Add additional code for discount but it does not affect visibility
    let code2 = project
        .create_code()
        .with_event(&event)
        .for_ticket_type(&ticket_type2)
        .with_code_type(CodeTypes::Discount)
        .finish();

    // Behavior mimics previous logic in that no ticket types have been filtered for new code
    let results = TicketType::find_by_event_id(event.id, true, None, &connection).unwrap();
    assert_eq!(vec![ticket_type2.clone()], results);
    let results = TicketType::find_by_event_id(
        event.id,
        true,
        Some(code.redemption_code.clone()),
        &connection,
    )
    .unwrap();
    assert_eq!(vec![ticket_type.clone(), ticket_type2.clone()], results);
    let results = TicketType::find_by_event_id(
        event.id,
        true,
        Some(code2.redemption_code.clone()),
        &connection,
    )
    .unwrap();
    assert_eq!(vec![ticket_type2.clone()], results);
    let results = TicketType::find_by_event_id(event.id, false, None, &connection).unwrap();
    assert_eq!(vec![ticket_type.clone(), ticket_type2.clone()], results);

    // Add additional access code
    let code3 = project
        .create_code()
        .with_event(&event)
        .for_ticket_type(&ticket_type2)
        .with_code_type(CodeTypes::Access)
        .finish();

    // No ticket types present when filtering but they can show up with each redemption code or without filtering
    let results = TicketType::find_by_event_id(event.id, true, None, &connection).unwrap();
    assert!(results.is_empty());
    let results = TicketType::find_by_event_id(
        event.id,
        true,
        Some(code.redemption_code.clone()),
        &connection,
    )
    .unwrap();
    assert_eq!(vec![ticket_type.clone()], results);
    let results = TicketType::find_by_event_id(
        event.id,
        true,
        Some(code2.redemption_code.clone()),
        &connection,
    )
    .unwrap();
    assert!(results.is_empty());
    let results = TicketType::find_by_event_id(
        event.id,
        true,
        Some(code3.redemption_code.clone()),
        &connection,
    )
    .unwrap();
    assert_eq!(vec![ticket_type2.clone()], results);
    let results = TicketType::find_by_event_id(event.id, false, None, &connection).unwrap();
    assert_eq!(vec![ticket_type.clone(), ticket_type2.clone()], results);
}

#[test]
fn create_large_amount() {
    let project = TestProject::new();
    let conn = project.get_connection();
    let event = project.create_event().finish();
    let wallet_id = event.issuer_wallet(conn).unwrap().id;
    let sd = NaiveDate::from_ymd(2016, 7, 8).and_hms(4, 10, 11);
    let ed = NaiveDate::from_ymd(2016, 7, 9).and_hms(4, 10, 11);
    let ticket_type = event
        .add_ticket_type(
            "VIP".to_string(),
            None,
            100_000,
            sd,
            ed,
            wallet_id,
            None,
            0,
            100,
            conn,
        )
        .unwrap();

    assert_eq!(ticket_type.event_id, event.id);
    assert_eq!(ticket_type.name, "VIP".to_string())
}

#[test]
pub fn create_with_same_date_validation_errors() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project.create_event().finish();
    let wallet_id = event.issuer_wallet(connection).unwrap().id;
    let same_date = NaiveDate::from_ymd(2016, 7, 8).and_hms(4, 10, 11);
    let result = event.add_ticket_type(
        "VIP".to_string(),
        None,
        100,
        same_date,
        same_date,
        wallet_id,
        None,
        0,
        100,
        connection,
    );
    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("start_date"));
                assert_eq!(errors["start_date"].len(), 1);
                assert_eq!(
                    errors["start_date"][0].code,
                    "start_date_must_be_before_end_date"
                );
                assert_eq!(
                    &errors["start_date"][0]
                        .message
                        .clone()
                        .unwrap()
                        .into_owned(),
                    "Start date must be before end date"
                );
            }
            _ => panic!("Expected validation error"),
        },
    }
}

#[test]
fn validate_ticket_pricing() {
    let project = TestProject::new();
    let event = project.create_event().with_tickets().finish();
    let ticket_type = &event
        .ticket_types(true, None, project.get_connection())
        .unwrap()[0];

    // Set short window for validations to detect dates outside of ticket type window
    let ticket_type = ticket_type
        .update(
            TicketTypeEditableAttributes {
                start_date: Some(NaiveDate::from_ymd(2016, 7, 8).and_hms(4, 10, 11)),
                end_date: Some(NaiveDate::from_ymd(2016, 7, 9).and_hms(4, 10, 11)),
                ..Default::default()
            },
            project.get_connection(),
        )
        .unwrap();

    let start_date1 = NaiveDate::from_ymd(2016, 7, 6).and_hms(4, 10, 11);
    let end_date1 = NaiveDate::from_ymd(2016, 7, 10).and_hms(4, 10, 11);
    let start_date2 = NaiveDate::from_ymd(2016, 7, 7).and_hms(4, 10, 11);
    let end_date2 = NaiveDate::from_ymd(2016, 7, 11).and_hms(4, 10, 11);
    TicketPricing::create(
        ticket_type.id,
        "Early Bird".to_string(),
        start_date1,
        end_date1,
        100,
        false,
        None,
    )
    .commit(project.get_connection())
    .unwrap();
    let ticket_pricing = TicketPricing::create(
        ticket_type.id,
        "Regular".to_string(),
        start_date2,
        end_date2,
        100,
        false,
        None,
    )
    .commit(project.get_connection())
    .unwrap();
    let mut ticket_pricing_parameters: TicketPricingEditableAttributes = Default::default();

    // Overlapping period and overlapping ticket type window
    let validation_results = ticket_type.validate_ticket_pricing(project.get_connection());
    assert!(validation_results.is_err());
    let error = validation_results.unwrap_err();
    match &error.error_code {
        ErrorCode::ValidationError { errors } => {
            assert!(errors.contains_key("ticket_pricing"));
            assert_eq!(errors["ticket_pricing"].len(), 2);
            assert_eq!(
                errors["ticket_pricing"][0].code,
                "ticket_pricing_overlapping_periods"
            );
            assert_eq!(
                &errors["ticket_pricing"][0]
                    .message
                    .clone()
                    .unwrap()
                    .into_owned(),
                "Ticket pricing dates overlap another ticket pricing period"
            );

            assert!(errors.contains_key("ticket_pricing.start_date"));
            assert_eq!(errors["ticket_pricing.start_date"].len(), 2);
            assert_eq!(
                errors["ticket_pricing.start_date"][0].code,
                "ticket_pricing_overlapping_ticket_type_start_date"
            );
            assert_eq!(
                &errors["ticket_pricing.start_date"][0]
                    .message
                    .clone()
                    .unwrap()
                    .into_owned(),
                "Ticket pricing dates overlap ticket type start date"
            );

            assert!(errors.contains_key("ticket_pricing.end_date"));
            assert_eq!(errors["ticket_pricing.end_date"].len(), 2);
            assert_eq!(
                errors["ticket_pricing.end_date"][0].code,
                "ticket_pricing_overlapping_ticket_type_end_date"
            );
            assert_eq!(
                &errors["ticket_pricing.end_date"][0]
                    .message
                    .clone()
                    .unwrap()
                    .into_owned(),
                "Ticket pricing dates overlap ticket type end date"
            );
        }
        _ => panic!("Expected validation error"),
    }

    // Ticket type adjusted so ticket pricing inclusive of its dates
    let ticket_type = ticket_type
        .update(
            TicketTypeEditableAttributes {
                start_date: Some(NaiveDate::from_ymd(2016, 6, 1).and_hms(4, 10, 11)),
                end_date: Some(NaiveDate::from_ymd(2055, 7, 6).and_hms(4, 10, 11)),
                ..Default::default()
            },
            project.get_connection(),
        )
        .unwrap();

    // Period adjusted to not overlap (after existing record)
    ticket_pricing_parameters.start_date = Some(end_date1);
    ticket_pricing_parameters.end_date = Some(NaiveDate::from_ymd(2016, 7, 15).and_hms(4, 10, 11));
    ticket_pricing
        .update(ticket_pricing_parameters.clone(), project.get_connection())
        .unwrap();
    ticket_type
        .validate_ticket_pricing(project.get_connection())
        .unwrap();

    // Period adjusted to not overlap (prior to existing record)
    ticket_pricing_parameters.start_date = Some(NaiveDate::from_ymd(2016, 7, 4).and_hms(4, 10, 11));
    ticket_pricing_parameters.end_date = Some(start_date1);
    ticket_pricing
        .update(ticket_pricing_parameters.clone(), project.get_connection())
        .unwrap();

    ticket_type
        .validate_ticket_pricing(project.get_connection())
        .unwrap();
}

#[test]
pub fn remaining_ticket_count() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project.create_event().with_ticket_pricing().finish();
    let ticket_type = event
        .ticket_types(true, None, connection)
        .unwrap()
        .remove(0);
    let mut order = project.create_order().for_event(&event).finish();
    let user_id = order.user_id;
    assert_eq!(
        90,
        ticket_type
            .valid_available_ticket_count(connection)
            .unwrap()
    );

    order
        .update_quantities(
            user_id,
            &[UpdateOrderItem {
                ticket_type_id: ticket_type.id,
                quantity: 20,
                redemption_code: None,
            }],
            false,
            false,
            connection,
        )
        .unwrap();
    assert_eq!(
        80,
        ticket_type
            .valid_available_ticket_count(connection)
            .unwrap()
    );

    order
        .update_quantities(
            user_id,
            &[UpdateOrderItem {
                ticket_type_id: ticket_type.id,
                quantity: 16,
                redemption_code: None,
            }],
            false,
            false,
            connection,
        )
        .unwrap();
    assert_eq!(
        84,
        ticket_type
            .valid_available_ticket_count(connection)
            .unwrap()
    );
}

#[test]
fn update() {
    let db = TestProject::new();
    let connection = db.get_connection();
    let event = db.create_event().with_tickets().finish();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
    //Change editable parameter and submit ticket type update request
    let update_name = String::from("updated_event_name");
    let update_start_date = NaiveDate::from_ymd(2018, 4, 23).and_hms(5, 14, 18);
    let update_end_date = NaiveDate::from_ymd(2018, 6, 1).and_hms(8, 5, 34);
    let update_parameters = TicketTypeEditableAttributes {
        name: Some(update_name.clone()),
        start_date: Some(update_start_date),
        end_date: Some(update_end_date),
        ..Default::default()
    };
    let updated_ticket_type = ticket_type.update(update_parameters, connection).unwrap();
    assert_eq!(updated_ticket_type.id, ticket_type.id);
    assert_eq!(updated_ticket_type.name, update_name);
    assert_eq!(updated_ticket_type.start_date, update_start_date);
    assert_eq!(updated_ticket_type.end_date, update_end_date);
}

#[test]
fn cancel() {
    let db = TestProject::new();
    let connection = db.get_connection();
    let event = db.create_event().with_tickets().finish();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];

    let cancelled_ticket_type = ticket_type.cancel(connection).unwrap();

    assert_eq!(cancelled_ticket_type.status, TicketTypeStatus::Cancelled);
}

#[test]
pub fn update_with_validation_errors() {
    let db = TestProject::new();
    let connection = db.get_connection();
    let event = db.create_event().with_tickets().finish();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
    //Change editable parameter and submit ticket type update request
    let update_name = String::from("updated_event_name");
    let update_start_date = NaiveDate::from_ymd(2018, 6, 23).and_hms(5, 14, 18);
    let update_end_date = NaiveDate::from_ymd(2018, 4, 1).and_hms(8, 5, 34);
    let update_parameters = TicketTypeEditableAttributes {
        name: Some(update_name.clone()),
        start_date: Some(update_start_date),
        end_date: Some(update_end_date),
        ..Default::default()
    };
    let result = ticket_type.update(update_parameters, connection);
    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("start_date"));
                assert_eq!(errors["start_date"].len(), 1);
                assert_eq!(
                    errors["start_date"][0].code,
                    "start_date_must_be_before_end_date"
                );
                assert_eq!(
                    &errors["start_date"][0]
                        .message
                        .clone()
                        .unwrap()
                        .into_owned(),
                    "Start date must be before end date"
                );
            }
            _ => panic!("Expected validation error"),
        },
    }
}

#[test]
fn find() {
    let db = TestProject::new();
    let event = db.create_event().with_tickets().finish();
    let ticket_type = &event
        .ticket_types(true, None, &db.get_connection())
        .unwrap()[0];

    let found_ticket_type = TicketType::find(ticket_type.id, &db.get_connection()).unwrap();
    assert_eq!(&found_ticket_type, ticket_type);
}
