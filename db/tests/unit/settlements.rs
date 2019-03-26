use bigneon_db::dev::TestProject;
use bigneon_db::models::*;
use chrono::prelude::*;

#[test]
fn prepare() {
    let project = TestProject::new();
    let connection = project.get_connection();
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
    let new_settlement = NewSettlementRequest {
        start_utc: NaiveDate::from_ymd(2016, 7, 8).and_hms(4, 10, 11),
        end_utc: NaiveDate::from_ymd(2020, 7, 8).and_hms(4, 10, 11),
        comment: Some("test comment".to_string()),
        only_finished_events: None,
        adjustments: None,
    };

    let prepared = new_settlement
        .prepare(organization.id, user.id, connection)
        .unwrap();

    assert_eq!(prepared.organization_id, organization.id);
    assert_eq!(prepared.user_id, user.id);
    assert_eq!(prepared.comment, Some("test comment".to_string()));
}

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
    let _event = project
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

    assert_eq!(settlement.organization_id, organization.id);
    assert_eq!(settlement.user_id, user.id);
    assert_eq!(settlement.comment, Some("test comment".to_string()));
}

#[test]
fn read() {
    let project = TestProject::new();
    let connection = project.get_connection();
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
    let read_settlement = Settlement::read(settlement.id, connection).unwrap();
    assert_eq!(settlement.id, read_settlement.id);
    assert_eq!(settlement.comment, read_settlement.comment);
    assert_eq!(settlement.start_time, read_settlement.start_time);
}

#[test]
fn get_counts() {
    let project = TestProject::new();
    let connection = project.get_connection();
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
    let _event2 = project
        .create_event()
        .with_name("NewEvent2".into())
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
    let _settlement = new_settlement
        .commit(organization.id, user.id, connection)
        .unwrap();
    let count = Settlement::get_counts(
        organization.id,
        NaiveDate::from_ymd(2016, 7, 8).and_hms(4, 10, 11),
        NaiveDate::from_ymd(2020, 7, 8).and_hms(4, 10, 11),
        connection,
    )
    .unwrap();
    assert_eq!(count.len(), 2);
}

#[test]
fn display() {
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
    let display = settlement.clone().for_display(connection).unwrap();
    assert_eq!(display.settlement.id, settlement.id);
    assert_eq!(display.events[0].id, event.id);
}

#[test]
fn index() {
    let project = TestProject::new();
    let connection = project.get_connection();
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
    let _event2 = project
        .create_event()
        .with_name("NewEvent2".into())
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
    let settlements = Settlement::index(organization.id, None, None, connection)
        .unwrap()
        .0;
    assert_eq!(settlements.len(), 1);
    assert_eq!(settlements[0].id, settlement.id);
}
