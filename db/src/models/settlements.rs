use chrono::NaiveDateTime;
use diesel;
use diesel::expression::dsl::sql;
use diesel::prelude::*;
use diesel::sql_types::BigInt;
use models::*;
use schema::{settlement_transactions as settlement_transactions_table, settlements};
use std::collections::HashMap;
use utils::errors::ConvertToDatabaseError;
use utils::errors::DatabaseError;
use utils::errors::ErrorCode;
use uuid::Uuid;

#[derive(Associations, Identifiable, Queryable, Serialize, Deserialize, Clone)]
#[table_name = "settlements"]
pub struct Settlement {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub user_id: Uuid,
    pub start_time: NaiveDateTime,
    pub end_time: NaiveDateTime,
    pub status: SettlementStatus,
    pub comment: Option<String>,
    pub only_finished_events: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Associations, Queryable, Serialize)]
#[table_name = "settlements"]
pub struct PendingSettlement {
    pub organization_id: Uuid,
    pub user_id: Uuid,
    pub start_time: NaiveDateTime,
    pub end_time: NaiveDateTime,
    pub status: SettlementStatus,
    pub comment: Option<String>,
    pub only_finished_events: bool,
    pub sales_per_event: HashMap<Uuid, TicketSalesAndCounts>,
    pub transactions: Vec<NewSettlementTransaction>,
}

#[derive(Insertable, Serialize, Deserialize, PartialEq, Debug)]
#[table_name = "settlements"]
pub struct NewSettlement {
    pub organization_id: Uuid,
    pub user_id: Uuid,
    pub start_time: NaiveDateTime,
    pub end_time: NaiveDateTime,
    pub status: SettlementStatus,
    pub comment: Option<String>,
    pub only_finished_events: bool,
}

#[derive(Clone, Deserialize)]
pub struct NewSettlementRequest {
    pub start_utc: NaiveDateTime,
    pub end_utc: NaiveDateTime,
    pub comment: Option<String>,
    pub only_finished_events: Option<bool>,
    pub adjustments: Option<Vec<NewSettlementTransaction>>,
}

#[derive(Serialize, Deserialize)]
pub struct DisplaySettlement {
    pub settlement: Settlement,
    pub transactions: Vec<SettlementTransaction>,
    pub events: Vec<Event>,
}

impl NewSettlementRequest {
    pub fn commit(
        &self,
        organization_id: Uuid,
        user_id: Uuid,
        conn: &PgConnection,
    ) -> Result<Settlement, DatabaseError> {
        let new_settlement = NewSettlement {
            organization_id,
            user_id,
            start_time: self.start_utc.clone(),
            end_time: self.end_utc.clone(),
            status: SettlementStatus::PendingSettlement,
            comment: self.comment.clone(),
            only_finished_events: self.only_finished_events.clone().unwrap_or(true),
        };

        //        let _adjustments = self.adjustments.unwrap_or(HashMap::new());
        let settlement = DatabaseError::wrap(
            ErrorCode::InsertError,
            "Could not create new settlement",
            diesel::insert_into(settlements::table)
                .values(new_settlement)
                .get_result::<Settlement>(conn),
        )?;

        let new_settlement_transactions = Settlement::create_base_transactions(
            Some(settlement.id),
            organization_id.clone(),
            self.start_utc,
            self.end_utc,
            conn,
        )?;
        let _settlement_transactions =
            settlement.store_base_transactions(new_settlement_transactions, conn)?;

        let new_adjustments = self.adjustments.clone().unwrap_or(vec![]);

        for new_adjustment in new_adjustments {
            let new_adjustment_transaction = NewSettlementTransaction {
                settlement_id: Some(settlement.id.clone()),
                ..new_adjustment
            };
            let _adjustment_transaction = new_adjustment_transaction.commit(conn)?;
        }
        Ok(settlement)
    }

    pub fn prepare(
        &self,
        organization_id: Uuid,
        user_id: Uuid,
        conn: &PgConnection,
    ) -> Result<PendingSettlement, DatabaseError> {
        let pending_settlement = PendingSettlement {
            organization_id,
            user_id,
            start_time: self.start_utc.clone(),
            end_time: self.end_utc.clone(),
            status: SettlementStatus::PendingSettlement,
            comment: self.comment.clone(),
            only_finished_events: self.only_finished_events.unwrap_or(true),
            sales_per_event: Settlement::get_counts(
                organization_id,
                self.start_utc.clone(),
                self.end_utc.clone(),
                conn,
            )?,
            transactions: Settlement::create_base_transactions(
                None,
                organization_id,
                self.start_utc.clone(),
                self.end_utc.clone(),
                conn,
            )?,
        };
        Ok(pending_settlement)
    }
}

