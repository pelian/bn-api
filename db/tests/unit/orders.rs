use bigneon_db::dev::times;
use bigneon_db::dev::TestProject;
use bigneon_db::models::*;
use bigneon_db::schema::orders;
use bigneon_db::utils::errors::ErrorCode::ValidationError;
use chrono::prelude::*;
use diesel;
use diesel::prelude::*;
use time::Duration;
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
    let _event = project
        .create_event()
        .with_name("NewEvent".into())
        .with_organization(&organization)
        .with_venue(&venue)
        .finish();
    let order = Order::find_or_create_cart(&user, project.get_connection()).unwrap();
    assert_eq!(order.user_id, user.id);
    assert_eq!(order.id.to_string().is_empty(), false);
}

#[test]
fn add_tickets() {
    let project = TestProject::new();
    let creator = project.create_user().finish();

    let connection = project.get_connection();
    let organization = project
        .create_organization()
        .with_fee_schedule(&project.create_fee_schedule().finish(creator.id))
        .finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    let ticket = &event.ticket_types(true, None, connection).unwrap()[0];
    cart.update_quantities(
        user.id,
        &vec![UpdateOrderItem {
            ticket_type_id: ticket.id,
            quantity: 10,
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
        .find(|i| i.ticket_type_id == Some(ticket.id))
        .unwrap();

    let ticket_pricing =
        TicketPricing::find(order_item.ticket_pricing_id.unwrap(), connection).unwrap();
    assert_eq!(order_item.calculate_quantity(connection), Ok(10));
    assert_eq!(
        order_item.unit_price_in_cents,
        ticket_pricing.price_in_cents
    );

    let fee_item = order_item.find_fee_item(connection).unwrap().unwrap();
    let fee_schedule_range =
        FeeScheduleRange::find(fee_item.fee_schedule_range_id.unwrap(), connection).unwrap();
    assert_eq!(
        fee_item.unit_price_in_cents,
        fee_schedule_range.fee_in_cents
    );
    assert_eq!(fee_item.quantity, 10);

    // Add some more
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket.id,
            quantity: 15,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();
    let items = cart.items(connection).unwrap();

    assert_eq!(items.len(), 2);
    let order_item = items
        .iter()
        .find(|i| i.ticket_type_id == Some(ticket.id))
        .unwrap();

    assert_eq!(order_item.calculate_quantity(connection), Ok(15));
}

#[test]
fn add_tickets_below_min_fee() {
    let project = TestProject::new();
    let creator = project.create_user().finish();

    let connection = project.get_connection();
    let organization = project
        .create_organization()
        .with_fee_schedule(&project.create_fee_schedule().finish(creator.id))
        .finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .finish();

    let ticket_type = event
        .add_ticket_type(
            "Free Tix".to_string(),
            None,
            10,
            times::zero(),
            times::infinity(),
            event.issuer_wallet(connection).unwrap().id,
            Some(1),
            10,
            0,
            connection,
        )
        .unwrap();

    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();

    cart.update_quantities(
        user.id,
        &vec![UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 10,
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

    assert_eq!(order_item.unit_price_in_cents, 0);
    assert_eq!(items.len(), 1);

    let fee_item = order_item.find_fee_item(connection).unwrap();

    assert_eq!(fee_item, None);
}

#[test]
fn details() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let creator = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_event_fee()
        .with_fee_schedule(&project.create_fee_schedule().finish(creator.id))
        .finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 2,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();

    let total = cart.calculate_total(connection).unwrap();
    cart.add_external_payment(Some("Test".to_string()), user.id, total, connection)
        .unwrap();

    let items = cart.items(connection).unwrap();
    let order_item = OrderItem::find(
        items
            .iter()
            .find(|i| i.ticket_type_id == Some(ticket_type.id))
            .unwrap()
            .id,
        connection,
    )
    .unwrap();

    let event_fee_item = items
        .iter()
        .find(|i| i.item_type == OrderItemTypes::EventFees)
        .unwrap();

    let fee_item = order_item.find_fee_item(connection).unwrap().unwrap();
    let tickets = TicketInstance::find_for_order_item(order_item.id, connection).unwrap();
    let ticket = &tickets[0];
    let ticket2 = &tickets[1];

    let refund_items = vec![RefundItem {
        order_item_id: order_item.id,
        ticket_instance_id: Some(ticket.id),
    }];
    let refund_amount = order_item.unit_price_in_cents + fee_item.unit_price_in_cents;
    assert_eq!(
        cart.refund(refund_items, user.id, connection).unwrap(),
        refund_amount as u32
    );

    let mut expected_order_details = vec![
        OrderDetailsLineItem {
            ticket_instance_id: Some(ticket2.id),
            order_item_id: order_item.id,
            description: format!("{} - {}", event.name, ticket_type.name),
            ticket_price_in_cents: order_item.unit_price_in_cents,
            fees_price_in_cents: fee_item.unit_price_in_cents,
            total_price_in_cents: order_item.unit_price_in_cents + fee_item.unit_price_in_cents,
            status: "Purchased".to_string(),
            refundable: true,
        },
        OrderDetailsLineItem {
            ticket_instance_id: Some(ticket.id),
            order_item_id: order_item.id,
            description: format!("{} - {}", event.name, ticket_type.name),
            ticket_price_in_cents: 0,
            fees_price_in_cents: 0,
            total_price_in_cents: 0,
            status: "Refunded".to_string(),
            refundable: false,
        },
    ];

    expected_order_details.sort_by(|a, b| {
        a.ticket_instance_id
            .unwrap()
            .cmp(&b.ticket_instance_id.unwrap())
    });
    expected_order_details.push(OrderDetailsLineItem {
        ticket_instance_id: None,
        order_item_id: event_fee_item.id,
        description: format!("Event Fees - {}", event.name),
        ticket_price_in_cents: 0,
        fees_price_in_cents: event_fee_item.unit_price_in_cents,
        total_price_in_cents: event_fee_item.unit_price_in_cents,
        status: "Purchased".to_string(),
        refundable: true,
    });

    let order_details = cart.details(vec![organization.id], connection).unwrap();
    assert_eq!(expected_order_details, order_details);

    // No details when this organization is not specified
    assert!(cart.details(vec![], connection).unwrap().is_empty());

    // Refund already refunded ticket which doesn't change anything
    let refund_items = vec![RefundItem {
        order_item_id: order_item.id,
        ticket_instance_id: Some(ticket.id),
    }];
    assert!(cart.refund(refund_items, user.id, connection).is_err());
    let order_details = cart.details(vec![organization.id], connection).unwrap();
    assert_eq!(expected_order_details, order_details);

    // Refund last item triggering event fee to refund as well
    let refund_items = vec![RefundItem {
        order_item_id: order_item.id,
        ticket_instance_id: Some(ticket2.id),
    }];
    let refund_amount = order_item.unit_price_in_cents
        + fee_item.unit_price_in_cents
        + event_fee_item.unit_price_in_cents;
    assert_eq!(
        cart.refund(refund_items, user.id, connection).unwrap(),
        refund_amount as u32
    );

    let mut expected_order_details = vec![
        OrderDetailsLineItem {
            ticket_instance_id: Some(ticket2.id),
            order_item_id: order_item.id,
            description: format!("{} - {}", event.name, ticket_type.name),
            ticket_price_in_cents: 0,
            fees_price_in_cents: 0,
            total_price_in_cents: 0,
            status: "Refunded".to_string(),
            refundable: false,
        },
        OrderDetailsLineItem {
            ticket_instance_id: Some(ticket.id),
            order_item_id: order_item.id,
            description: format!("{} - {}", event.name, ticket_type.name),
            ticket_price_in_cents: 0,
            fees_price_in_cents: 0,
            total_price_in_cents: 0,
            status: "Refunded".to_string(),
            refundable: false,
        },
    ];

    expected_order_details.sort_by(|a, b| {
        a.ticket_instance_id
            .unwrap()
            .cmp(&b.ticket_instance_id.unwrap())
    });
    expected_order_details.push(OrderDetailsLineItem {
        ticket_instance_id: None,
        order_item_id: event_fee_item.id,
        description: format!("Event Fees - {}", event.name),
        ticket_price_in_cents: 0,
        fees_price_in_cents: 0,
        total_price_in_cents: 0,
        status: "Refunded".to_string(),
        refundable: false,
    });

    let order_details = cart.details(vec![organization.id], connection).unwrap();
    assert_eq!(expected_order_details, order_details);
}

#[test]
fn refund() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let creator = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_event_fee()
        .with_fee_schedule(&project.create_fee_schedule().finish(creator.id))
        .finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 2,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();

    let total = cart.calculate_total(connection).unwrap();
    cart.add_external_payment(Some("Test".to_string()), user.id, total, connection)
        .unwrap();

    let items = cart.items(&connection).unwrap();
    let order_item = items
        .iter()
        .find(|i| i.ticket_type_id == Some(ticket_type.id))
        .unwrap();

    let event_fee_item = items
        .iter()
        .find(|i| i.item_type == OrderItemTypes::EventFees)
        .unwrap();

    let fee_item = order_item.find_fee_item(connection).unwrap().unwrap();
    let tickets = TicketInstance::find_for_order_item(order_item.id, connection).unwrap();
    let ticket = &tickets[0];

    // Refund first ticket and event fee (leaving one ticket + one fee item for that ticket)
    let refund_items = vec![
        RefundItem {
            order_item_id: order_item.id,
            ticket_instance_id: Some(ticket.id),
        },
        RefundItem {
            order_item_id: event_fee_item.id,
            ticket_instance_id: None,
        },
    ];
    let refund_amount = event_fee_item.unit_price_in_cents
        + order_item.unit_price_in_cents
        + fee_item.unit_price_in_cents;
    assert_eq!(
        cart.refund(refund_items, user.id, connection).unwrap(),
        refund_amount as u32
    );

    // Reload ticket
    let ticket = TicketInstance::find(ticket.id, connection).unwrap();
    assert!(ticket.order_item_id.is_none());

    // Reload order item
    let order_item = OrderItem::find_in_order(cart.id, order_item.id, connection).unwrap();
    assert_eq!(order_item.refunded_quantity, 1);

    // Reload fee item
    let fee_item = order_item.find_fee_item(connection).unwrap().unwrap();
    assert_eq!(fee_item.refunded_quantity, 1);
}

#[test]
fn organizations() {
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
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
    let ticket_type2 = &event2.ticket_types(true, None, connection).unwrap()[0];
    cart.update_quantities(
        user.id,
        &[
            UpdateOrderItem {
                ticket_type_id: ticket_type.id,
                quantity: 10,
                redemption_code: None,
            },
            UpdateOrderItem {
                ticket_type_id: ticket_type2.id,
                quantity: 10,
                redemption_code: None,
            },
        ],
        false,
        false,
        connection,
    )
    .unwrap();

    assert_eq!(
        cart.organizations(connection).unwrap(),
        vec![organization, organization2]
    );
}

#[test]
fn payments() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let event = project
        .create_event()
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
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
    assert_eq!(cart.calculate_total(connection).unwrap(), 2000);

    cart.add_external_payment(Some("Test".to_string()), user.id, 500, connection)
        .unwrap();
    cart.add_external_payment(Some("Test2".to_string()), user.id, 1500, connection)
        .unwrap();

    let payments = cart.payments(connection).unwrap();
    assert_eq!(payments.len(), 2);
    assert_eq!(
        payments.iter().map(|p| p.amount).collect::<Vec<i64>>(),
        vec![500, 1500]
    );
}

#[test]
fn add_tickets_with_increment() {
    let project = TestProject::new();
    let creator = project.create_user().finish();

    let connection = project.get_connection();
    let organization = project
        .create_organization()
        .with_fee_schedule(&project.create_fee_schedule().finish(creator.id))
        .finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
    let update_parameters = TicketTypeEditableAttributes {
        increment: Some(4),
        ..Default::default()
    };
    let ticket_type = ticket_type.update(update_parameters, connection).unwrap();

    let add_tickets_result = cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 10,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    );
    assert!(add_tickets_result.is_err());
    let error = add_tickets_result.unwrap_err();
    match &error.error_code {
        ValidationError { errors } => {
            assert!(errors.contains_key("quantity"));
            assert_eq!(errors["quantity"].len(), 1);
            assert_eq!(errors["quantity"][0].code, "quantity_invalid_increment");
            assert_eq!(
                &errors["quantity"][0].message.clone().unwrap().into_owned(),
                "Order item quantity invalid for ticket pricing increment"
            );
        }
        _ => panic!("Expected validation error"),
    }

    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 4,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();

    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 12,
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

    assert_eq!(order_item.quantity, 12);
}

#[test]
fn clear_cart() {
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
        .with_tickets()
        .with_ticket_pricing()
        .finish();

    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];

    cart.update_quantities(
        user.id,
        &vec![UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 10,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();
    assert!(!cart.items(&connection).unwrap().is_empty());

    cart.clear_cart(connection).unwrap();
    assert!(cart.items(&connection).unwrap().is_empty());
}

#[test]
fn replace_tickets_for_box_office() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let creator = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_event_fee()
        .with_fee_schedule(&project.create_fee_schedule().finish(creator.id))
        .finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();

    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    assert!(!cart.box_office_pricing);

    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];

    let box_office_pricing = ticket_type
        .add_ticket_pricing(
            "Box office".into(),
            NaiveDate::from_ymd(2016, 7, 8).and_hms(7, 8, 10),
            NaiveDate::from_ymd(9999, 7, 8).and_hms(7, 8, 10),
            5000,
            true,
            None,
            connection,
        )
        .unwrap();

    // Add normal tickets to cart (box_office_pricing = false)
    cart.update_quantities(
        user.id,
        &vec![UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 10,
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
    let ticket_pricing =
        TicketPricing::find(order_item.ticket_pricing_id.unwrap(), connection).unwrap();
    assert_eq!(order_item.calculate_quantity(connection), Ok(10));
    assert_eq!(
        order_item.unit_price_in_cents,
        ticket_pricing.price_in_cents
    );

    let fee_item = order_item.find_fee_item(connection).unwrap().unwrap();
    let fee_schedule_range =
        FeeScheduleRange::find(fee_item.fee_schedule_range_id.unwrap(), connection).unwrap();
    assert_eq!(
        fee_item.unit_price_in_cents,
        fee_schedule_range.fee_in_cents
    );
    assert_eq!(fee_item.quantity, 10);
    assert!(items
        .iter()
        .find(|i| i.item_type == OrderItemTypes::EventFees)
        .is_some());

    // Add box office priced tickets to cart (box_office_pricing = true)
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 10,
            redemption_code: None,
        }],
        true,
        true,
        connection,
    )
    .unwrap();
    assert!(cart.box_office_pricing);
    let items = cart.items(connection).unwrap();
    assert_eq!(items.len(), 1);
    let order_item = items
        .iter()
        .find(|i| i.ticket_type_id == Some(ticket_type.id))
        .unwrap();
    assert_eq!(order_item.calculate_quantity(connection), Ok(10));
    assert_eq!(
        order_item.unit_price_in_cents,
        box_office_pricing.price_in_cents
    );

    // No fee for box office items
    assert!(order_item.find_fee_item(connection).unwrap().is_none());
    assert!(items
        .iter()
        .find(|i| i.item_type == OrderItemTypes::EventFees)
        .is_none());
}

