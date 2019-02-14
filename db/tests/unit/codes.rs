use bigneon_db::dev::TestProject;
use bigneon_db::models::*;
use bigneon_db::utils::errors::ErrorCode::ValidationError;
use chrono::prelude::*;
use chrono::NaiveDateTime;
use time::Duration;
use uuid::Uuid;

#[test]
fn create() {
    let db = TestProject::new();
    let start_date = NaiveDateTime::from(Utc::now().naive_utc() - Duration::days(1));
    let end_date = NaiveDateTime::from(Utc::now().naive_utc() + Duration::days(2));
    let event = db.create_event().with_tickets().finish();
    Code::create(
        "test".into(),
        event.id,
        CodeTypes::Discount,
        "REDEMPTION".into(),
        10,
        Some(100),
        start_date,
        end_date,
        None,
    )
    .commit(db.get_connection())
    .unwrap();
}

#[test]
pub fn confirm_code_valid() {
    let db = TestProject::new();
    let code = db.create_code().finish();
    assert!(code.confirm_code_valid().is_ok());

    let start_date = NaiveDateTime::from(Utc::now().naive_utc() - Duration::days(3));
    let end_date = NaiveDateTime::from(Utc::now().naive_utc() - Duration::days(2));
    let code = db
        .create_code()
        .with_start_date(start_date)
        .with_end_date(end_date)
        .finish();
    assert!(code.confirm_code_valid().is_err());
}

#[test]
pub fn create_with_validation_errors() {
    let db = TestProject::new();
    let event = db.create_event().with_tickets().finish();
    let start_date = NaiveDateTime::from(Utc::now().naive_utc() + Duration::days(1));
    let end_date = NaiveDateTime::from(Utc::now().naive_utc() - Duration::days(2));
    let result = Code::create(
        "test".into(),
        event.id,
        CodeTypes::Discount,
        "A".into(),
        10,
        None,
        start_date,
        end_date,
        None,
    )
    .commit(db.get_connection());
    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("redemption_code"));
                assert_eq!(errors["redemption_code"].len(), 1);
                assert_eq!(errors["redemption_code"][0].code, "length");
                assert_eq!(
                    &errors["redemption_code"][0]
                        .message
                        .clone()
                        .unwrap()
                        .into_owned(),
                    "Redemption code must be at least 6 characters long"
                );

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

                assert!(errors.contains_key("discount_in_cents"));
                assert_eq!(errors["discount_in_cents"].len(), 1);
                assert_eq!(errors["discount_in_cents"][0].code, "required");
                assert_eq!(
                    &errors["discount_in_cents"][0]
                        .message
                        .clone()
                        .unwrap()
                        .into_owned(),
                    "Discount required for Discount code type"
                );
            }
            _ => panic!("Expected validation error"),
        },
    }

    // Dupe redemption code
    let code = db.create_code().finish();
    let start_date = NaiveDateTime::from(Utc::now().naive_utc() - Duration::days(1));
    let end_date = NaiveDateTime::from(Utc::now().naive_utc() + Duration::days(2));
    let result = Code::create(
        "test".into(),
        event.id,
        CodeTypes::Discount,
        code.redemption_code,
        10,
        Some(100),
        start_date,
        end_date,
        None,
    )
    .commit(db.get_connection());
    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("redemption_code"));
                assert_eq!(errors["redemption_code"].len(), 1);
                assert_eq!(errors["redemption_code"][0].code, "uniqueness");
                assert_eq!(
                    &errors["redemption_code"][0]
                        .message
                        .clone()
                        .unwrap()
                        .into_owned(),
                    "Redemption code must be unique"
                );
            }
            _ => panic!("Expected validation error"),
        },
    }

    // Redemption code used by a hold
    let hold = db.create_hold().finish();
    let start_date = NaiveDateTime::from(Utc::now().naive_utc() - Duration::days(1));
    let end_date = NaiveDateTime::from(Utc::now().naive_utc() + Duration::days(2));
    let result = Code::create(
        "test".into(),
        event.id,
        CodeTypes::Discount,
        hold.redemption_code,
        10,
        Some(100),
        start_date,
        end_date,
        None,
    )
    .commit(db.get_connection());
    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("redemption_code"));
                assert_eq!(errors["redemption_code"].len(), 1);
                assert_eq!(errors["redemption_code"][0].code, "uniqueness");
                assert_eq!(
                    &errors["redemption_code"][0]
                        .message
                        .clone()
                        .unwrap()
                        .into_owned(),
                    "Redemption code must be unique"
                );
            }
            _ => panic!("Expected validation error"),
        },
    }

    // Access code does not require a discount
    let start_date = NaiveDateTime::from(Utc::now().naive_utc() - Duration::days(1));
    let end_date = NaiveDateTime::from(Utc::now().naive_utc() + Duration::days(2));
    let result = Code::create(
        "test".into(),
        event.id,
        CodeTypes::Access,
        "NEWUNUSEDCODE".into(),
        10,
        None,
        start_date,
        end_date,
        None,
    )
    .commit(db.get_connection());
    assert!(result.is_ok());
}

