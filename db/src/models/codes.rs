use chrono::prelude::*;
use diesel;
use diesel::dsl;
use diesel::prelude::*;
use diesel::sql_types::{Array, BigInt, Nullable, Text, Timestamp, Uuid as dUuid};
use models::*;
use schema::codes;
use std::borrow::Cow;
use utils::errors::*;
use uuid::Uuid;
use validator::*;
use validators::{self, *};

#[derive(Clone, Debug, Deserialize, Identifiable, PartialEq, Queryable, Serialize)]
pub struct Code {
    pub id: Uuid,
    pub name: String,
    pub event_id: Uuid,
    pub code_type: CodeTypes,
    pub redemption_code: String,
    pub max_uses: i64,
    pub discount_in_cents: Option<i64>,
    pub start_date: NaiveDateTime,
    pub end_date: NaiveDateTime,
    pub max_tickets_per_user: Option<i64>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Deserialize, PartialEq, Queryable, Serialize, QueryableByName)]
pub struct DisplayCode {
    #[sql_type = "dUuid"]
    pub id: Uuid,
    #[sql_type = "Text"]
    pub name: String,
    #[sql_type = "dUuid"]
    pub event_id: Uuid,
    #[sql_type = "Text"]
    pub code_type: CodeTypes,
    #[sql_type = "Text"]
    pub redemption_code: String,
    #[sql_type = "BigInt"]
    pub max_uses: i64,
    #[sql_type = "Nullable<BigInt>"]
    pub discount_in_cents: Option<i64>,
    #[sql_type = "Timestamp"]
    pub start_date: NaiveDateTime,
    #[sql_type = "Timestamp"]
    pub end_date: NaiveDateTime,
    #[sql_type = "Nullable<BigInt>"]
    pub max_tickets_per_user: Option<i64>,
    #[sql_type = "Timestamp"]
    pub created_at: NaiveDateTime,
    #[sql_type = "Timestamp"]
    pub updated_at: NaiveDateTime,
    #[sql_type = "Array<dUuid>"]
    pub ticket_type_ids: Vec<Uuid>,
}

#[derive(AsChangeset, Default, Deserialize, Validate)]
#[table_name = "codes"]
pub struct UpdateCodeAttributes {
    pub name: Option<String>,
    #[validate(length(
        min = "6",
        message = "Redemption code must be at least 6 characters long"
    ))]
    pub redemption_code: Option<String>,
    pub max_uses: Option<i64>,
    pub discount_in_cents: Option<Option<i64>>,
    pub start_date: Option<NaiveDateTime>,
    pub end_date: Option<NaiveDateTime>,
    pub max_tickets_per_user: Option<Option<i64>>,
}

impl Code {
    pub fn find_by_redemption_code(
        redemption_code: &str,
        conn: &PgConnection,
    ) -> Result<Code, DatabaseError> {
        codes::table
            .filter(codes::redemption_code.eq(redemption_code.to_uppercase()))
            .first(conn)
            .to_db_error(
                ErrorCode::QueryError,
                "Could not load code with that redeem code",
            )
    }

    pub fn update_ticket_types(
        &self,
        ticket_type_ids: Vec<Uuid>,
        conn: &PgConnection,
    ) -> Result<(), DatabaseError> {
        let existing_ticket_type_ids = TicketType::find_for_code(self.id, conn)?
            .into_iter()
            .map(|tt| tt.id)
            .collect::<Vec<Uuid>>();
        let pending_deletion = existing_ticket_type_ids
            .clone()
            .into_iter()
            .filter(|id| !ticket_type_ids.contains(id))
            .collect::<Vec<Uuid>>();
        let pending_addition = ticket_type_ids
            .into_iter()
            .filter(|id| !existing_ticket_type_ids.contains(id))
            .collect::<Vec<Uuid>>();
        TicketTypeCode::destroy_multiple(self.id, pending_deletion, conn)?;

        for ticket_type_id in pending_addition {
            TicketTypeCode::create(ticket_type_id, self.id).commit(conn)?;
        }
        Ok(())
    }

