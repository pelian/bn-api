use actix_web::{http::StatusCode, FromRequest, HttpResponse, Path, Query};
use bigneon_api::controllers::orders::{self, *};
use bigneon_api::extractors::Json;
use bigneon_api::models::PathParameters;
use bigneon_db::models::*;
use bigneon_db::schema;
use chrono::prelude::*;
use diesel;
use diesel::prelude::*;
use functional::base;
use serde_json;
use std::collections::HashMap;
use support;
use support::database::TestDatabase;
use support::test_request::TestRequest;
use uuid::Uuid;

#[test]
pub fn show() {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let mut order = database.create_order().for_user(&user).finish();
    let conn = database.connection.get();
    let total = order.calculate_total(conn).unwrap();
    order
        .add_external_payment(Some("test".to_string()), user.id, total, conn)
        .unwrap();
    assert_eq!(order.status, OrderStatus::Paid);

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = order.id;

    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);
    let response: HttpResponse =
        orders::show((database.connection.clone(), path, auth_user)).into();
    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();
    let found_order: DisplayOrder = serde_json::from_str(&body).unwrap();
    assert_eq!(found_order.id, order.id);
}

#[cfg(test)]
mod show_other_user_order_tests {
    use super::*;
    #[test]
    fn show_other_user_order_org_member() {
        base::orders::show_other_user_order(Roles::OrgMember, true);
    }
    #[test]
    fn show_other_user_order_admin() {
        base::orders::show_other_user_order(Roles::Admin, true);
    }
    #[test]
    fn show_other_user_order_user() {
        base::orders::show_other_user_order(Roles::User, false);
    }
    #[test]
    fn show_other_user_order_org_owner() {
        base::orders::show_other_user_order(Roles::OrgOwner, true);
    }
    #[test]
    fn show_other_user_order_door_person() {
        base::orders::show_other_user_order(Roles::DoorPerson, false);
    }
    #[test]
    fn show_other_user_order_org_admin() {
        base::orders::show_other_user_order(Roles::OrgAdmin, true);
    }
    #[test]
    fn show_other_user_order_box_office() {
        base::orders::show_other_user_order(Roles::OrgBoxOffice, false);
    }
}

#[cfg(test)]
mod show_other_user_order_not_matching_users_organization_tests {
    use super::*;
    #[test]
    fn show_other_user_order_not_matching_users_organization_org_member() {
        base::orders::show_other_user_order_not_matching_users_organization(
            Roles::OrgMember,
            false,
        );
    }
    #[test]
    fn show_other_user_order_not_matching_users_organization_admin() {
        base::orders::show_other_user_order_not_matching_users_organization(Roles::Admin, true);
    }
    #[test]
    fn show_other_user_order_not_matching_users_organization_user() {
        base::orders::show_other_user_order_not_matching_users_organization(Roles::User, false);
    }
    #[test]
    fn show_other_user_order_not_matching_users_organization_org_owner() {
        base::orders::show_other_user_order_not_matching_users_organization(Roles::OrgOwner, false);
    }
    #[test]
    fn show_other_user_order_not_matching_users_organization_door_person() {
        base::orders::show_other_user_order_not_matching_users_organization(
            Roles::DoorPerson,
            false,
        );
    }
    #[test]
    fn show_other_user_order_not_matching_users_organization_org_admin() {
        base::orders::show_other_user_order_not_matching_users_organization(Roles::OrgAdmin, false);
    }
    #[test]
    fn show_other_user_order_not_matching_users_organization_box_office() {
        base::orders::show_other_user_order_not_matching_users_organization(
            Roles::OrgBoxOffice,
            false,
        );
    }
}

#[test]
pub fn show_for_draft_returns_forbidden() {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let order = database.create_order().for_user(&user).finish();
    assert_eq!(order.status, OrderStatus::Draft);

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = order.id;

    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);
    let response: HttpResponse =
        orders::show((database.connection.clone(), path, auth_user)).into();
    support::expects_forbidden(&response, Some("You do not have access to this order"));
}

