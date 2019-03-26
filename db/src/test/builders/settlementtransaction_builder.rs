use diesel::prelude::*;
use prelude::*;
use uuid::Uuid;

pub struct SettlementtransactionBuilder<'a> {
    settlement_id: Option<Uuid>,
    event_id: Uuid,
    order_item_id: Option<Uuid>,
    settlement_status: Option<SettlementStatus>,
    transaction_type: Option<SettlementTransactionType>,
    value_in_cents: i64,
    comment: Option<String>,
    connection: &'a PgConnection,
}

impl<'a> SettlementtransactionBuilder<'a> {
    pub fn new(connection: &PgConnection) -> SettlementtransactionBuilder {
        SettlementtransactionBuilder {
            settlement_id: None,
            event_id: Uuid::new_v4(),
            order_item_id: None,
            settlement_status: None,
            transaction_type: None,
            value_in_cents: 100,
            comment: Some("test comment".to_string()),

            connection: connection,
        }
    }

    pub fn with_value_in_cents(mut self, value_in_cents: i64) -> Self {
        self.value_in_cents = value_in_cents;
        self
    }

    pub fn with_event_id(mut self, event_id: Uuid) -> Self {
        self.event_id = event_id;
        self
    }

    pub fn with_settlement_id(mut self, settlement_id: Uuid) -> Self {
        self.settlement_id = Some(settlement_id);
        self
    }

    pub fn finish(&mut self) -> SettlementTransaction {
        let settlement_trans = NewSettlementTransaction {
            settlement_id: self.settlement_id,
            event_id: self.event_id,
            order_item_id: self.order_item_id,
            settlement_status: self.settlement_status,
            transaction_type: self.transaction_type,
            value_in_cents: self.value_in_cents,
            comment: self.comment.clone(),
        };
        settlement_trans.commit(self.connection).unwrap()
    }
}
