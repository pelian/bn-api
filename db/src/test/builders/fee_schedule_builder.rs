use diesel::prelude::*;
use models::*;
use uuid::Uuid;

pub struct FeeScheduleBuilder<'a> {
    name: String,
    connection: &'a PgConnection,
}

impl<'a> FeeScheduleBuilder<'a> {
    pub fn new(connection: &PgConnection) -> FeeScheduleBuilder {
        FeeScheduleBuilder {
            connection,
            name: "Name".into(),
        }
    }

    pub fn finish(self, current_user_id: Uuid) -> FeeSchedule {
        FeeSchedule::create(
            Uuid::nil(),
            self.name,
            vec![
                NewFeeScheduleRange {
                    min_price_in_cents: 50,
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
        .commit(current_user_id, self.connection)
        .unwrap()
    }
}
