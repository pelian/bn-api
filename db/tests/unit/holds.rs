use bigneon_db::dev::TestProject;
use bigneon_db::models::*;
use bigneon_db::utils::errors::ErrorCode::ValidationError;
use chrono::prelude::*;
use time::Duration;
use uuid::Uuid;

#[test]
fn create() {
    let db = TestProject::new();
    let event = db.create_event().with_tickets().finish();
    Hold::create_hold(
        "test".to_string(),
        event.id,
        Some("IHAVEACODE".to_string()),
        Some(0),
        None,
        Some(4),
        HoldTypes::Discount,
        event.ticket_types(true, None, db.get_connection()).unwrap()[0].id,
    )
    .commit(None, db.get_connection())
    .unwrap();
}

#[test]
fn create_with_validation_errors() {
    let db = TestProject::new();
    let event = db.create_event().with_tickets().finish();
    let result = Hold::create_hold(
        "test".to_string(),
        event.id,
        Some("IHAVEACODE".to_string()),
        None,
        None,
        Some(4),
        HoldTypes::Discount,
        event.ticket_types(true, None, db.get_connection()).unwrap()[0].id,
    )
    .commit(None, db.get_connection());

    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("discount_in_cents"));
                assert_eq!(errors["discount_in_cents"].len(), 1);
                assert_eq!(errors["discount_in_cents"][0].code, "required");
            }
            _ => panic!("Expected validation error"),
        },
    }

    // Dupe redemption code
    let hold = db.create_hold().finish();
    let result = Hold::create_hold(
        "test".to_string(),
        event.id,
        hold.redemption_code,
        Some(0),
        None,
        Some(4),
        HoldTypes::Discount,
        event.ticket_types(true, None, db.get_connection()).unwrap()[0].id,
    )
    .commit(None, db.get_connection());
    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("redemption_code"));
                assert_eq!(errors["redemption_code"].len(), 1);
                assert_eq!(errors["redemption_code"][0].code, "uniqueness");
            }
            _ => panic!("Expected validation error"),
        },
    }

    // Redemption code used by a code
    let code = db.create_code().finish();
    let result = Hold::create_hold(
        "test".to_string(),
        event.id,
        Some(code.redemption_code),
        Some(0),
        None,
        Some(4),
        HoldTypes::Discount,
        event.ticket_types(true, None, db.get_connection()).unwrap()[0].id,
    )
    .commit(None, db.get_connection());
    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("redemption_code"));
                assert_eq!(errors["redemption_code"].len(), 1);
                assert_eq!(errors["redemption_code"][0].code, "uniqueness");
            }
            _ => panic!("Expected validation error"),
        },
    }
}

#[test]
pub fn confirm_hold_valid() {
    let db = TestProject::new();
    let hold = db.create_hold().finish();
    assert!(hold.confirm_hold_valid().is_ok());

    let end_at = NaiveDateTime::from(Utc::now().naive_utc() - Duration::days(2));
    let hold = db.create_hold().with_end_at(end_at).finish();
    let result = hold.confirm_hold_valid();

    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("hold_id"));
                assert_eq!(errors["hold_id"].len(), 1);
                assert_eq!(errors["hold_id"][0].code, "invalid");
                assert_eq!(
                    &errors["hold_id"][0].message.clone().unwrap().into_owned(),
                    "Hold not valid for current datetime"
                );
            }
            _ => panic!("Expected validation error"),
        },
    }
}

#[test]
fn update() {
    let db = TestProject::new();
    let hold = db.create_hold().finish();

    let update_patch = UpdateHoldAttributes {
        discount_in_cents: Some(Some(10)),
        max_per_user: Some(None),
        end_at: Some(None),
        name: Some("New name".to_string()),
        ..Default::default()
    };
    let new_hold = hold.update(update_patch, db.get_connection()).unwrap();
    assert_eq!(new_hold.name, "New name".to_string());
    assert_eq!(new_hold.max_per_user, None);
    assert_eq!(new_hold.end_at, None);
    assert_eq!(new_hold.discount_in_cents, Some(10));
}

#[test]
fn update_with_validation_errors() {
    let project = TestProject::new();
    let creator = project.create_user().finish();

    let connection = project.get_connection();
    let hold = project
        .create_hold()
        .with_hold_type(HoldTypes::Comp)
        .finish();
    assert!(hold.discount_in_cents.is_none());

    let update_patch = UpdateHoldAttributes {
        hold_type: Some(HoldTypes::Discount),
        ..Default::default()
    };
    let result = hold.update(update_patch, connection);
    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("discount_in_cents"));
                assert_eq!(errors["discount_in_cents"].len(), 1);
                assert_eq!(errors["discount_in_cents"][0].code, "required");
            }
            _ => panic!("Expected validation error"),
        },
    }

    // Dupe redemption code
    let hold2 = project.create_hold().finish();
    let update_patch = UpdateHoldAttributes {
        redemption_code: Some(hold2.redemption_code),
        ..Default::default()
    };
    let result = hold.update(update_patch, connection);
    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("redemption_code"));
                assert_eq!(errors["redemption_code"].len(), 1);
                assert_eq!(errors["redemption_code"][0].code, "uniqueness");
            }
            _ => panic!("Expected validation error"),
        },
    }

    // Dupe redemption code used by code
    let code = project.create_code().finish();
    let update_patch = UpdateHoldAttributes {
        redemption_code: Some(Some(code.redemption_code)),
        ..Default::default()
    };
    let result = hold.update(update_patch, connection);
    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("redemption_code"));
                assert_eq!(errors["redemption_code"].len(), 1);
                assert_eq!(errors["redemption_code"][0].code, "uniqueness");
            }
            _ => panic!("Expected validation error"),
        },
    }

    // Create and assign comp to active cart
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
    let hold = project.create_hold().with_event(&event).finish();
    let comp = project
        .create_comp()
        .with_quantity(10)
        .with_hold(&hold)
        .finish();

    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
    cart.update_quantities(
        user.id,
        &vec![UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 10,
            redemption_code: comp.redemption_code,
        }],
        false,
        false,
        connection,
    )
    .unwrap();
}

