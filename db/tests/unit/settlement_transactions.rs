use bigneon_db::dev::TestProject;
use bigneon_db::models::*;
use chrono::prelude::*;

#[test]
fn create() {
    let project = TestProject::new();
    let connection = project.get_connection();
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
    let new_settlement = NewSettlementRequest {
        start_utc: NaiveDate::from_ymd(2016, 7, 8).and_hms(4, 10, 11),
        end_utc: NaiveDate::from_ymd(2020, 7, 8).and_hms(4, 10, 11),
        comment: Some("test comment".to_string()),
        only_finished_events: None,
        adjustments: None,
    };

    let _prepared = new_settlement
        .prepare(organization.id, user.id, connection)
        .unwrap();
    let settlement = new_settlement
        .commit(organization.id, user.id, connection)
        .unwrap();

    let created_settlement_trans = project
        .create_new_settlement_transaction()
        .with_value_in_cents(100)
        .with_event_id(event.id)
        .with_settlement_id(settlement.id)
        .finish();

    assert_eq!(created_settlement_trans.event_id, event.id);
    assert_eq!(
        created_settlement_trans.comment,
        Some("test comment".to_string())
    );
    assert_eq!(created_settlement_trans.order_item_id, None);
    assert_eq!(created_settlement_trans.value_in_cents, 100);
}