#[test]
fn update() {
    let db = TestProject::new();
    let code = db.create_code().finish();

    let update_patch = UpdateCodeAttributes {
        name: Some("New name".into()),
        ..Default::default()
    };
    let new_code = code.update(update_patch, db.get_connection()).unwrap();
    assert_eq!(new_code.name, "New name".to_string());
}

#[test]
pub fn update_with_validation_errors() {
    let db = TestProject::new();
    let code = db.create_code().finish();
    let start_date = NaiveDateTime::from(Utc::now().naive_utc() + Duration::days(1));
    let end_date = NaiveDateTime::from(Utc::now().naive_utc() - Duration::days(2));

    let update_patch = UpdateCodeAttributes {
        redemption_code: Some("a".into()),
        start_date: Some(start_date),
        end_date: Some(end_date),
        discount_in_cents: Some(None),
        ..Default::default()
    };
    let result = code.update(update_patch, db.get_connection());
    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("redemption_code"));
                assert_eq!(errors["redemption_code"].len(), 1);
                assert_eq!(errors["redemption_code"][0].code, "length");
                assert_eq!(
                    &errors["redemption_code"][0]
                        .message
                        .clone()
                        .unwrap()
                        .into_owned(),
                    "Redemption code must be at least 6 characters long"
                );

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

                assert!(errors.contains_key("discount_in_cents"));
                assert_eq!(errors["discount_in_cents"].len(), 1);
                assert_eq!(errors["discount_in_cents"][0].code, "required");
                assert_eq!(
                    &errors["discount_in_cents"][0]
                        .message
                        .clone()
                        .unwrap()
                        .into_owned(),
                    "Discount required for Discount code type"
                );
            }
            _ => panic!("Expected validation error"),
        },
    }

    // Dupe redemption code
    let code2 = db.create_code().finish();
    let update_patch = UpdateCodeAttributes {
        redemption_code: Some(code2.redemption_code),
        ..Default::default()
    };
    let result = code.update(update_patch, db.get_connection());
    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("redemption_code"));
                assert_eq!(errors["redemption_code"].len(), 1);
                assert_eq!(errors["redemption_code"][0].code, "uniqueness");
                assert_eq!(
                    &errors["redemption_code"][0]
                        .message
                        .clone()
                        .unwrap()
                        .into_owned(),
                    "Redemption code must be unique"
                );
            }
            _ => panic!("Expected validation error"),
        },
    }

    // Dupe redemption code used by hold
    let hold = db.create_hold().finish();
    let update_patch = UpdateCodeAttributes {
        redemption_code: Some(hold.redemption_code),
        ..Default::default()
    };
    let result = code.update(update_patch, db.get_connection());
    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("redemption_code"));
                assert_eq!(errors["redemption_code"].len(), 1);
                assert_eq!(errors["redemption_code"][0].code, "uniqueness");
                assert_eq!(
                    &errors["redemption_code"][0]
                        .message
                        .clone()
                        .unwrap()
                        .into_owned(),
                    "Redemption code must be unique"
                );
            }
            _ => panic!("Expected validation error"),
        },
    }
}

