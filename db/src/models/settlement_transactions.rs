use chrono::NaiveDateTime;
use diesel;
use diesel::prelude::*;
use models::*;
use schema::settlement_transactions;
use utils::errors::DatabaseError;
use utils::errors::ErrorCode;
use uuid::Uuid;

#[derive(Identifiable, Queryable, QueryableByName, AsChangeset, Serialize, Deserialize)]
#[table_name = "settlement_transactions"]
pub struct SettlementTransaction {
    pub id: Uuid,
    pub settlement_id: Uuid,
    pub event_id: Uuid,
    pub order_item_id: Option<Uuid>,
    pub settlement_status: SettlementStatus,
    pub transaction_type: SettlementTransactionType,
    pub value_in_cents: i64,
    pub comment: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Clone, Default, Insertable, Serialize, Deserialize, PartialEq, Debug)]
#[table_name = "settlement_transactions"]
pub struct NewSettlementTransaction {
    pub settlement_id: Option<Uuid>,
    pub event_id: Uuid,
    pub order_item_id: Option<Uuid>,
    pub settlement_status: Option<SettlementStatus>,
    pub transaction_type: Option<SettlementTransactionType>,
    pub value_in_cents: i64,
    pub comment: Option<String>,
}
impl NewSettlementTransaction {
    pub fn commit(&self, conn: &PgConnection) -> Result<SettlementTransaction, DatabaseError> {
        DatabaseError::wrap(
            ErrorCode::InsertError,
            "Could not create new settlement transaction",
            diesel::insert_into(settlement_transactions::table)
                .values(self)
                .get_result::<SettlementTransaction>(conn),
        )
    }
}
