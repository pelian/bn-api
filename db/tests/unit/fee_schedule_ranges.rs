use bigneon_db::dev::TestProject;
use bigneon_db::models::{FeeSchedule, FeeScheduleRange, NewFeeScheduleRange};
use uuid::Uuid;

#[test]
fn find() {
    let project = TestProject::new();
    let creator = project.create_user().finish();

    let fee_schedule = FeeSchedule::create(
        Uuid::nil(),
        "default".to_string(),
        vec![
            NewFeeScheduleRange {
                min_price_in_cents: 20,
                company_fee_in_cents: 4,
                client_fee_in_cents: 6,
            },
            NewFeeScheduleRange {
                min_price_in_cents: 100,
                company_fee_in_cents: 8,
                client_fee_in_cents: 12,
            },
        ],
    )
    .commit(creator.id, project.get_connection())
    .unwrap();

    let fee_schedule_range = fee_schedule
        .get_range(30, project.get_connection())
        .unwrap();

    let found_fee_schedule_range =
        FeeScheduleRange::find(fee_schedule_range.id, project.get_connection()).unwrap();
    assert_eq!(found_fee_schedule_range, fee_schedule_range);
}
