use bigneon_db::dev::TestProject;
use bigneon_db::prelude::*;
use bigneon_db::schema::orders;
use bigneon_db::utils::errors;
use bigneon_db::utils::errors::ErrorCode;
use chrono::{Duration, NaiveDateTime, Utc};
use diesel;
use diesel::prelude::*;
use std::collections::HashMap;
use uuid::Uuid;
use validator::Validate;

#[test]
fn commit() {
    let project = TestProject::new();
    let first_name = Some("Jeff".to_string());
    let last_name = Some("Wilco".to_string());
    let email = Some("jeff@tari.com".to_string());
    let phone_number = Some("555-555-5555".to_string());
    let password = "examplePassword";
    let user = User::create(
        first_name.clone(),
        last_name.clone(),
        email.clone(),
        phone_number.clone(),
        password,
    )
    .commit(project.get_connection())
    .unwrap();

    assert_eq!(user.first_name, first_name);
    assert_eq!(user.last_name, last_name);
    assert_eq!(user.email, email);
    assert_eq!(user.phone, phone_number);
    assert_ne!(user.hashed_pw, password);
    assert_eq!(user.hashed_pw.is_empty(), false);
    assert_eq!(user.id.to_string().is_empty(), false);

    let wallets = user.wallets(project.get_connection()).unwrap();
    assert_eq!(wallets.len(), 1);
}

#[test]
fn commit_duplicate_email() {
    let project = TestProject::new();
    let user1 = project.create_user().finish();
    let first_name = Some("Jeff".to_string());
    let last_name = Some("Wilco".to_string());
    let email = user1.email;
    let phone_number = Some("555-555-5555".to_string());
    let password = "examplePassword";
    let result = User::create(first_name, last_name, email, phone_number, password)
        .commit(project.get_connection());

    assert_eq!(result.is_err(), true);
    assert_eq!(
        result.err().unwrap().code,
        errors::get_error_message(&ErrorCode::DuplicateKeyError).0
    );
}

#[test]
fn find_external_login() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();

    // No external login for facebook, returns None
    assert_eq!(
        None,
        user.find_external_login(FACEBOOK_SITE, connection).unwrap()
    );

    // With external login present
    let external_login = user
        .add_external_login(
            "abc".to_string(),
            FACEBOOK_SITE.to_string(),
            "123".to_string(),
            connection,
        )
        .unwrap();
    assert_eq!(
        Some(external_login),
        user.find_external_login(FACEBOOK_SITE, connection).unwrap()
    );
}

