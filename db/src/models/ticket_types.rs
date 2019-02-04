use chrono::NaiveDateTime;
use diesel;
use diesel::dsl;
use diesel::prelude::*;
use diesel::sql_types::{Nullable, Text, Uuid as dUuid};
use models::*;
use schema::{
    assets, events, fee_schedules, organizations, ticket_instances, ticket_pricing,
    ticket_type_codes, ticket_types,
};
use utils::errors::*;
use uuid::Uuid;
use validator::*;
use validators;

#[derive(Associations, Clone, Debug, Identifiable, PartialEq, Queryable, QueryableByName)]
#[table_name = "ticket_types"]
#[belongs_to(Event)]
pub struct TicketType {
    pub id: Uuid,
    pub event_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub status: TicketTypeStatus,
    pub start_date: NaiveDateTime,
    pub end_date: NaiveDateTime,
    pub increment: i32,
    pub limit_per_person: i32,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
    pub price_in_cents: i64,
    pub cancelled_at: Option<NaiveDateTime>,
}

#[derive(AsChangeset, Default, Deserialize)]
#[table_name = "ticket_types"]
pub struct TicketTypeEditableAttributes {
    pub name: Option<String>,
    pub description: Option<Option<String>>,
    pub start_date: Option<NaiveDateTime>,
    pub end_date: Option<NaiveDateTime>,
    pub increment: Option<i32>,
    pub limit_per_person: Option<i32>,
    pub price_in_cents: Option<i64>,
}

impl TicketType {
    // Properties at the top

    pub fn fee_schedule(&self, conn: &PgConnection) -> Result<FeeSchedule, DatabaseError> {
        ticket_types::table
            .inner_join(
                events::table.inner_join(organizations::table.inner_join(fee_schedules::table)),
            )
            .filter(ticket_types::id.eq(self.id))
            .select(fee_schedules::all_columns)
            .first(conn)
            .to_db_error(
                ErrorCode::QueryError,
                "Could not retrieve fee schedule for ticket type",
            )
    }

    /// Creates a ticket type. `Event::add_ticket_type` should be used in most scenarios
    pub(crate) fn create(
        event_id: Uuid,
        name: String,
        description: Option<String>,
        start_date: NaiveDateTime,
        end_date: NaiveDateTime,
        increment: Option<i32>,
        limit_per_person: i32,
        price_in_cents: i64,
    ) -> NewTicketType {
        NewTicketType {
            event_id,
            name,
            description,
            status: TicketTypeStatus::Published,
            start_date,
            end_date,
            increment,
            limit_per_person,
            price_in_cents,
        }
    }

    pub fn update(
        &self,
        attributes: TicketTypeEditableAttributes,
        conn: &PgConnection,
    ) -> Result<TicketType, DatabaseError> {
        self.validate_record(&attributes)?;
        let result: TicketType = diesel::update(self)
            .set((&attributes, ticket_types::updated_at.eq(dsl::now)))
            .get_result(conn)
            .to_db_error(ErrorCode::UpdateError, "Could not update ticket_types")?;

        //Delete the old default ticket pricing and create a new default to preserve the purchase history
        let ticket_pricing = TicketPricing::get_default(self.id, conn);

        match ticket_pricing {
            Ok(tp) => {
                tp.destroy(conn)?;
            }
            Err(e) => {
                println!("{}", e.message);
            }
        }

        self.add_ticket_pricing(
            result.name.clone(),
            result.start_date,
            result.end_date,
            result.price_in_cents,
            false,
            Some(TicketPricingStatus::Default),
            conn,
        )?;

        Ok(result)
    }

    pub fn cancel(&self, conn: &PgConnection) -> Result<TicketType, DatabaseError> {
        let result: TicketType = diesel::update(self)
            .set((
                ticket_types::status.eq(TicketTypeStatus::Cancelled),
                ticket_types::cancelled_at.eq(dsl::now.nullable()),
                ticket_types::updated_at.eq(dsl::now),
            ))
            .get_result(conn)
            .to_db_error(ErrorCode::UpdateError, "Could not update ticket_types")?;

        Ok(result)
    }

    pub fn find_for_code(
        code_id: Uuid,
        conn: &PgConnection,
    ) -> Result<Vec<TicketType>, DatabaseError> {
        ticket_types::table
            .inner_join(
                ticket_type_codes::table.on(ticket_type_codes::ticket_type_id.eq(ticket_types::id)),
            )
            .filter(ticket_type_codes::code_id.eq(code_id))
            .select(ticket_types::all_columns)
            .get_results(conn)
            .to_db_error(
                ErrorCode::QueryError,
                "Could not find ticket types for code",
            )
    }

    pub fn find(id: Uuid, conn: &PgConnection) -> Result<TicketType, DatabaseError> {
        ticket_types::table
            .filter(ticket_types::id.eq(id))
            .get_result(conn)
            .to_db_error(ErrorCode::QueryError, "Could not find ticket type")
    }