impl Settlement {
    pub fn read(id: Uuid, conn: &PgConnection) -> Result<Settlement, DatabaseError> {
        let settlement = settlements::table
            .filter(settlements::id.eq(id))
            .get_result::<Settlement>(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load Settlement")?;

        Ok(settlement)
    }

    pub fn get_counts(
        organization_id: Uuid,
        start_time: NaiveDateTime,
        end_time: NaiveDateTime,
        conn: &PgConnection,
    ) -> Result<HashMap<Uuid, TicketSalesAndCounts>, DatabaseError> {
        let events = Event::get_all_events_ending_between(
            organization_id,
            start_time,
            end_time,
            EventStatus::Published,
            conn,
        )?;

        let mut result: HashMap<Uuid, TicketSalesAndCounts> = HashMap::new();

        for event in events {
            //TODO We currently only generate settlements for events ending before end_time.
            let group_by_ticket_type = true;
            let group_by_ticket_pricing = true;
            let group_by_hold = true;
            let group_by_event = true;
            result.insert(
                event.id,
                event.count_report(
                    Some(start_time),
                    Some(end_time),
                    group_by_ticket_type,
                    group_by_ticket_pricing,
                    group_by_hold,
                    group_by_event,
                    conn,
                )?,
            );
        }

        Ok(result)
    }

    pub fn for_display(self, conn: &PgConnection) -> Result<DisplaySettlement, DatabaseError> {
        let transactions = settlement_transactions_table::table
            .filter(settlement_transactions_table::settlement_id.eq(self.id))
            .get_results::<SettlementTransaction>(conn)
            .to_db_error(
                ErrorCode::QueryError,
                "Could not load Settlement Transactions",
            )?;

        let mut unique_events: Vec<Uuid> = transactions.iter().map(|i| i.event_id).collect();
        unique_events.sort();
        unique_events.dedup();

        let events = Event::find_by_ids(unique_events, conn)?;
        let settlement = self;
        Ok(DisplaySettlement {
            settlement,
            transactions,
            events,
        })
    }
    pub fn destroy(self, conn: &PgConnection) -> Result<usize, DatabaseError> {
        diesel::delete(settlements::table.filter(settlements::id.eq(self.id)))
            .execute(conn)
            .to_db_error(ErrorCode::DeleteError, "Error removing user")
    }

    pub fn store_base_transactions(
        &self,
        transactions: Vec<NewSettlementTransaction>,
        conn: &PgConnection,
    ) -> Result<Vec<SettlementTransaction>, DatabaseError> {
        let mut result = vec![];
        for new_transaction in transactions {
            result.push(new_transaction.commit(conn)?);
        }
        Ok(result)
    }

    pub fn create_base_transactions(
        settlement_id: Option<Uuid>,
        organization_id: Uuid,
        start_time: NaiveDateTime,
        end_time: NaiveDateTime,
        conn: &PgConnection,
    ) -> Result<Vec<NewSettlementTransaction>, DatabaseError> {
        let settlement_id = settlement_id.unwrap_or(Uuid::default());
        let counts_by_event = Settlement::get_counts(organization_id, start_time, end_time, conn)?;
        let mut results = vec![];
        for (event_id, counts) in counts_by_event.iter() {
            let mut face_value = 0;
            let mut service_fee_value = 0;
            for row in counts.sales.iter() {
                face_value += row.online_sales_in_cents;
                service_fee_value += row.total_online_fees_in_cents;
            }
            results.push(NewSettlementTransaction {
                settlement_id: Some(settlement_id.clone()),
                event_id: event_id.clone().to_owned(),
                order_item_id: None,
                settlement_status: Some(SettlementStatus::PendingSettlement),
                transaction_type: Some(SettlementTransactionType::Report),
                value_in_cents: face_value,
                comment: Some("Face Amount Owed To Client".to_string()),
            });
            results.push(NewSettlementTransaction {
                settlement_id: Some(settlement_id.clone()),
                event_id: event_id.clone().to_owned(),
                order_item_id: None,
                settlement_status: Some(SettlementStatus::PendingSettlement),
                transaction_type: Some(SettlementTransactionType::Report),
                value_in_cents: service_fee_value,
                comment: Some("Service Fee Revenue Share".to_string()),
            });
        }
        Ok(results)
    }

    pub fn index(
        organization_id: Uuid,
        limit: Option<u32>,
        page: Option<u32>,
        conn: &PgConnection,
    ) -> Result<(Vec<Settlement>, i64), DatabaseError> {
        let limit = limit.unwrap_or(20);
        let page = page.unwrap_or(0);

        let total = settlements::table
            .filter(settlements::organization_id.eq(organization_id))
            .select(sql::<BigInt>("count(*) AS total"))
            .get_result::<i64>(conn)
            .to_db_error(ErrorCode::QueryError, "Could not get total settlements")?;

        let settlements_vec = settlements::table
            .filter(settlements::organization_id.eq(organization_id))
            .order_by(settlements::start_time.desc())
            .select(settlements::all_columns)
            .limit(limit as i64)
            .offset(limit as i64 * page as i64)
            .load::<Settlement>(conn)
            .to_db_error(ErrorCode::QueryError, "Could not get total settlements")?;

        Ok((settlements_vec, total))
    }
}