#[test]
fn get_profile_for_organization() {
    let project = TestProject::new();
    let admin = project.create_user().finish();

    let connection = project.get_connection();
    let user = project.create_user().finish();
    let user2 = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_fee_schedule(&project.create_fee_schedule().finish(admin.id))
        .finish();

    let event = project
        .create_event()
        .with_event_start(NaiveDateTime::from(
            Utc::now().naive_utc() + Duration::days(1),
        ))
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let event2 = project
        .create_event()
        .with_event_start(NaiveDateTime::from(
            Utc::now().naive_utc() + Duration::days(2),
        ))
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let event3 = project
        .create_event()
        .with_event_start(NaiveDateTime::from(
            Utc::now().naive_utc() + Duration::days(3),
        ))
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
    let ticket_type2 = &event2.ticket_types(true, None, connection).unwrap()[0];
    let ticket_type3 = &event3.ticket_types(true, None, connection).unwrap()[0];

    // No purchases
    assert_eq!(
        user.get_profile_for_organization(&organization, connection)
            .unwrap(),
        FanProfile {
            first_name: user.first_name.clone(),
            last_name: user.last_name.clone(),
            email: user.email.clone(),
            facebook_linked: false,
            event_count: 0,
            revenue_in_cents: 0,
            ticket_sales: 0,
            profile_pic_url: user.profile_pic_url.clone(),
            thumb_profile_pic_url: user.thumb_profile_pic_url.clone(),
            cover_photo_url: user.cover_photo_url.clone(),
            created_at: user.created_at,
            attendance_information: Vec::new(),
        }
    );

    // Add facebook login
    user.add_external_login(
        "abc".to_string(),
        FACEBOOK_SITE.to_string(),
        "123".to_string(),
        connection,
    )
    .unwrap();
    assert_eq!(
        user.get_profile_for_organization(&organization, connection)
            .unwrap(),
        FanProfile {
            first_name: user.first_name.clone(),
            last_name: user.last_name.clone(),
            email: user.email.clone(),
            facebook_linked: true,
            event_count: 0,
            revenue_in_cents: 0,
            ticket_sales: 0,
            profile_pic_url: user.profile_pic_url.clone(),
            thumb_profile_pic_url: user.thumb_profile_pic_url.clone(),
            cover_photo_url: user.cover_photo_url.clone(),
            created_at: user.created_at,
            attendance_information: Vec::new(),
        }
    );

    // Add order but do not checkout
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
    assert_eq!(
        user.get_profile_for_organization(&organization, connection)
            .unwrap(),
        FanProfile {
            first_name: user.first_name.clone(),
            last_name: user.last_name.clone(),
            email: user.email.clone(),
            facebook_linked: true,
            event_count: 0,
            revenue_in_cents: 0,
            ticket_sales: 0,
            profile_pic_url: user.profile_pic_url.clone(),
            thumb_profile_pic_url: user.thumb_profile_pic_url.clone(),
            cover_photo_url: user.cover_photo_url.clone(),
            created_at: user.created_at,
            attendance_information: Vec::new(),
        }
    );

    // Checkout which changes sales data
    assert_eq!(cart.calculate_total(connection).unwrap(), 1700);
    cart.add_external_payment(Some("test".to_string()), user.id, 1700, connection)
        .unwrap();
    assert_eq!(cart.status, OrderStatus::Paid);
    assert_eq!(
        user.get_profile_for_organization(&organization, connection)
            .unwrap(),
        FanProfile {
            first_name: user.first_name.clone(),
            last_name: user.last_name.clone(),
            email: user.email.clone(),
            facebook_linked: true,
            event_count: 1,
            revenue_in_cents: 1700,
            ticket_sales: 10,
            profile_pic_url: user.profile_pic_url.clone(),
            thumb_profile_pic_url: user.thumb_profile_pic_url.clone(),
            cover_photo_url: user.cover_photo_url.clone(),
            created_at: user.created_at,
            attendance_information: Vec::new(),
        }
    );

    // Redeem tickets from order
    let items = cart.items(&connection).unwrap();
    let order_item = items
        .iter()
        .find(|i| i.ticket_type_id == Some(ticket_type.id))
        .unwrap();
    let tickets = TicketInstance::find_for_order_item(order_item.id, connection).unwrap();
    let ticket = &tickets[0];
    let ticket2 = &tickets[1];
    TicketInstance::redeem_ticket(
        ticket.id,
        ticket.redeem_key.clone().unwrap(),
        user.id,
        connection,
    )
    .unwrap();
    TicketInstance::redeem_ticket(
        ticket2.id,
        ticket2.redeem_key.clone().unwrap(),
        user.id,
        connection,
    )
    .unwrap();
    assert_eq!(
        user.get_profile_for_organization(&organization, connection)
            .unwrap(),
        FanProfile {
            first_name: user.first_name.clone(),
            last_name: user.last_name.clone(),
            email: user.email.clone(),
            facebook_linked: true,
            event_count: 1,
            revenue_in_cents: 1700,
            ticket_sales: 10,
            profile_pic_url: user.profile_pic_url.clone(),
            thumb_profile_pic_url: user.thumb_profile_pic_url.clone(),
            cover_photo_url: user.cover_photo_url.clone(),
            created_at: user.created_at,
            attendance_information: vec![AttendanceInformation {
                event_name: event.name.clone(),
                event_id: event.id,
                event_start: event.event_start
            }],
        }
    );

    // Checkout with a second order same event
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 1,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();
    assert_eq!(cart.calculate_total(connection).unwrap(), 170);
    cart.add_external_payment(Some("test".to_string()), user.id, 170, connection)
        .unwrap();
    assert_eq!(cart.status, OrderStatus::Paid);

    // Redeem a ticket from new cart
    let items = cart.items(&connection).unwrap();
    let order_item = items
        .iter()
        .find(|i| i.ticket_type_id == Some(ticket_type.id))
        .unwrap();
    let tickets = TicketInstance::find_for_order_item(order_item.id, connection).unwrap();
    let ticket = &tickets[0];
    TicketInstance::redeem_ticket(
        ticket.id,
        ticket.redeem_key.clone().unwrap(),
        user.id,
        connection,
    )
    .unwrap();

    assert_eq!(
        user.get_profile_for_organization(&organization, connection)
            .unwrap(),
        FanProfile {
            first_name: user.first_name.clone(),
            last_name: user.last_name.clone(),
            email: user.email.clone(),
            facebook_linked: true,
            event_count: 1,
            revenue_in_cents: 1870,
            ticket_sales: 11,
            profile_pic_url: user.profile_pic_url.clone(),
            thumb_profile_pic_url: user.thumb_profile_pic_url.clone(),
            cover_photo_url: user.cover_photo_url.clone(),
            created_at: user.created_at,
            attendance_information: vec![AttendanceInformation {
                event_name: event.name.clone(),
                event_id: event.id,
                event_start: event.event_start
            }],
        }
    );

    // Checkout with new event increasing event count as well
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type2.id,
            quantity: 1,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();
    assert_eq!(cart.calculate_total(connection).unwrap(), 170);
    cart.add_external_payment(Some("test".to_string()), user.id, 170, connection)
        .unwrap();
    assert_eq!(cart.status, OrderStatus::Paid);
    assert_eq!(
        user.get_profile_for_organization(&organization, connection)
            .unwrap(),
        FanProfile {
            first_name: user.first_name.clone(),
            last_name: user.last_name.clone(),
            email: user.email.clone(),
            facebook_linked: true,
            event_count: 2,
            revenue_in_cents: 2040,
            ticket_sales: 12,
            profile_pic_url: user.profile_pic_url.clone(),
            thumb_profile_pic_url: user.thumb_profile_pic_url.clone(),
            cover_photo_url: user.cover_photo_url.clone(),
            created_at: user.created_at,
            attendance_information: vec![AttendanceInformation {
                event_name: event.name.clone(),
                event_id: event.id,
                event_start: event.event_start
            }],
        }
    );

    // Redeem ticket from new event
    let items = cart.items(&connection).unwrap();
    let order_item = items
        .iter()
        .find(|i| i.ticket_type_id == Some(ticket_type2.id))
        .unwrap();
    let tickets = TicketInstance::find_for_order_item(order_item.id, connection).unwrap();
    let ticket = &tickets[0];

    // Transfer ticket to different user removing it from attendance information
    let sender_wallet = Wallet::find_default_for_user(user.id, connection).unwrap();
    let receiver_wallet = Wallet::find_default_for_user(user2.id, connection).unwrap();
    let transfer_auth =
        TicketInstance::authorize_ticket_transfer(user.id, &vec![ticket.id], 3600, connection)
            .unwrap();
    TicketInstance::receive_ticket_transfer(
        transfer_auth,
        &sender_wallet,
        receiver_wallet.id,
        connection,
    )
    .unwrap();

    TicketInstance::redeem_ticket(
        ticket.id,
        ticket.redeem_key.clone().unwrap(),
        user.id,
        connection,
    )
    .unwrap();
    assert_eq!(
        user.get_profile_for_organization(&organization, connection)
            .unwrap(),
        FanProfile {
            first_name: user.first_name.clone(),
            last_name: user.last_name.clone(),
            email: user.email.clone(),
            facebook_linked: true,
            event_count: 2,
            revenue_in_cents: 2040,
            ticket_sales: 12,
            profile_pic_url: user.profile_pic_url.clone(),
            thumb_profile_pic_url: user.thumb_profile_pic_url.clone(),
            cover_photo_url: user.cover_photo_url.clone(),
            created_at: user.created_at,
            attendance_information: vec![AttendanceInformation {
                event_name: event.name.clone(),
                event_id: event.id,
                event_start: event.event_start
            }],
        }
    );

    // Purchase and redeem from other event without transferring
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type2.id,
            quantity: 1,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();
    assert_eq!(cart.calculate_total(connection).unwrap(), 170);
    cart.add_external_payment(Some("test".to_string()), user.id, 170, connection)
        .unwrap();
    assert_eq!(cart.status, OrderStatus::Paid);

    let items = cart.items(&connection).unwrap();
    let order_item = items
        .iter()
        .find(|i| i.ticket_type_id == Some(ticket_type2.id))
        .unwrap();
    let tickets = TicketInstance::find_for_order_item(order_item.id, connection).unwrap();
    let ticket = &tickets[0];
    TicketInstance::redeem_ticket(
        ticket.id,
        ticket.redeem_key.clone().unwrap(),
        user.id,
        connection,
    )
    .unwrap();

    assert_eq!(
        user.get_profile_for_organization(&organization, connection)
            .unwrap(),
        FanProfile {
            first_name: user.first_name.clone(),
            last_name: user.last_name.clone(),
            email: user.email.clone(),
            facebook_linked: true,
            event_count: 2,
            revenue_in_cents: 2210,
            ticket_sales: 13,
            profile_pic_url: user.profile_pic_url.clone(),
            thumb_profile_pic_url: user.thumb_profile_pic_url.clone(),
            cover_photo_url: user.cover_photo_url.clone(),
            created_at: user.created_at,
            attendance_information: vec![
                AttendanceInformation {
                    event_name: event.name.clone(),
                    event_id: event.id,
                    event_start: event.event_start
                },
                AttendanceInformation {
                    event_name: event2.name.clone(),
                    event_id: event2.id,
                    event_start: event2.event_start
                }
            ],
        }
    );

    // Purchased by other user and transferred
    let mut cart = Order::find_or_create_cart(&user2, connection).unwrap();
    cart.update_quantities(
        user2.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type3.id,
            quantity: 1,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();
    assert_eq!(cart.calculate_total(connection).unwrap(), 170);
    cart.add_external_payment(Some("test".to_string()), user2.id, 170, connection)
        .unwrap();
    assert_eq!(cart.status, OrderStatus::Paid);

    // Redeem ticket from new event
    let items = cart.items(&connection).unwrap();
    let order_item = items
        .iter()
        .find(|i| i.ticket_type_id == Some(ticket_type3.id))
        .unwrap();
    let tickets = TicketInstance::find_for_order_item(order_item.id, connection).unwrap();
    let ticket = &tickets[0];

    let sender_wallet = Wallet::find_default_for_user(user2.id, connection).unwrap();
    let receiver_wallet = Wallet::find_default_for_user(user.id, connection).unwrap();
    let transfer_auth =
        TicketInstance::authorize_ticket_transfer(user2.id, &vec![ticket.id], 3600, connection)
            .unwrap();
    TicketInstance::receive_ticket_transfer(
        transfer_auth,
        &sender_wallet,
        receiver_wallet.id,
        connection,
    )
    .unwrap();

    TicketInstance::redeem_ticket(
        ticket.id,
        ticket.redeem_key.clone().unwrap(),
        user.id,
        connection,
    )
    .unwrap();
    assert_eq!(
        user.get_profile_for_organization(&organization, connection)
            .unwrap(),
        FanProfile {
            first_name: user.first_name.clone(),
            last_name: user.last_name.clone(),
            email: user.email.clone(),
            facebook_linked: true,
            event_count: 2,
            revenue_in_cents: 2210,
            ticket_sales: 13,
            profile_pic_url: user.profile_pic_url.clone(),
            thumb_profile_pic_url: user.thumb_profile_pic_url.clone(),
            cover_photo_url: user.cover_photo_url.clone(),
            created_at: user.created_at,
            attendance_information: vec![
                AttendanceInformation {
                    event_name: event.name.clone(),
                    event_id: event.id,
                    event_start: event.event_start
                },
                AttendanceInformation {
                    event_name: event2.name.clone(),
                    event_id: event2.id,
                    event_start: event2.event_start
                },
                AttendanceInformation {
                    event_name: event3.name.clone(),
                    event_id: event3.id,
                    event_start: event3.event_start
                }
            ],
        }
    );
}

#[test]
fn get_history_for_organization() {
    let project = TestProject::new();
    let admin = project.create_user().finish();

    let connection = project.get_connection();
    let user = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_fee_schedule(&project.create_fee_schedule().finish(admin.id))
        .finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];

    // No history to date
    assert!(user
        .get_history_for_organization(&organization, 0, 100, SortingDir::Desc, connection)
        .unwrap()
        .is_empty());

    // User adds item to cart but does not checkout so no history
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
    assert!(user
        .get_history_for_organization(&organization, 0, 100, SortingDir::Desc, connection)
        .unwrap()
        .is_empty());

    // User checks out so has a paid order so history exists
    assert_eq!(cart.calculate_total(connection).unwrap(), 1700);
    cart.add_external_payment(Some("test".to_string()), user.id, 1700, connection)
        .unwrap();
    assert_eq!(cart.status, OrderStatus::Paid);

    let mut paging = Paging::new(0, 100);
    paging.dir = SortingDir::Desc;
    let mut payload = Payload::new(
        vec![HistoryItem::Purchase {
            order_id: cart.id,
            order_date: cart.order_date,
            event_name: event.name.clone(),
            ticket_sales: 10,
            revenue_in_cents: 1700,
        }],
        paging,
    );
    payload.paging.total = 1;
    assert_eq!(
        user.get_history_for_organization(&organization, 0, 100, SortingDir::Desc, connection)
            .unwrap(),
        payload
    );

    // User makes a second order
    let mut cart2 = Order::find_or_create_cart(&user, connection).unwrap();
    cart2
        .update_quantities(
            user.id,
            &[UpdateOrderItem {
                ticket_type_id: ticket_type.id,
                quantity: 1,
                redemption_code: None,
            }],
            false,
            false,
            connection,
        )
        .unwrap();

    // Update cart2 to a future date to avoid test timing errors
    let mut cart2 = diesel::update(orders::table.filter(orders::id.eq(cart2.id)))
        .set(orders::order_date.eq(Utc::now().naive_utc() + Duration::seconds(1)))
        .get_result::<Order>(connection)
        .unwrap();

    assert_eq!(cart2.calculate_total(connection).unwrap(), 170);
    cart2
        .add_external_payment(Some("test".to_string()), user.id, 170, connection)
        .unwrap();
    assert_eq!(cart2.status, OrderStatus::Paid);

    let mut paging = Paging::new(0, 100);
    paging.dir = SortingDir::Desc;
    let mut payload = Payload::new(
        vec![
            HistoryItem::Purchase {
                order_id: cart2.id,
                order_date: cart2.order_date,
                event_name: event.name.clone(),
                ticket_sales: 1,
                revenue_in_cents: 170,
            },
            HistoryItem::Purchase {
                order_id: cart.id,
                order_date: cart.order_date,
                event_name: event.name.clone(),
                ticket_sales: 10,
                revenue_in_cents: 1700,
            },
        ],
        paging,
    );
    payload.paging.total = 2;
    assert_eq!(
        user.get_history_for_organization(&organization, 0, 100, SortingDir::Desc, connection)
            .unwrap(),
        payload
    );
}