    pub fn validate_record(
        &self,
        attributes: &TicketTypeEditableAttributes,
    ) -> Result<(), DatabaseError> {
        let validation_errors = validators::append_validation_error(
            Ok(()),
            "start_date",
            validators::start_date_valid(
                attributes.start_date.unwrap_or(self.start_date),
                attributes.end_date.unwrap_or(self.end_date),
            ),
        );

        Ok(validation_errors?)
    }

    pub fn validate_ticket_pricing(&self, conn: &PgConnection) -> Result<(), DatabaseError> {
        let mut validation_errors: Result<(), ValidationErrors> = Ok(());

        for ticket_pricing in self.ticket_pricing(conn)? {
            validation_errors = validators::append_validation_error(
                validation_errors,
                "ticket_pricing",
                TicketPricing::ticket_pricing_no_overlapping_periods(
                    ticket_pricing.id,
                    self.id,
                    ticket_pricing.start_date,
                    ticket_pricing.end_date,
                    ticket_pricing.is_box_office_only,
                    ticket_pricing.status,
                    conn,
                )?,
            );
            validation_errors = validators::append_validation_error(
                validation_errors,
                "ticket_pricing.start_date",
                TicketPricing::ticket_pricing_does_not_overlap_ticket_type_start_date(
                    self,
                    ticket_pricing.start_date,
                )?,
            );
            validation_errors = validators::append_validation_error(
                validation_errors,
                "ticket_pricing.end_date",
                TicketPricing::ticket_pricing_does_not_overlap_ticket_type_end_date(
                    self,
                    ticket_pricing.end_date,
                )?,
            );
        }

        Ok(validation_errors?)
    }

    pub fn find_by_event_id(
        event_id: Uuid,
        filter_access_tokens: bool,
        redemption_code: Option<String>,
        conn: &PgConnection,
    ) -> Result<Vec<TicketType>, DatabaseError> {
        if filter_access_tokens {
            let query = r#"
                    SELECT DISTINCT tt.*
                    FROM ticket_types tt
                    LEFT JOIN (
                        SELECT ttc.ticket_type_id, c.redemption_code, c.start_date, c.end_date
                        FROM ticket_type_codes ttc
                        JOIN codes c ON ttc.code_id = c.id
                        JOIN ticket_types tt ON tt.id = ttc.ticket_type_id
                        WHERE c.code_type = 'Access' AND tt.event_id = $1
                    ) ttc ON ttc.ticket_type_id = tt.id
                    WHERE tt.event_id = $1
                    AND (
                        ttc.redemption_code is null
                        OR (
                            ttc.redemption_code = $2 and ttc.start_date <= now() and ttc.end_date >= now()
                        )
                    )
                    ORDER BY tt.name
                    "#;

            diesel::sql_query(query)
                .bind::<dUuid, _>(event_id)
                .bind::<Nullable<Text>, _>(redemption_code)
                .load(conn)
                .to_db_error(
                    ErrorCode::QueryError,
                    "Could not load ticket types for event",
                )
        } else {
            ticket_types::table
                .filter(ticket_types::event_id.eq(event_id))
                .load(conn)
                .to_db_error(
                    ErrorCode::QueryError,
                    "Could not load ticket types for event",
                )
        }
    }

    pub fn ticket_count(&self, conn: &PgConnection) -> Result<u32, DatabaseError> {
        let valid_ticket_count: i64 = ticket_instances::table
            .inner_join(assets::table)
            .filter(assets::ticket_type_id.eq(self.id))
            .select(dsl::count(ticket_instances::id))
            .first(conn)
            .to_db_error(
                ErrorCode::QueryError,
                "Could not load ticket pricing for ticket type",
            )?;
        Ok(valid_ticket_count as u32)
    }

    pub fn valid_ticket_count(&self, conn: &PgConnection) -> Result<u32, DatabaseError> {
        let valid_ticket_count: i64 = ticket_instances::table
            .inner_join(assets::table)
            .filter(assets::ticket_type_id.eq(self.id))
            .filter(ticket_instances::status.ne(TicketInstanceStatus::Nullified))
            .select(dsl::count(ticket_instances::id))
            .first(conn)
            .to_db_error(
                ErrorCode::QueryError,
                "Could not load ticket pricing for ticket type",
            )?;
        Ok(valid_ticket_count as u32)
    }

    pub fn valid_unsold_ticket_count(&self, conn: &PgConnection) -> Result<u32, DatabaseError> {
        let valid_unsold_ticket_count: i64 = ticket_instances::table
            .inner_join(assets::table)
            .filter(assets::ticket_type_id.eq(self.id))
            .filter(ticket_instances::status.eq_any(vec![
                TicketInstanceStatus::Available,
                TicketInstanceStatus::Reserved,
            ]))
            .select(dsl::count(ticket_instances::id))
            .first(conn)
            .to_db_error(
                ErrorCode::QueryError,
                "Could not load ticket pricing for ticket type",
            )?;
        Ok(valid_unsold_ticket_count as u32)
    }