#[test]
fn replace_tickets() {
    let project = TestProject::new();
    let creator = project.create_user().finish();

    let connection = project.get_connection();
    let organization = project
        .create_organization()
        .with_fee_schedule(&project.create_fee_schedule().finish(creator.id))
        .finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    let ticket = &event.ticket_types(true, None, connection).unwrap()[0];
    cart.update_quantities(
        user.id,
        &vec![UpdateOrderItem {
            ticket_type_id: ticket.id,
            quantity: 10,
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
        .find(|i| i.ticket_type_id == Some(ticket.id))
        .unwrap();

    let ticket_pricing =
        TicketPricing::find(order_item.ticket_pricing_id.unwrap(), connection).unwrap();
    assert_eq!(order_item.calculate_quantity(connection), Ok(10));
    assert_eq!(
        order_item.unit_price_in_cents,
        ticket_pricing.price_in_cents
    );

    let fee_item = order_item.find_fee_item(connection).unwrap().unwrap();
    let fee_schedule_range =
        FeeScheduleRange::find(fee_item.fee_schedule_range_id.unwrap(), connection).unwrap();
    assert_eq!(
        fee_item.unit_price_in_cents,
        fee_schedule_range.fee_in_cents
    );
    assert_eq!(fee_item.quantity, 10);

    // Add some more
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket.id,
            quantity: 15,
            redemption_code: None,
        }],
        false,
        true,
        connection,
    )
    .unwrap();
    let items = cart.items(connection).unwrap();

    assert_eq!(items.len(), 2);
    let order_item = items
        .iter()
        .find(|i| i.ticket_type_id == Some(ticket.id))
        .unwrap();

    assert_eq!(order_item.calculate_quantity(connection), Ok(15));
}