#[test]
fn find() {
    let project = TestProject::new();
    let user = project.create_user().finish();

    let found_user = User::find(user.id, project.get_connection()).expect("User was not found");
    assert_eq!(found_user.id, user.id);
    assert_eq!(found_user.email, user.email);

    assert!(
        match User::find(Uuid::new_v4(), project.get_connection()) {
            Ok(_user) => false,
            Err(_e) => true,
        },
        "User incorrectly returned when id invalid"
    );
}

#[test]
fn get_event_ids_by_organization() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();

    // No results
    assert_eq!(
        HashMap::new(),
        user.get_event_ids_by_organization(connection).unwrap()
    );

    let organization = project
        .create_organization()
        .with_name("Organization1".into())
        .finish();
    let organization2 = project
        .create_organization()
        .with_name("Organization2".into())
        .finish();
    let organization3 = project
        .create_organization()
        .with_name("Organization3".into())
        .with_member(&user, Roles::OrgAdmin)
        .finish();

    let event = project
        .create_event()
        .with_organization(&organization)
        .finish();
    let event2 = project
        .create_event()
        .with_organization(&organization2)
        .finish();
    let event3 = project
        .create_event()
        .with_organization(&organization2)
        .finish();

    OrganizationUser::create(
        organization.id,
        user.id,
        vec![Roles::Promoter],
        vec![event.id],
    )
    .commit(connection)
    .unwrap();
    OrganizationUser::create(
        organization2.id,
        user.id,
        vec![Roles::Promoter],
        vec![event2.id, event3.id],
    )
    .commit(connection)
    .unwrap();

    let mut expected_results = HashMap::new();
    expected_results.insert(organization.id.clone(), vec![event.id]);
    expected_results.insert(organization2.id.clone(), vec![event2.id, event3.id]);
    expected_results.insert(organization3.id.clone(), Vec::new());

    assert_eq!(
        expected_results,
        user.get_event_ids_by_organization(connection).unwrap()
    );

    // get_event_ids_for_organization
    assert_eq!(
        vec![event.id],
        user.get_event_ids_for_organization(organization.id, connection)
            .unwrap()
    );
    assert_eq!(
        vec![event2.id, event3.id],
        user.get_event_ids_for_organization(organization2.id, connection)
            .unwrap()
    );
    assert!(user
        .get_event_ids_for_organization(organization3.id, connection)
        .unwrap()
        .is_empty());
}