#[test]
pub fn index() {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let mut order1 = database.create_order().for_user(&user).finish();
    let date1 = NaiveDate::from_ymd(2017, 7, 8).and_hms(9, 10, 11);
    let date2 = NaiveDate::from_ymd(2017, 7, 9).and_hms(9, 10, 11);
    let conn = database.connection.get();
    let total = order1.calculate_total(conn).unwrap();
    order1
        .add_external_payment(Some("test".to_string()), user.id, total, conn)
        .unwrap();
    order1 = diesel::update(&order1)
        .set(schema::orders::order_date.eq(date1))
        .get_result(conn)
        .unwrap();
    let mut order2 = database.create_order().for_user(&user).finish();
    let total = order2.calculate_total(conn).unwrap();
    order2
        .add_external_payment(Some("test".to_string()), user.id, total - 100, conn)
        .unwrap();
    order2 = diesel::update(&order2)
        .set(schema::orders::order_date.eq(date2))
        .get_result(conn)
        .unwrap();

    assert_eq!(order1.status, OrderStatus::Paid);
    assert_eq!(order2.status, OrderStatus::Draft);

    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);
    let test_request = TestRequest::create_with_uri(&format!("/?"));
    let query_parameters = Query::<PagingParameters>::extract(&test_request.request).unwrap();
    let response: HttpResponse =
        orders::index((database.connection.clone(), query_parameters, auth_user)).into();

    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();

    let orders: Payload<DisplayOrder> = serde_json::from_str(body).unwrap();
    assert_eq!(orders.data.len(), 1);
    let order_ids: Vec<Uuid> = orders.data.iter().map(|o| o.id).collect();
    assert_eq!(order_ids, vec![order1.id]);
}

#[cfg(test)]
mod details_tests {
    use super::*;
    #[test]
    fn details_org_member() {
        base::orders::details(Roles::OrgMember, true);
    }
    #[test]
    fn details_admin() {
        base::orders::details(Roles::Admin, true);
    }
    #[test]
    fn details_user() {
        base::orders::details(Roles::User, false);
    }
    #[test]
    fn details_org_owner() {
        base::orders::details(Roles::OrgOwner, true);
    }
    #[test]
    fn details_door_person() {
        base::orders::details(Roles::DoorPerson, false);
    }
    #[test]
    fn details_org_admin() {
        base::orders::details(Roles::OrgAdmin, true);
    }
    #[test]
    fn details_box_office() {
        base::orders::details(Roles::OrgBoxOffice, false);
    }
}

#[cfg(test)]
mod refund_tests {
    use super::*;
    #[test]
    fn refund_org_member() {
        base::orders::refund(Roles::OrgMember, true);
    }
    #[test]
    fn refund_admin() {
        base::orders::refund(Roles::Admin, true);
    }
    #[test]
    fn refund_user() {
        base::orders::refund(Roles::User, false);
    }
    #[test]
    fn refund_org_owner() {
        base::orders::refund(Roles::OrgOwner, true);
    }
    #[test]
    fn refund_door_person() {
        base::orders::refund(Roles::DoorPerson, false);
    }
    #[test]
    fn refund_org_admin() {
        base::orders::refund(Roles::OrgAdmin, true);
    }
    #[test]
    fn refund_box_office() {
        base::orders::refund(Roles::OrgBoxOffice, false);
    }
}