#[test]
fn replace_tickets_with_code_pricing() {
    let project = TestProject::new();
    let creator = project.create_user().finish();

    let connection = project.get_connection();
    let organization = project
        .create_organization()
        .with_fee_schedule(&project.create_fee_schedule().finish(creator.id))
        .finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
    let discount_in_cents: i64 = 20;
    let code = project
        .create_code()
        .with_discount_in_cents(Some(discount_in_cents as u32))
        .with_code_type(CodeTypes::Access)
        .with_event(&event)
        .for_ticket_type(&ticket_type)
        .finish();
    cart.update_quantities(
        user.id,
        &vec![UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 10,
            redemption_code: Some(code.redemption_code.clone()),
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

    let ticket_pricing =
        TicketPricing::find(order_item.ticket_pricing_id.unwrap(), connection).unwrap();
    assert_eq!(order_item.calculate_quantity(connection), Ok(10));
    assert_eq!(
        order_item.unit_price_in_cents,
        ticket_pricing.price_in_cents - discount_in_cents
    );

    let fee_item = order_item.find_fee_item(connection).unwrap().unwrap();
    let fee_schedule_range =
        FeeScheduleRange::find(fee_item.fee_schedule_range_id.unwrap(), connection).unwrap();
    assert_eq!(
        fee_item.unit_price_in_cents,
        fee_schedule_range.fee_in_cents
    );
    assert_eq!(fee_item.quantity, 10);

    // Add some more
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 15,
            redemption_code: Some(code.redemption_code.clone()),
        }],
        false,
        true,
        connection,
    )
    .unwrap();
    let items = cart.items(connection).unwrap();

    assert_eq!(items.len(), 2);
    let order_item = items
        .iter()
        .find(|i| i.ticket_type_id == Some(ticket_type.id))
        .unwrap();

    assert_eq!(order_item.calculate_quantity(connection), Ok(15));
}