#[test]
fn payment_method() {
    let project = TestProject::new();
    let user = project.create_user().finish();
    assert!(user
        .payment_method(PaymentProviders::External, project.get_connection())
        .is_err());

    let payment_method = project
        .create_payment_method()
        .with_name(PaymentProviders::External)
        .with_user(&user)
        .finish();
    assert_eq!(
        payment_method,
        user.payment_method(payment_method.name.clone(), project.get_connection())
            .unwrap(),
    );
}

#[test]
fn default_payment_method() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();

    // No payment methods set
    assert!(user.default_payment_method(connection).is_err());

    // Payment method exists but not default
    project
        .create_payment_method()
        .with_name(PaymentProviders::External)
        .with_user(&user)
        .finish();
    assert!(user.default_payment_method(connection).is_err());

    // Default set
    let payment_method2 = project
        .create_payment_method()
        .with_name(PaymentProviders::Stripe)
        .with_user(&user)
        .make_default()
        .finish();
    let default_payment_method = user.default_payment_method(connection).unwrap();
    assert_eq!(payment_method2, default_payment_method);
}

#[test]
fn payment_methods() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    assert!(user.payment_methods(connection).unwrap().is_empty());

    let payment_method = project
        .create_payment_method()
        .with_name(PaymentProviders::External)
        .with_user(&user)
        .finish();
    assert_eq!(
        vec![payment_method.clone()],
        user.payment_methods(connection).unwrap(),
    );

    let payment_method2 = project
        .create_payment_method()
        .with_name(PaymentProviders::Stripe)
        .with_user(&user)
        .finish();
    assert_eq!(
        vec![payment_method, payment_method2],
        user.payment_methods(connection).unwrap(),
    );
}