#[test]
fn find_by_redemption_code() {
    let db = TestProject::new();
    let connection = db.get_connection();
    let code = db.create_code().finish();
    let found_code = Code::find_by_redemption_code(&code.redemption_code, connection).unwrap();
    assert_eq!(code, found_code);
}

#[test]
fn find() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let code = project.create_code().finish();

    // Record found
    let found_code = Code::find(code.id, connection).unwrap();
    assert_eq!(code, found_code);

    // Code does not exist so returns error
    assert!(Code::find(Uuid::new_v4(), connection).is_err());
}

#[test]
fn for_display() {
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
    let code = project
        .create_code()
        .with_event(&event)
        .for_ticket_type(&ticket_type)
        .for_ticket_type(&ticket_type2)
        .finish();

    let mut display_code: DisplayCode = code.for_display(connection).unwrap();
    assert_eq!(code.id, display_code.id);
    assert_eq!(code.name, display_code.name);
    assert_eq!(code.redemption_code, display_code.redemption_code);
    assert_eq!(
        display_code.ticket_type_ids.sort(),
        vec![ticket_type.id, ticket_type2.id].sort()
    );
}

#[test]
fn update_ticket_types() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project
        .create_event()
        .with_ticket_pricing()
        .with_ticket_type_count(3)
        .finish();
    let ticket_types = event.ticket_types(true, None, &connection).unwrap();
    let ticket_type = &ticket_types[0];
    let ticket_type2 = &ticket_types[1];
    let ticket_type3 = &ticket_types[2];
    let code = project
        .create_code()
        .with_event(&event)
        .for_ticket_type(&ticket_type)
        .for_ticket_type(&ticket_type2)
        .finish();
    let mut display_code = code.for_display(&connection).unwrap();
    assert_eq!(
        display_code.ticket_type_ids.sort(),
        vec![ticket_type.id, ticket_type2.id].sort()
    );

    code.update_ticket_types(vec![ticket_type.id, ticket_type3.id], connection)
        .unwrap();
    let mut display_code = code.for_display(&connection).unwrap();
    assert_eq!(
        display_code.ticket_type_ids.sort(),
        vec![ticket_type.id, ticket_type3.id].sort()
    );
}

#[test]
fn event() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project.create_event().finish();
    let code = project.create_code().with_event(&event).finish();

    assert_eq!(code.event(connection).unwrap(), event);
}

#[test]
fn organization() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project.create_event().finish();
    let code = project.create_code().with_event(&event).finish();

    let organization = code.organization(connection).unwrap();
    assert_eq!(event.organization(connection).unwrap(), organization);
}

#[test]
fn destroy() {
    let project = TestProject::new();
    let code = project.create_code().finish();
    assert!(code.destroy(project.get_connection()).unwrap() > 0);
    assert!(Code::find(code.id, project.get_connection()).is_err());
}

#[test]
pub fn find_for_event() {
    let db = TestProject::new();
    let event = db.create_event().with_ticket_pricing().finish();
    let code = db
        .create_code()
        .with_name("Discount 1".into())
        .with_event(&event)
        .with_code_type(CodeTypes::Discount)
        .finish();
    let code2 = db
        .create_code()
        .with_name("Discount 2".into())
        .with_event(&event)
        .with_code_type(CodeTypes::Discount)
        .finish();
    let code3 = db
        .create_code()
        .with_name("Access 1".into())
        .with_event(&event)
        .with_code_type(CodeTypes::Access)
        .finish();
    let code4 = db
        .create_code()
        .with_name("Access 2".into())
        .with_event(&event)
        .with_code_type(CodeTypes::Access)
        .finish();

    let codes =
        Code::find_for_event(event.id, Some(CodeTypes::Discount), db.get_connection()).unwrap();
    assert_eq!(
        vec![
            code.for_display(db.get_connection()).unwrap(),
            code2.for_display(db.get_connection()).unwrap()
        ],
        codes
    );

    let codes =
        Code::find_for_event(event.id, Some(CodeTypes::Access), db.get_connection()).unwrap();
    assert_eq!(
        vec![
            code3.for_display(db.get_connection()).unwrap(),
            code4.for_display(db.get_connection()).unwrap()
        ],
        codes
    );
}
