use actix_web::{http::StatusCode, FromRequest, HttpResponse, Path};
use bigneon_api::controllers::orders::{self, *};
use bigneon_api::extractors::Json;
use bigneon_api::models::PathParameters;
use bigneon_db::models::*;
use serde_json;
use std::collections::HashMap;
use support;
use support::database::TestDatabase;
use support::test_request::TestRequest;

pub fn show_other_user_order(role: Roles, should_succeed: bool) {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let auth_user =
        support::create_auth_user_from_user(&user, role, Some(&organization), &database);
    let user2 = database.create_user().finish();

    // Order contains ticket type belonging to logged in user's organization
    let event = database
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let mut order = database
        .create_order()
        .for_event(&event)
        .for_user(&user2)
        .finish();

    let conn = database.connection.get();
    let total = order.calculate_total(conn).unwrap();
    order
        .add_external_payment(Some("test".to_string()), user.id, total, conn)
        .unwrap();
    assert_eq!(order.status, OrderStatus::Paid);

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = order.id;

    let response: HttpResponse =
        orders::show((database.connection.clone(), path, auth_user)).into();

    if should_succeed {
        assert_eq!(response.status(), StatusCode::OK);
        let body = support::unwrap_body_to_string(&response).unwrap();
        let found_order: DisplayOrder = serde_json::from_str(&body).unwrap();
        assert_eq!(found_order.id, order.id);
    } else {
        support::expects_forbidden(&response, None);
    }
}

pub fn show_other_user_order_not_matching_users_organization(role: Roles, should_succeed: bool) {
    let database = TestDatabase::new();
    let organization = database.create_organization().finish();
    let user = database.create_user().finish();
    let auth_user =
        support::create_auth_user_from_user(&user, role, Some(&organization), &database);
    let user2 = database.create_user().finish();
    let mut order = database.create_order().for_user(&user2).finish();

    let conn = database.connection.get();
    let total = order.calculate_total(conn).unwrap();
    order
        .add_external_payment(Some("test".to_string()), user.id, total, conn)
        .unwrap();
    assert_eq!(order.status, OrderStatus::Paid);

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = order.id;

    let response: HttpResponse =
        orders::show((database.connection.clone(), path, auth_user)).into();

    if should_succeed {
        assert_eq!(response.status(), StatusCode::OK);
        let body = support::unwrap_body_to_string(&response).unwrap();
        let found_order: DisplayOrder = serde_json::from_str(&body).unwrap();
        assert_eq!(found_order.id, order.id);
    } else {
        support::expects_forbidden(&response, None);
    }
}

pub fn details(role: Roles, should_succeed: bool) {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let creator = database.create_user().finish();
    let organization = database
        .create_organization()
        .with_event_fee()
        .with_fee_schedule(&database.create_fee_schedule().finish(creator.id))
        .finish();
    let auth_user =
        support::create_auth_user_from_user(&user, role, Some(&organization), &database);
    let event = database
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let user = database.create_user().finish();
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
        cart.refund(refund_items, auth_user.id(), connection)
            .unwrap(),
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

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = cart.id;

    let response: HttpResponse =
        orders::details((database.connection.clone(), path, auth_user)).into();

    if should_succeed {
        assert_eq!(response.status(), StatusCode::OK);
        let body = support::unwrap_body_to_string(&response).unwrap();
        let details_response: DetailsResponse = serde_json::from_str(&body).unwrap();
        assert_eq!(details_response.items, expected_order_details);
        assert_eq!(
            details_response.order_contains_tickets_for_other_organizations,
            false
        );
    } else {
        support::expects_unauthorized(&response);
    }
}

pub fn refund(role: Roles, should_succeed: bool) {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let creator = database.create_user().finish();
    let organization = database
        .create_organization()
        .with_event_fee()
        .with_fee_schedule(&database.create_fee_schedule().finish(creator.id))
        .finish();
    let event = database
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let user = database.create_user().finish();
    let auth_user =
        support::create_auth_user_from_user(&user, role, Some(&organization), &database);
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
    let json = Json(RefundAttributes {
        items: refund_items,
    });

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = cart.id;
    let response: HttpResponse = orders::refund((
        database.connection.clone(),
        path,
        json,
        auth_user,
        test_request.extract_state(),
    ))
    .into();

    if should_succeed {
        assert_eq!(response.status(), StatusCode::OK);
        let body = support::unwrap_body_to_string(&response).unwrap();
        let refund_response: RefundResponse = serde_json::from_str(&body).unwrap();
        let expected_refund_amount = event_fee_item.unit_price_in_cents
            + order_item.unit_price_in_cents
            + fee_item.unit_price_in_cents;
        assert_eq!(
            refund_response.amount_refunded,
            expected_refund_amount as u32
        );

        let mut expected_refund_breakdown = HashMap::new();
        expected_refund_breakdown.insert(PaymentMethods::External, expected_refund_amount as u32);
        assert_eq!(refund_response.refund_breakdown, expected_refund_breakdown);

        // Reload ticket
        let ticket = TicketInstance::find(ticket.id, connection).unwrap();
        assert!(ticket.order_item_id.is_none());

        // Reload order item
        let order_item = OrderItem::find_in_order(cart.id, order_item.id, connection).unwrap();
        assert_eq!(order_item.refunded_quantity, 1);

        // Reload fee item
        let fee_item = order_item.find_fee_item(connection).unwrap().unwrap();
        assert_eq!(fee_item.refunded_quantity, 1);

        // Reload event fee
        let event_fee_item = OrderItem::find(event_fee_item.id, connection).unwrap();
        assert_eq!(event_fee_item.refunded_quantity, 1);
    } else {
        support::expects_unauthorized(&response);
    }
}