#[test]
fn full_name() {
    let project = TestProject::new();

    let first_name = "Bob".to_string();
    let last_name = "Jones".to_string();

    let user = project
        .create_user()
        .with_first_name(&first_name)
        .with_last_name(&last_name)
        .finish();
    assert_eq!(user.full_name(), format!("{} {}", first_name, last_name));
}

#[test]
fn find_by_email() {
    let project = TestProject::new();
    let user = project.create_user().finish();

    let found_user = User::find_by_email(&user.email.clone().unwrap(), project.get_connection())
        .expect("User was not found");
    assert_eq!(found_user, user);

    let not_found = User::find_by_email("not@real.com", project.get_connection());
    let error = not_found.unwrap_err();
    assert_eq!(
        error.to_string(),
        "[2000] No results\nCaused by: Error loading user, NotFound"
    );
}

#[test]
fn update() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let mut attributes: UserEditableAttributes = Default::default();
    let email = "new_email@tari.com";
    attributes.email = Some(Some(email.to_string()));

    let updated_user = user.update(&attributes.into(), connection).unwrap();
    assert_eq!(updated_user.email, Some(email.into()));
}

#[test]
fn new_user_validate() {
    let email = "abc";
    let user = User::create(
        Some("First".to_string()),
        Some("Last".to_string()),
        Some(email.to_string()),
        Some("123".to_string()),
        &"Password",
    );
    let result = user.validate();
    assert!(result.is_err());
    let errors = result.unwrap_err().field_errors();

    assert!(errors.contains_key("email"));
    assert_eq!(errors["email"].len(), 1);
    assert_eq!(errors["email"][0].code, "email");
    assert_eq!(
        &errors["email"][0].message.clone().unwrap().into_owned(),
        "Email is invalid"
    );
}