#[test]
fn remove_tickets() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project
        .create_event()
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
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
    let items = cart.items(&connection).unwrap();
    let order_item = items
        .iter()
        .find(|i| i.ticket_type_id == Some(ticket_type.id))
        .unwrap();

    let ticket_pricing =
        TicketPricing::find(order_item.ticket_pricing_id.unwrap(), connection).unwrap();
    assert_eq!(order_item.quantity, 10);
    assert_eq!(
        order_item.unit_price_in_cents,
        ticket_pricing.price_in_cents
    );

    let fee_item = order_item.find_fee_item(connection).unwrap().unwrap();
    let fee_schedule_range =
        FeeScheduleRange::find(fee_item.fee_schedule_range_id.unwrap(), connection).unwrap();
    assert_eq!(
        fee_item.unit_price_in_cents,
        fee_schedule_range.fee_in_cents
    );
    assert_eq!(fee_item.quantity, 10);

    // Remove tickets
    assert!(cart
        .update_quantities(
            user.id,
            &[UpdateOrderItem {
                ticket_type_id: ticket_type.id,
                quantity: 6,
                redemption_code: None,
            }],
            false,
            false,
            connection
        )
        .is_ok());
    let items = cart.items(&connection).unwrap();
    let order_item = items
        .iter()
        .find(|i| i.ticket_type_id == Some(ticket_type.id))
        .unwrap();
    assert_eq!(order_item.quantity, 6);
    assert_eq!(
        order_item.unit_price_in_cents,
        ticket_pricing.price_in_cents
    );
    let fee_item = order_item.find_fee_item(connection).unwrap().unwrap();
    assert_eq!(
        fee_item.unit_price_in_cents,
        fee_schedule_range.fee_in_cents
    );
    assert_eq!(fee_item.quantity, 6);

    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 0,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();
    // Item removed from cart completely
    assert!(cart.items(connection).unwrap().is_empty());
}