#[test]
fn find_by_ticket_type() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let hold = project.create_hold().finish();
    let ticket_type = TicketType::find(hold.ticket_type_id, connection).unwrap();

    assert_eq!(
        vec![hold],
        Hold::find_by_ticket_type(ticket_type.id, connection).unwrap()
    );
}

#[test]
fn remove_available_quantity() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let hold = project.create_hold().finish();

    let child_hold = Hold::create_comp_for_person(
        "Child".into(),
        None,
        hold.id,
        None,
        None,
        "ChildCode".into(),
        None,
        None,
        2,
        connection,
    )
    .unwrap();
    let ticket_type = TicketType::find(hold.ticket_type_id, connection).unwrap();

    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    cart.update_quantities(
        user.id,
        &vec![
            UpdateOrderItem {
                ticket_type_id: ticket_type.id,
                quantity: 4,
                redemption_code: hold.redemption_code.clone(),
            },
            UpdateOrderItem {
                ticket_type_id: ticket_type.id,
                quantity: 1,
                redemption_code: child_hold.redemption_code.clone(),
            },
        ],
        false,
        false,
        connection,
    )
    .unwrap();
    let total = cart.calculate_total(connection).unwrap();
    cart.add_external_payment(
        Some("test".to_string()),
        ExternalPaymentType::CreditCard,
        user.id,
        total,
        connection,
    )
    .unwrap();

    // Add additional cart item from existing unsold quantity (is removed from hold)
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    cart.update_quantities(
        user.id,
        &vec![UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 1,
            redemption_code: hold.redemption_code.clone(),
        }],
        false,
        false,
        connection,
    )
    .unwrap();

    assert_eq!(8, hold.quantity(connection).unwrap().0);
    assert_eq!(2, child_hold.quantity(connection).unwrap().0);

    hold.remove_available_quantity(None, connection).unwrap();
    assert_eq!(4, hold.quantity(connection).unwrap().0);
    assert_eq!(1, child_hold.quantity(connection).unwrap().0);
}

#[test]
fn destroy() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let hold = project.create_hold().finish();
    assert!(hold.clone().destroy(None, connection).is_ok());
    assert_eq!(
        Hold::find(hold.id, connection).unwrap().status,
        HoldStatus::Deleted
    );

    // Destroy hold with comps
    let comp = project.create_comp().finish();
    let hold = Hold::find(comp.id, connection).unwrap();
    hold.clone().destroy(None, connection).unwrap();
    assert_eq!(
        Hold::find(hold.id, connection).unwrap().status,
        HoldStatus::Deleted
    );
}

#[test]
fn set_quantity() {
    let db = TestProject::new();
    let event = db.create_event().with_tickets().finish();
    let hold = db.create_hold().with_event(&event).finish();
    hold.set_quantity(None, 30, db.get_connection()).unwrap();

    assert_eq!(hold.quantity(db.get_connection()).unwrap(), (30, 30));
}

#[test]
fn event() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project.create_event().with_ticket_pricing().finish();
    let hold = project.create_hold().with_event(&event).finish();

    assert_eq!(hold.event(connection).unwrap(), event);
}

#[test]
fn organization() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project.create_event().with_ticket_pricing().finish();
    let hold = project.create_hold().with_event(&event).finish();

    let organization = hold.organization(connection).unwrap();
    assert_eq!(event.organization(connection).unwrap(), organization);
}

#[test]
fn find() {
    let db = TestProject::new();
    let hold = db.create_hold().finish();

    let db_hold = Hold::find(hold.id, db.get_connection()).unwrap();

    assert_eq!(hold, db_hold);
}

#[test]
fn find_for_event() {
    let db = TestProject::new();
    let event = db.create_event().with_ticket_pricing().finish();
    let hold = db
        .create_hold()
        .with_name("Hold 1".to_string())
        .with_event(&event)
        .finish();
    let hold2 = db
        .create_hold()
        .with_name("Hold 2".to_string())
        .with_event(&event)
        .finish();

    let holds = Hold::find_for_event(event.id, db.get_connection()).unwrap();

    assert_eq!(vec![hold, hold2], holds);
}

#[test]
fn find_by_parent_hold_id() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let comp = project.create_comp().finish();

    // Record found
    let found_comp = Hold::find_by_parent_id(
        comp.parent_hold_id.unwrap(),
        HoldTypes::Comp,
        0,
        1000,
        connection,
    )
    .unwrap();
    assert_eq!(comp, found_comp.data[0]);

    // Comp does not exist for hold so returns error
    assert!(
        Hold::find_by_parent_id(Uuid::new_v4(), HoldTypes::Comp, 0, 1000, connection)
            .unwrap()
            .is_empty()
    );

    assert!(Hold::find_by_parent_id(
        comp.parent_hold_id.unwrap(),
        HoldTypes::Discount,
        0,
        1000,
        connection
    )
    .unwrap()
    .is_empty());
}