#[test]
fn user_editable_attributes_validate() {
    let mut user_parameters: UserEditableAttributes = Default::default();
    user_parameters.email = Some(Some("abc".into()));

    let result = user_parameters.validate();
    assert!(result.is_err());
    let errors = result.unwrap_err().field_errors();

    assert!(errors.contains_key("email"));
    assert_eq!(errors["email"].len(), 1);
    assert_eq!(errors["email"][0].code, "email");
    assert_eq!(
        &errors["email"][0].message.clone().unwrap().into_owned(),
        "Email is invalid"
    );
}

#[test]
fn create_from_external_login() {
    let project = TestProject::new();
    let external_id = "123";
    let first_name = "Dennis";
    let last_name = "Miguel";
    let email = "dennis@tari.com";
    let site = "facebook.com";
    let access_token = "abc-123";

    let user = User::create_from_external_login(
        external_id.to_string(),
        first_name.to_string(),
        last_name.to_string(),
        email.to_string(),
        site.to_string(),
        access_token.to_string(),
        project.get_connection(),
    )
    .unwrap();

    let external_login = ExternalLogin::find_user(external_id, site, project.get_connection())
        .unwrap()
        .unwrap();

    assert_eq!(user.id, external_login.user_id);
    assert_eq!(access_token, external_login.access_token);
    assert_eq!(site, external_login.site);
    assert_eq!(external_id, external_login.external_user_id);

    assert_eq!(Some(email.to_string()), user.email);
    assert_eq!(first_name, user.first_name.unwrap_or("".to_string()));
    assert_eq!(last_name, user.last_name.unwrap_or("".to_string()));
}