    pub fn for_display(&self, conn: &PgConnection) -> Result<DisplayCode, DatabaseError> {
        let ticket_type_ids = TicketType::find_for_code(self.id, conn)?
            .into_iter()
            .map(|tt| tt.id)
            .collect::<Vec<Uuid>>();

        Ok(DisplayCode {
            id: self.id,
            name: self.name.clone(),
            event_id: self.event_id,
            code_type: self.code_type.clone(),
            redemption_code: self.redemption_code.clone(),
            max_uses: self.max_uses,
            discount_in_cents: self.discount_in_cents,
            start_date: self.start_date,
            end_date: self.end_date,
            max_tickets_per_user: self.max_tickets_per_user,
            created_at: self.created_at,
            updated_at: self.updated_at,
            ticket_type_ids: ticket_type_ids,
        })
    }

    pub fn create(
        name: String,
        event_id: Uuid,
        code_type: CodeTypes,
        redemption_code: String,
        max_uses: u32,
        discount_in_cents: Option<u32>,
        start_date: NaiveDateTime,
        end_date: NaiveDateTime,
        max_tickets_per_user: Option<u32>,
    ) -> NewCode {
        NewCode {
            name,
            event_id,
            code_type,
            redemption_code: redemption_code.to_uppercase(),
            max_uses: max_uses as i64,
            discount_in_cents: discount_in_cents.map(|max| max as i64),
            start_date,
            end_date,
            max_tickets_per_user: max_tickets_per_user.map(|max| max as i64),
        }
    }

    pub fn confirm_code_valid(&self) -> Result<(), DatabaseError> {
        let now = Utc::now().naive_utc();
        if now < self.start_date || now > self.end_date {
            return DatabaseError::validation_error(
                "code_id",
                "Code not valid for current datetime",
            );
        }
        Ok(())
    }