#[test]
pub fn details_with_tickets_user_has_no_access_to() {
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
        support::create_auth_user_from_user(&user, Roles::OrgOwner, Some(&organization), &database);
    let event = database
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let event2 = database
        .create_event()
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let user = database.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
    let ticket_type2 = &event2.ticket_types(true, None, connection).unwrap()[0];
    cart.update_quantities(
        user.id,
        &[
            UpdateOrderItem {
                ticket_type_id: ticket_type.id,
                quantity: 2,
                redemption_code: None,
            },
            UpdateOrderItem {
                ticket_type_id: ticket_type2.id,
                quantity: 2,
                redemption_code: None,
            },
        ],
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

    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();
    let details_response: DetailsResponse = serde_json::from_str(&body).unwrap();
    assert_eq!(details_response.items, expected_order_details);
    assert_eq!(
        details_response.order_contains_tickets_for_other_organizations,
        true
    );
}

#[test]
pub fn refund_for_non_refundable_tickets() {
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
    let user2 = database.create_user().finish();
    let auth_user =
        support::create_auth_user_from_user(&user, Roles::OrgOwner, Some(&organization), &database);
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

    let tickets = TicketInstance::find_for_order_item(order_item.id, connection).unwrap();
    let ticket = &tickets[0];

    // Transfer the first ticket away
    let sender_wallet = Wallet::find_default_for_user(user.id, connection).unwrap();
    let receiver_wallet = Wallet::find_default_for_user(user2.id, connection).unwrap();
    let transfer_auth =
        TicketInstance::authorize_ticket_transfer(user.id, vec![ticket.id], 3600, connection)
            .unwrap();
    TicketInstance::receive_ticket_transfer(
        transfer_auth,
        &sender_wallet,
        &receiver_wallet.id,
        connection,
    )
    .unwrap();

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

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // Reload ticket
    let ticket = TicketInstance::find(ticket.id, connection).unwrap();
    assert!(ticket.order_item_id.is_some());

    // Reload order item
    let order_item = OrderItem::find_in_order(cart.id, order_item.id, connection).unwrap();
    assert_eq!(order_item.refunded_quantity, 0);

    // Reload fee item
    let fee_item = order_item.find_fee_item(connection).unwrap().unwrap();
    assert_eq!(fee_item.refunded_quantity, 0);

    // Reload event fee
    let event_fee_item = OrderItem::find(event_fee_item.id, connection).unwrap();
    assert_eq!(event_fee_item.refunded_quantity, 0);
}

#[test]
pub fn refund_hold_ticket() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let organization = database.create_organization().finish();
    let event = database
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let hold = database
        .create_hold()
        .with_quantity(1)
        .with_event(&event)
        .finish();
    let user = database.create_user().finish();
    let auth_user =
        support::create_auth_user_from_user(&user, Roles::OrgOwner, Some(&organization), &database);
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 1,
            redemption_code: Some(hold.redemption_code.clone()),
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
    let fee_item = order_item.find_fee_item(connection).unwrap().unwrap();
    let tickets = TicketInstance::find_for_order_item(order_item.id, connection).unwrap();
    let ticket = &tickets[0];

    // Refund first ticket and event fee (leaving one ticket + one fee item for that ticket)
    let refund_items = vec![RefundItem {
        order_item_id: order_item.id,
        ticket_instance_id: Some(ticket.id),
    }];
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

    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();
    let refund_response: RefundResponse = serde_json::from_str(&body).unwrap();
    let expected_refund_amount = order_item.unit_price_in_cents + fee_item.unit_price_in_cents;
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
    assert_eq!(ticket.status, TicketInstanceStatus::Available);

    // Confirm hold ticket is available for purchase again via new cart
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 1,
            redemption_code: Some(hold.redemption_code.clone()),
        }],
        false,
        false,
        connection,
    )
    .unwrap();

    // Reload ticket
    let ticket = TicketInstance::find(ticket.id, connection).unwrap();
    assert!(ticket.order_item_id.is_some());
    assert_eq!(ticket.status, TicketInstanceStatus::Reserved);
    assert_ne!(Some(order_item.id), ticket.order_item_id);
}

#[test]
pub fn refund_removes_event_fee_if_all_event_tickets_refunded() {
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
        support::create_auth_user_from_user(&user, Roles::OrgOwner, Some(&organization), &database);
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
    let ticket2 = &tickets[1];

    // Refund first ticket and event fee (leaving one ticket + one fee item for that ticket)
    let refund_items = vec![
        RefundItem {
            order_item_id: order_item.id,
            ticket_instance_id: Some(ticket.id),
        },
        RefundItem {
            order_item_id: order_item.id,
            ticket_instance_id: Some(ticket2.id),
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

    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();
    let refund_response: RefundResponse = serde_json::from_str(&body).unwrap();
    let expected_refund_amount = event_fee_item.unit_price_in_cents
        + order_item.unit_price_in_cents * 2
        + fee_item.unit_price_in_cents * 2;
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
    assert_eq!(order_item.refunded_quantity, 2);

    // Reload fee item
    let fee_item = order_item.find_fee_item(connection).unwrap().unwrap();
    assert_eq!(fee_item.refunded_quantity, 2);

    // Reload event fee
    let event_fee_item = OrderItem::find(event_fee_item.id, connection).unwrap();
    assert_eq!(event_fee_item.refunded_quantity, 1);
}