#[test]
fn for_display() {
    let project = TestProject::new();
    let user = project.create_user().finish();
    let user_id = user.id.clone();
    let display_user = user.for_display().unwrap();

    assert_eq!(display_user.id, user_id);
}

#[test]
fn organizations() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();

    let organization = project
        .create_organization()
        .with_name("Organization1".into())
        .with_member(&user, Roles::OrgOwner)
        .finish();
    let organization2 = project
        .create_organization()
        .with_name("Organization2".into())
        .with_member(&user, Roles::OrgMember)
        .finish();
    let _organization3 = project
        .create_organization()
        .with_name("Organization3".into())
        .finish();

    assert_eq!(
        vec![organization, organization2],
        user.organizations(connection).unwrap()
    );
}

#[test]
fn find_events_with_access_to_scan() {
    //create event
    let project = TestProject::new();
    let connection = project.get_connection();
    let venue = project.create_venue().finish();

    let owner = project.create_user().finish();
    let scanner = project.create_user().finish();
    let _normal_user = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_member(&owner, Roles::OrgOwner)
        .with_member(&scanner, Roles::OrgMember)
        .finish();
    let _draft_event = project
        .create_event()
        .with_status(EventStatus::Draft)
        .with_event_start(Utc::now().naive_utc())
        .with_name("DraftEvent".into())
        .with_organization(&organization)
        .with_venue(&venue)
        .finish();
    let published_event = project
        .create_event()
        .with_status(EventStatus::Published)
        .with_event_start(Utc::now().naive_utc())
        .with_name("PublishedEvent".into())
        .with_organization(&organization)
        .with_venue(&venue)
        .finish();
    let _published_external_event = project
        .create_event()
        .with_status(EventStatus::Published)
        .external()
        .with_event_start(Utc::now().naive_utc())
        .with_name("PublishedExternalEvent".into())
        .with_organization(&organization)
        .with_venue(&venue)
        .finish();

    let owner_events = owner.find_events_with_access_to_scan(connection).unwrap();
    let scanner_events = scanner.find_events_with_access_to_scan(connection).unwrap();
    let normal_user_events = _normal_user
        .find_events_with_access_to_scan(connection)
        .unwrap();

    assert_eq!(owner_events, vec![published_event.clone()]);
    assert_eq!(scanner_events, vec![published_event]);
    assert!(normal_user_events.is_empty());
}

#[test]
fn get_roles_by_organization() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();

    let organization = project
        .create_organization()
        .with_name("Organization1".into())
        .with_member(&user, Roles::OrgOwner)
        .finish();
    let organization2 = project
        .create_organization()
        .with_name("Organization2".into())
        .with_member(&user, Roles::OrgMember)
        .finish();
    let _organization3 = project
        .create_organization()
        .with_name("Organization3".into())
        .finish();

    let mut expected_results = HashMap::new();
    expected_results.insert(organization.id.clone(), vec![Roles::OrgOwner]);
    expected_results.insert(organization2.id.clone(), vec![Roles::OrgMember]);

    assert_eq!(
        user.get_roles_by_organization(connection).unwrap(),
        expected_results
    );
}

