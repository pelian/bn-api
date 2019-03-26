use bigneon_db::dev::TestProject;
use bigneon_db::models::*;

#[test]
fn log_refund() {
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
    let payment = project
        .create_payment()
        .with_user(&user)
        .with_organization(&organization)
        .with_event(&event)
        .finish();
    assert!(
        payment.log_refund(user.id, 100, None, connection).is_ok(),
        true
    )
}

#[test]
fn find_by_order() {
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
    let payment = project
        .create_payment()
        .with_user(&user)
        .with_organization(&organization)
        .with_event(&event)
        .finish();
    let found_payment =
        Payment::find_by_order(payment.order_id, &"Test".to_string(), connection).unwrap();
    assert_eq!(payment, found_payment);
}

#[test]
fn find() {
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
    let payment = project
        .create_payment()
        .with_user(&user)
        .with_organization(&organization)
        .with_event(&event)
        .finish();
    let found_payment = Payment::find(payment.id, connection).unwrap();
    assert_eq!(payment, found_payment);
}

#[test]
fn find_all_with_orders_paginated_by_provider() {
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
    let payment = project
        .create_payment()
        .with_user(&user)
        .with_organization(&organization)
        .with_event(&event)
        .finish();
    let found_payment = Payment::find_all_with_orders_paginated_by_provider(
        PaymentProviders::External,
        0,
        10,
        connection,
    )
    .unwrap();
    assert_eq!(payment, found_payment[0].0);
}

#[test]
fn add_ipn() {
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
    let payment = project
        .create_payment()
        .with_user(&user)
        .with_organization(&organization)
        .with_event(&event)
        .finish();
    assert!(
        payment
            .add_ipn(
                PaymentStatus::Cancelled,
                json!(null),
                Some(user.id),
                connection
            )
            .is_ok(),
        true
    )
}

#[test]
fn update_amount() {
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
    let payment = project
        .create_payment()
        .with_user(&user)
        .with_organization(&organization)
        .with_event(&event)
        .finish();
    assert!(
        payment
            .update_amount(Some(user.id), 100, connection)
            .is_ok(),
        true
    )
}

#[test]
fn mark_complete() {
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
    let payment = project
        .create_payment()
        .with_user(&user)
        .with_organization(&organization)
        .with_event(&event)
        .finish();
    assert!(
        payment
            .mark_complete(json!(null), Some(user.id), connection)
            .is_ok(),
        true
    )
}

#[test]
fn mark_pending_ipn() {
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
    let payment = project
        .create_payment()
        .with_user(&user)
        .with_organization(&organization)
        .with_event(&event)
        .finish();
    assert!(
        payment.mark_pending_ipn(Some(user.id), connection).is_ok(),
        true
    )
}

#[test]
fn mark_cancelled() {
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
    let payment = project
        .create_payment()
        .with_user(&user)
        .with_organization(&organization)
        .with_event(&event)
        .with_status(PaymentStatus::Draft)
        .finish();
    assert!(
        payment
            .mark_cancelled(json!(null), Some(user.id), connection)
            .is_ok(),
        true
    )
}