#[test]
fn clear_tickets() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project
        .create_event()
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
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

    // Remove tickets
    assert!(cart
        .update_quantities(user.id, &[], false, true, connection)
        .is_ok());

    // Item removed from cart completely
    assert!(cart.items(connection).unwrap().is_empty());
}

#[test]
fn remove_tickets_with_increment() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project
        .create_event()
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
    let update_parameters = TicketTypeEditableAttributes {
        increment: Some(4),
        ..Default::default()
    };
    let ticket_type = ticket_type.update(update_parameters, connection).unwrap();
    let add_tickets_result = cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 8,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    );
    assert!(add_tickets_result.is_ok());
    let items = cart.items(&connection).unwrap();
    let order_item = items
        .iter()
        .find(|i| i.ticket_type_id == Some(ticket_type.id))
        .unwrap();
    assert_eq!(order_item.quantity, 8);

    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 4,
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

    assert_eq!(order_item.quantity, 4);

    let remove_tickets_result = cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 5,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    );
    assert!(remove_tickets_result.is_err());
    let error = remove_tickets_result.unwrap_err();
    match &error.error_code {
        ValidationError { errors } => {
            assert!(errors.contains_key("quantity"));
            assert_eq!(errors["quantity"].len(), 1);
            assert_eq!(errors["quantity"][0].code, "quantity_invalid_increment");
            assert_eq!(
                &errors["quantity"][0].message.clone().unwrap().into_owned(),
                "Order item quantity invalid for ticket pricing increment"
            );
        }
        _ => panic!("Expected validation error"),
    }
}