#[test]
fn get_scopes_by_organization() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();

    let organization = project
        .create_organization()
        .with_name("Organization1".into())
        .with_member(&user, Roles::OrgOwner)
        .finish();
    let organization2 = project
        .create_organization()
        .with_name("Organization2".into())
        .with_member(&user, Roles::OrgMember)
        .finish();
    let _organization3 = project
        .create_organization()
        .with_name("Organization3".into())
        .finish();

    let mut expected_results = HashMap::new();
    expected_results.insert(
        organization.id,
        vec![
            Scopes::ArtistWrite,
            Scopes::BoxOfficeTicketRead,
            Scopes::BoxOfficeTicketWrite,
            Scopes::CodeRead,
            Scopes::CodeWrite,
            Scopes::CompRead,
            Scopes::CompWrite,
            Scopes::DashboardRead,
            Scopes::EventCancel,
            Scopes::EventFinancialReports,
            Scopes::EventInterest,
            Scopes::EventReports,
            Scopes::EventScan,
            Scopes::EventViewGuests,
            Scopes::EventWrite,
            Scopes::HoldRead,
            Scopes::HoldWrite,
            Scopes::OrderMakeExternalPayment,
            Scopes::OrderRead,
            Scopes::OrderReadOwn,
            Scopes::OrderRefund,
            Scopes::OrgAdminUsers,
            Scopes::OrgFans,
            Scopes::OrgRead,
            Scopes::OrgReports,
            Scopes::OrgUsers,
            Scopes::OrgWrite,
            Scopes::RedeemTicket,
            Scopes::TicketAdmin,
            Scopes::TicketRead,
            Scopes::TicketTransfer,
            Scopes::TicketTypeRead,
            Scopes::TicketTypeWrite,
            Scopes::UserRead,
            Scopes::VenueWrite,
        ],
    );
    expected_results.insert(
        organization2.id,
        vec![
            Scopes::ArtistWrite,
            Scopes::BoxOfficeTicketRead,
            Scopes::BoxOfficeTicketWrite,
            Scopes::CodeRead,
            Scopes::CodeWrite,
            Scopes::CompRead,
            Scopes::CompWrite,
            Scopes::DashboardRead,
            Scopes::EventCancel,
            Scopes::EventInterest,
            Scopes::EventScan,
            Scopes::EventViewGuests,
            Scopes::EventWrite,
            Scopes::HoldRead,
            Scopes::HoldWrite,
            Scopes::OrderRead,
            Scopes::OrderReadOwn,
            Scopes::OrderRefund,
            Scopes::OrgFans,
            Scopes::OrgRead,
            Scopes::RedeemTicket,
            Scopes::TicketAdmin,
            Scopes::TicketRead,
            Scopes::TicketTransfer,
            Scopes::TicketTypeRead,
            Scopes::TicketTypeWrite,
            Scopes::VenueWrite,
        ],
    );

    assert_eq!(
        user.get_scopes_by_organization(connection).unwrap(),
        expected_results
    );
}

#[test]
fn get_global_scopes() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let user2 = project.create_user().finish();
    let mut user3 = project.create_user().finish();
    let _organization = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .with_member(&user2, Roles::OrgMember)
        .finish();
    user3 = user3.add_role(Roles::Admin, connection).unwrap();

    assert_eq!(
        user.get_global_scopes()
            .into_iter()
            .map(|scope| scope.to_string())
            .collect::<Vec<String>>(),
        vec!["event:interest", "order:read-own", "ticket:transfer"]
    );
    assert_eq!(
        user2
            .get_global_scopes()
            .into_iter()
            .map(|scope| scope.to_string())
            .collect::<Vec<String>>(),
        vec!["event:interest", "order:read-own", "ticket:transfer"]
    );
    assert_eq!(
        user3
            .get_global_scopes()
            .into_iter()
            .map(|scope| scope.to_string())
            .collect::<Vec<String>>(),
        vec![
            "artist:write",
            "box-office-ticket:read",
            "box-office-ticket:write",
            "code:read",
            "code:write",
            "comp:read",
            "comp:write",
            "dashboard:read",
            "event:cancel",
            "event:financial-reports",
            "event:interest",
            "event:reports",
            "event:scan",
            "event:view-guests",
            "event:write",
            "hold:read",
            "hold:write",
            "order:make-external-payment",
            "order:read",
            "order:read-own",
            "order:refund",
            "org:admin",
            "org:admin-users",
            "org:fans",
            "org:financial-reports",
            "org:read",
            "org:reports",
            "org:users",
            "org:write",
            "redeem:ticket",
            "region:write",
            "ticket:admin",
            "ticket:read",
            "ticket:transfer",
            "ticket-type:read",
            "ticket-type:write",
            "user:read",
            "venue:write"
        ]
    );
}

#[test]
fn add_role() {
    let project = TestProject::new();
    let user = project.create_user().finish();

    user.add_role(Roles::Admin, project.get_connection())
        .unwrap();
    //Try adding a duplicate role to check that it isnt duplicated.
    user.add_role(Roles::Admin, project.get_connection())
        .unwrap();

    let user2 = User::find(user.id, project.get_connection()).unwrap();
    assert_eq!(user2.role, vec![Roles::User, Roles::Admin]);
}