    pub fn event(&self, conn: &PgConnection) -> Result<Event, DatabaseError> {
        use schema::*;
        events::table
            .filter(events::id.eq(self.event_id))
            .first(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load event for code")
    }

    pub fn organization(&self, conn: &PgConnection) -> Result<Organization, DatabaseError> {
        use schema::*;
        events::table
            .inner_join(organizations::table)
            .filter(events::id.eq(self.event_id))
            .select(organizations::all_columns)
            .first(conn)
            .to_db_error(
                ErrorCode::QueryError,
                "Could not load organization for code",
            )
    }

    pub fn find_for_event(
        event_id: Uuid,
        code_type: Option<CodeTypes>,
        conn: &PgConnection,
    ) -> Result<Vec<DisplayCode>, DatabaseError> {
        let query = r#"
                SELECT
                    codes.id,
                    codes.name,
                    codes.event_id,
                    codes.code_type,
                    codes.redemption_code,
                    codes.max_uses,
                    codes.discount_in_cents,
                    codes.start_date,
                    codes.end_date,
                    codes.max_tickets_per_user,
                    codes.created_at,
                    codes.updated_at,
                    array(select ticket_type_id from ticket_type_codes where ticket_type_codes.code_id = codes.id) as ticket_type_ids
                FROM codes
                WHERE
                    codes.event_id = $1
                    AND ($2 IS NULL OR codes.code_type = $2)
                ORDER BY codes.name;"#;

        diesel::sql_query(query)
            .bind::<diesel::sql_types::Uuid, _>(event_id)
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(
                code_type.map(|s| s.to_string()),
            )
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Cannot find for events")
    }

    pub fn discount_present_for_discount_type(
        code_type: CodeTypes,
        discount_in_cents: Option<i64>,
    ) -> Result<(), ValidationError> {
        if code_type == CodeTypes::Discount && discount_in_cents.is_none() {
            let mut validation_error =
                create_validation_error("required", "Discount required for Discount code type");
            validation_error.add_param(Cow::from("code_type"), &code_type);
            validation_error.add_param(Cow::from("discount"), &discount_in_cents);
            return Err(validation_error);
        }
        Ok(())
    }

    fn validate_record(
        &self,
        update_attrs: &UpdateCodeAttributes,
        conn: &PgConnection,
    ) -> Result<(), DatabaseError> {
        let mut validation_errors = update_attrs.validate();

        validation_errors = validators::append_validation_error(
            validation_errors,
            "start_date",
            validators::start_date_valid(
                update_attrs.start_date.unwrap_or(self.start_date),
                update_attrs.end_date.unwrap_or(self.end_date),
            ),
        );
        validation_errors = validators::append_validation_error(
            validation_errors,
            "discount_in_cents",
            Code::discount_present_for_discount_type(
                self.code_type.clone(),
                update_attrs
                    .discount_in_cents
                    .unwrap_or(self.discount_in_cents),
            ),
        );
        validation_errors = validators::append_validation_error(
            validation_errors,
            "redemption_code",
            redemption_code_unique_per_event_validation(
                Some(self.id),
                "codes".into(),
                update_attrs
                    .redemption_code
                    .clone()
                    .unwrap_or(self.redemption_code.clone()),
                conn,
            )?,
        );

        Ok(validation_errors?)
    }

    pub fn update(
        &self,
        update_attrs: UpdateCodeAttributes,
        conn: &PgConnection,
    ) -> Result<Code, DatabaseError> {
        self.validate_record(&update_attrs, conn)?;
        diesel::update(
            codes::table
                .filter(codes::id.eq(self.id))
                .filter(codes::updated_at.eq(self.updated_at)),
        )
        .set((update_attrs, codes::updated_at.eq(dsl::now)))
        .get_result(conn)
        .to_db_error(ErrorCode::UpdateError, "Could not update code")
    }

    pub fn find(id: Uuid, conn: &PgConnection) -> Result<Code, DatabaseError> {
        codes::table
            .filter(codes::id.eq(id))
            .first(conn)
            .to_db_error(ErrorCode::QueryError, "Could not retrieve code")
    }

    pub fn destroy(&self, conn: &PgConnection) -> Result<usize, DatabaseError> {
        DatabaseError::wrap(
            ErrorCode::DeleteError,
            "Could not remove code",
            diesel::delete(self).execute(conn),
        )
    }
}

#[derive(Deserialize, Insertable, Serialize, Validate)]
#[table_name = "codes"]
pub struct NewCode {
    pub name: String,
    pub event_id: Uuid,
    pub code_type: CodeTypes,
    #[validate(length(
        min = "6",
        message = "Redemption code must be at least 6 characters long"
    ))]
    pub redemption_code: String,
    pub max_uses: i64,
    pub discount_in_cents: Option<i64>,
    pub start_date: NaiveDateTime,
    pub end_date: NaiveDateTime,
    pub max_tickets_per_user: Option<i64>,
}

impl NewCode {
    pub fn commit(self, conn: &PgConnection) -> Result<Code, DatabaseError> {
        self.validate_record(conn)?;

        diesel::insert_into(codes::table)
            .values(self)
            .get_result(conn)
            .to_db_error(ErrorCode::InsertError, "Could not create code")
    }

    fn validate_record(&self, conn: &PgConnection) -> Result<(), DatabaseError> {
        let mut validation_errors = self.validate();

        validation_errors = validators::append_validation_error(
            validation_errors,
            "discount_in_cents",
            Code::discount_present_for_discount_type(
                self.code_type.clone(),
                self.discount_in_cents,
            ),
        );
        validation_errors = validators::append_validation_error(
            validation_errors,
            "start_date",
            validators::start_date_valid(self.start_date, self.end_date),
        );
        validation_errors = validators::append_validation_error(
            validation_errors,
            "redemption_code",
            redemption_code_unique_per_event_validation(
                None,
                "codes".into(),
                self.redemption_code.clone(),
                conn,
            )?,
        );

        Ok(validation_errors?)
    }
}