#[test]
fn find_item() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project
        .create_event()
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
    let user = project.create_user().finish();
    let user2 = project.create_user().finish();
    let mut order = Order::find_or_create_cart(&user, connection).unwrap();
    order
        .update_quantities(
            user.id,
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
    let mut order2 = Order::find_or_create_cart(&user2, connection).unwrap();
    order2
        .update_quantities(
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

    let items = order.items(&connection).unwrap();
    let order_item = items
        .iter()
        .find(|i| i.ticket_type_id == Some(ticket_type.id))
        .unwrap();
    let items = order2.items(&connection).unwrap();
    let order_item2 = items
        .iter()
        .find(|i| i.ticket_type_id == Some(ticket_type.id))
        .unwrap();

    let found_item = order.find_item(order_item.id.clone(), connection).unwrap();
    assert_eq!(order_item, &found_item);

    let found_item = order2
        .find_item(order_item2.id.clone(), connection)
        .unwrap();
    assert_eq!(order_item2, &found_item);

    let find_results = order.find_item(order_item2.id.clone(), connection);
    assert_eq!(
        find_results.unwrap_err().cause,
        Some("Could not retrieve order item, NotFound".into())
    );

    let find_results = order2.find_item(order_item.id.clone(), connection);
    assert_eq!(
        find_results.unwrap_err().cause,
        Some("Could not retrieve order item, NotFound".into())
    );

    let find_results = order.find_item(Uuid::new_v4(), connection);
    assert_eq!(
        find_results.unwrap_err().cause,
        Some("Could not retrieve order item, NotFound".into())
    );
}

#[test]
fn find_cart_for_user() {
    let project = TestProject::new();
    let user = project.create_user().finish();

    // No cart
    let conn = project.get_connection();
    let cart_result = Order::find_cart_for_user(user.id, conn).unwrap();
    assert!(cart_result.is_none());

    // Cart exists, is not expired
    let cart = Order::find_or_create_cart(&user, conn).unwrap();
    let cart_result = Order::find_cart_for_user(user.id, conn);
    assert_eq!(cart_result.unwrap().unwrap(), cart);

    // Expired cart
    let one_minute_ago = NaiveDateTime::from(Utc::now().naive_utc() - Duration::minutes(1));
    diesel::update(&cart)
        .set(orders::expires_at.eq(one_minute_ago))
        .get_result::<Order>(conn)
        .unwrap();
    let cart_result = Order::find_cart_for_user(user.id, conn).unwrap();
    assert!(cart_result.is_none());
}

#[test]
fn has_items() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();

    // Without items
    assert!(!cart.has_items(connection).unwrap());

    // With items
    let event = project
        .create_event()
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
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
    assert!(cart.has_items(connection).unwrap());
}

#[test]
fn destroy() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let cart = Order::find_or_create_cart(&user, connection).unwrap();
    let cart_result = Order::find_cart_for_user(user.id, connection).unwrap();
    assert_eq!(cart_result.unwrap(), cart);

    cart.destroy(connection).unwrap();
    let cart_result = Order::find_cart_for_user(user.id, connection).unwrap();
    assert!(cart_result.is_none());
}

#[test]
fn calculate_cart_total() {
    let project = TestProject::new();
    let creator = project.create_user().finish();

    let organization = project
        .create_organization()
        .with_fee_schedule(&project.create_fee_schedule().finish(creator.id))
        .finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let user = project.create_user().finish();
    let conn = project.get_connection();
    let mut cart = Order::find_or_create_cart(&user, conn).unwrap();
    let ticket = &event.ticket_types(true, None, conn).unwrap()[0];
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket.id,
            quantity: 10,
            redemption_code: None,
        }],
        false,
        false,
        conn,
    )
    .unwrap();

    let total = cart.calculate_total(conn).unwrap();
    assert_eq!(total, 1700);

    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket.id,
            quantity: 30,
            redemption_code: None,
        }],
        false,
        false,
        conn,
    )
    .unwrap();
    let total = cart.calculate_total(conn).unwrap();
    assert_eq!(total, 5100);
}

#[test]
fn add_external_payment() {
    let project = TestProject::new();
    let user = project.create_user().finish();
    let event = project
        .create_event()
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let conn = project.get_connection();
    let mut cart = Order::find_or_create_cart(&user, conn).unwrap();
    let ticket = &event.ticket_types(true, None, conn).unwrap()[0];
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket.id,
            quantity: 10,
            redemption_code: None,
        }],
        false,
        false,
        conn,
    )
    .unwrap();
    assert_eq!(cart.calculate_total(conn).unwrap(), 2000);
    assert!(cart.paid_at.is_none());

    // Partially paid
    cart.add_external_payment(Some("test".to_string()), user.id, 1500, conn)
        .unwrap();
    assert_eq!(cart.status, OrderStatus::Draft);
    assert!(cart.paid_at.is_none());

    // Fully paid
    cart.add_external_payment(Some("test2".to_string()), user.id, 500, conn)
        .unwrap();
    assert_eq!(cart.status, OrderStatus::Paid);
    assert!(cart.paid_at.is_some());
}