    pub fn valid_available_ticket_count(&self, conn: &PgConnection) -> Result<u32, DatabaseError> {
        let valid_available_ticket_count: i64 = ticket_instances::table
            .inner_join(assets::table)
            .filter(assets::ticket_type_id.eq(self.id))
            .filter(ticket_instances::status.eq(TicketInstanceStatus::Available))
            .select(dsl::count(ticket_instances::id))
            .first(conn)
            .to_db_error(
                ErrorCode::QueryError,
                "Could not load ticket pricing for ticket type",
            )?;
        Ok(valid_available_ticket_count as u32)
    }

    pub fn current_ticket_pricing(
        &self,
        box_office_pricing: bool,
        conn: &PgConnection,
    ) -> Result<TicketPricing, DatabaseError> {
        TicketPricing::get_current_ticket_pricing(self.id, box_office_pricing, false, conn)
    }

    pub fn ticket_pricing(&self, conn: &PgConnection) -> Result<Vec<TicketPricing>, DatabaseError> {
        ticket_pricing::table
            .filter(ticket_pricing::ticket_type_id.eq(self.id))
            .filter(ticket_pricing::status.eq(TicketPricingStatus::Published))
            .order_by(ticket_pricing::name)
            .load(conn)
            .to_db_error(
                ErrorCode::QueryError,
                "Could not load ticket pricing for ticket type",
            )
    }

    pub fn is_event_not_draft(
        ticket_type_id: &Uuid,
        conn: &PgConnection,
    ) -> Result<bool, DatabaseError> {
        let valid_ticket_count: i64 = ticket_types::table
            .inner_join(events::table)
            .filter(ticket_types::id.eq(ticket_type_id))
            .filter(events::status.ne(EventStatus::Draft.to_string()))
            .select(dsl::count(ticket_types::id))
            .first(conn)
            .to_db_error(
                ErrorCode::QueryError,
                "Could not load ticket pricing for ticket type",
            )?;
        if valid_ticket_count <= 0 {
            return Ok(false);
        }
        Ok(true)
    }

    pub fn valid_ticket_pricing(
        &self,
        include_default: bool,
        conn: &PgConnection,
    ) -> Result<Vec<TicketPricing>, DatabaseError> {
        let mut query = ticket_pricing::table
            .filter(ticket_pricing::ticket_type_id.eq(self.id))
            .filter(ticket_pricing::status.ne(TicketPricingStatus::Deleted))
            .order_by(ticket_pricing::name)
            .into_boxed();
        if !include_default {
            query = query.filter(ticket_pricing::status.ne(TicketPricingStatus::Default));
        }
        query.load(conn).to_db_error(
            ErrorCode::QueryError,
            "Could not load ticket pricing for ticket type",
        )
    }

    pub fn add_ticket_pricing(
        &self,
        name: String,
        start_date: NaiveDateTime,
        end_date: NaiveDateTime,
        price_in_cents: i64,
        is_box_office_only: bool,
        status: Option<TicketPricingStatus>,
        conn: &PgConnection,
    ) -> Result<TicketPricing, DatabaseError> {
        TicketPricing::create(
            self.id,
            name,
            start_date,
            end_date,
            price_in_cents,
            is_box_office_only,
            status,
        )
        .commit(conn)
    }
}

#[derive(Insertable)]
#[table_name = "ticket_types"]
pub struct NewTicketType {
    event_id: Uuid,
    name: String,
    description: Option<String>,
    status: TicketTypeStatus,
    start_date: NaiveDateTime,
    end_date: NaiveDateTime,
    increment: Option<i32>,
    limit_per_person: i32,
    price_in_cents: i64,
}

impl NewTicketType {
    pub fn commit(self, conn: &PgConnection) -> Result<TicketType, DatabaseError> {
        self.validate_record()?;
        let result: TicketType = diesel::insert_into(ticket_types::table)
            .values(self)
            .get_result(conn)
            .to_db_error(ErrorCode::InsertError, "Could not create ticket type")?;

        result.add_ticket_pricing(
            "Default Pricing".to_string(),
            result.start_date,
            result.end_date,
            result.price_in_cents,
            false,
            Some(TicketPricingStatus::Default),
            conn,
        )?;

        Ok(result)
    }

    pub fn validate_record(&self) -> Result<(), DatabaseError> {
        let validation_errors = validators::append_validation_error(
            Ok(()),
            "start_date",
            validators::start_date_valid(self.start_date, self.end_date),
        );

        Ok(validation_errors?)
    }
}