#[test]
fn add_external_payment_for_expired_code() {
    let project = TestProject::new();
    let user = project.create_user().finish();
    let event = project
        .create_event()
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let conn = project.get_connection();
    let mut cart = Order::find_or_create_cart(&user, conn).unwrap();
    let ticket_type = &event.ticket_types(true, None, conn).unwrap()[0];
    let code = project
        .create_code()
        .with_code_type(CodeTypes::Access)
        .with_event(&event)
        .for_ticket_type(&ticket_type)
        .finish();
    assert_eq!(code.discount_in_cents, Some(100));
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 10,
            redemption_code: Some(code.redemption_code.clone()),
        }],
        false,
        false,
        conn,
    )
    .unwrap();
    assert_eq!(cart.calculate_total(conn).unwrap(), 1000);
    assert!(cart.paid_at.is_none());

    // Update code so it's expired
    let start_date = NaiveDateTime::from(Utc::now().naive_utc() - Duration::days(3));
    let end_date = NaiveDateTime::from(Utc::now().naive_utc() - Duration::days(2));
    code.update(
        UpdateCodeAttributes {
            start_date: Some(start_date),
            end_date: Some(end_date),
            ..Default::default()
        },
        conn,
    )
    .unwrap();

    // Attempting to pay triggers error
    let result = cart.add_external_payment(Some("test".to_string()), user.id, 1000, conn);
    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("code_id"));
                assert_eq!(errors["code_id"].len(), 1);
                assert_eq!(
                    &errors["code_id"][0].code,
                    "Code not valid for current datetime"
                );
            }
            _ => panic!("Expected validation error"),
        },
    }
}

#[test]
fn find_for_user_for_display() {
    let project = TestProject::new();
    let user = project.create_user().finish();
    let mut order1 = project.create_order().for_user(&user).finish();
    order1
        .add_external_payment(
            Some("test".to_string()),
            user.id,
            2000,
            project.get_connection(),
        )
        .unwrap();
    let mut order2 = project.create_order().for_user(&user).finish();
    order2
        .add_external_payment(
            Some("test".to_string()),
            user.id,
            500,
            project.get_connection(),
        )
        .unwrap();

    assert_eq!(order1.status, OrderStatus::Paid);
    assert_eq!(order2.status, OrderStatus::Draft);

    let display_orders =
        Order::find_for_user_for_display(user.id, project.get_connection()).unwrap();
    let ids: Vec<Uuid> = display_orders.iter().map(|o| o.id).collect();
    //The order of the ids is not certain so this test fails from time to time.
    //It is ordered by updated_at which is the same for the two orders

    assert_eq!(order1.id, ids[0]);

    // User list so items shown in full
    assert!(!&display_orders[0].order_contains_tickets_for_other_organizations);
    //    assert!(!&display_orders[1].order_contains_tickets_for_other_organizations);
}

#[test]
fn for_display() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let order = project.create_order().finish();

    // 1 minute from now expires
    let one_minute_from_now = NaiveDateTime::from(Utc::now().naive_utc() + Duration::minutes(1));
    let order = diesel::update(orders::table.filter(orders::id.eq(order.id)))
        .set(orders::expires_at.eq(one_minute_from_now))
        .get_result::<Order>(connection)
        .unwrap();
    let display_order = order.for_display(None, connection).unwrap();
    // Check both 59 and 60 for the purposes of the test to avoid timing errors
    assert!(vec![59, 60].contains(&display_order.seconds_until_expiry.unwrap()));

    // No organization filtering
    assert!(!display_order.order_contains_tickets_for_other_organizations);

    // 1 minute ago expires
    let one_minute_ago = NaiveDateTime::from(Utc::now().naive_utc() - Duration::minutes(1));
    let order = diesel::update(orders::table.filter(orders::id.eq(order.id)))
        .set(orders::expires_at.eq(one_minute_ago))
        .get_result::<Order>(connection)
        .unwrap();
    let display_order = order.for_display(None, connection).unwrap();
    assert_eq!(Some(0), display_order.seconds_until_expiry);
}

#[test]
fn for_display_with_organization_id_filter() {
    let project = TestProject::new();
    let connection = project.get_connection();

    // Events with different organizations
    let event = project
        .create_event()
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let event2 = project
        .create_event()
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
    let ticket_type2 = &event2.ticket_types(true, None, connection).unwrap()[0];

    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    cart.update_quantities(
        user.id,
        &[
            UpdateOrderItem {
                ticket_type_id: ticket_type.id,
                quantity: 1,
                redemption_code: None,
            },
            UpdateOrderItem {
                ticket_type_id: ticket_type2.id,
                quantity: 1,
                redemption_code: None,
            },
        ],
        false,
        false,
        connection,
    )
    .unwrap();

    // No filtering
    let display_order = cart.for_display(None, connection).unwrap();
    assert!(!display_order.order_contains_tickets_for_other_organizations);
    assert_eq!(display_order.items.len(), 4); // 2 tickets, 2 fees

    // With filtering
    let display_order = cart
        .for_display(Some(vec![event.organization_id]), connection)
        .unwrap();
    assert!(display_order.order_contains_tickets_for_other_organizations);
    assert_eq!(display_order.items.len(), 2); // 1 ticket, 1 fee
    let order_item: &DisplayOrderItem = display_order
        .items
        .iter()
        .find(|i| i.parent_id.is_none())
        .unwrap();
    assert_eq!(order_item.ticket_type_id, Some(ticket_type.id));

    // With filtering, entire list
    let display_order = cart
        .for_display(
            Some(vec![event.organization_id, event2.organization_id]),
            connection,
        )
        .unwrap();
    assert!(!display_order.order_contains_tickets_for_other_organizations);
    assert_eq!(display_order.items.len(), 4); // 2 tickets, 2 fees
}

#[test]
fn adding_event_fees() {
    let project = TestProject::new();
    let creator = project.create_user().finish();

    let connection = project.get_connection();
    let organization = project
        .create_organization()
        .with_fee_schedule(&project.create_fee_schedule().finish(creator.id))
        .with_event_fee()
        .finish();
    let event1 = project
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let event2 = project
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let event3 = project
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let organization2 = project
        .create_organization()
        .with_fee_schedule(&project.create_fee_schedule().finish(creator.id))
        .with_event_fee()
        .finish();
    let event4 = project
        .create_event()
        .with_organization(&organization2)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    let ticket1 = &event1.ticket_types(true, None, connection).unwrap()[0];
    let ticket2 = &event2.ticket_types(true, None, connection).unwrap()[0];
    let ticket3 = &event3.ticket_types(true, None, connection).unwrap()[0];
    let ticket4 = &event4.ticket_types(true, None, connection).unwrap()[0];
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket1.id,
            quantity: 10,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();

    let order_items = OrderItem::find_for_order(cart.id, connection).unwrap();

    let mut event_fees_count = 0;
    for o in &order_items {
        if o.item_type == OrderItemTypes::EventFees {
            event_fees_count += 1;
        }
    }
    assert_eq!(event_fees_count, 1);

    //Add some more of the same event and some of a second event
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket1.id,
            quantity: 15,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket2.id,
            quantity: 5,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();
    let order_items = OrderItem::find_for_order(cart.id, connection).unwrap();

    let mut event_fees_count = 0;

    for o in &order_items {
        if o.item_type == OrderItemTypes::EventFees {
            event_fees_count += 1;
        }
    }
    assert_eq!(event_fees_count, 2);

    //Add tickets with null event fee and null organization event_fee

    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket3.id,
            quantity: 5,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();
    let order_items = OrderItem::find_for_order(cart.id, connection).unwrap();

    let mut event_fees_count = 0;
    for o in &order_items {
        if o.item_type == OrderItemTypes::EventFees {
            event_fees_count += 1;
        }
    }
    assert_eq!(event_fees_count, 3);

    //Add tickets with null event fee and but default organization event_fee

    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket4.id,
            quantity: 5,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();
    let order_items = OrderItem::find_for_order(cart.id, connection).unwrap();

    let mut event_fees_count = 0;
    for o in &order_items {
        if o.item_type == OrderItemTypes::EventFees {
            event_fees_count += 1;
        }
    }
    assert_eq!(event_fees_count, 4);
}

#[test]
pub fn update() {
    let project = TestProject::new();
    let order = project.create_order().finish();

    let attrs = UpdateOrderAttributes {
        note: Some(Some("Client will pick up at 18h00".to_string())),
    };
    let updated = order.update(attrs, &project.connection).unwrap();
    assert_eq!(
        updated.note,
        Some("Client will pick up at 18h00".to_string())
    );
}
