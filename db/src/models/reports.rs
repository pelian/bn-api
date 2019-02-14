use chrono::prelude::*;
use diesel;
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Bool, Nullable, Text, Timestamp, Uuid as dUuid};
use itertools::Itertools;
use models::*;
use std::collections::HashMap;
use utils::errors::*;
use uuid::Uuid;

sql_function!(fn ticket_sales_per_ticket_pricing(start: Nullable<Timestamp>, end: Nullable<Timestamp>, group_by_ticket_type: Nullable<Bool>, group_by_event_id: Nullable<Bool>) -> Vec<TicketSalesRow>);
sql_function!(fn ticket_count_per_ticket_type(event_id: Nullable<dUuid>, organization_id: Nullable<dUuid>, group_by_event_id: Nullable<Bool>, group_by_organization_id: Nullable<Bool>) -> Vec<TicketCountRow>);
pub struct Report {}

#[derive(Default, Clone, Debug, Serialize, Deserialize, PartialEq, Queryable, QueryableByName)]
pub struct TicketSalesRow {
    #[sql_type = "Nullable<dUuid>"]
    pub organization_id: Option<Uuid>,
    #[sql_type = "Nullable<dUuid>"]
    pub event_id: Option<Uuid>,
    #[sql_type = "Nullable<dUuid>"]
    pub ticket_type_id: Option<Uuid>,
    #[sql_type = "Nullable<dUuid>"]
    pub ticket_pricing_id: Option<Uuid>,
    #[sql_type = "Nullable<dUuid>"]
    pub hold_id: Option<Uuid>,
    #[sql_type = "Nullable<Text>"]
    pub ticket_name: Option<String>,
    #[sql_type = "Nullable<Text>"]
    pub ticket_status: Option<String>,
    #[sql_type = "Nullable<Text>"]
    pub event_name: Option<String>,
    #[sql_type = "Nullable<Text>"]
    pub hold_name: Option<String>,
    #[sql_type = "Nullable<Text>"]
    pub ticket_pricing_name: Option<String>,
    #[sql_type = "BigInt"]
    pub ticket_pricing_price_in_cents: i64,
    #[sql_type = "BigInt"]
    pub box_office_order_count: i64,
    #[sql_type = "BigInt"]
    pub online_order_count: i64,
    #[sql_type = "BigInt"]
    pub box_office_refunded_count: i64,
    #[sql_type = "BigInt"]
    pub online_refunded_count: i64,
    #[sql_type = "BigInt"]
    pub box_office_sales_in_cents: i64,
    #[sql_type = "BigInt"]
    pub online_sales_in_cents: i64,
    #[sql_type = "BigInt"]
    pub box_office_sale_count: i64,
    #[sql_type = "BigInt"]
    pub online_sale_count: i64,
    #[sql_type = "BigInt"]
    pub comp_sale_count: i64,
    #[sql_type = "BigInt"]
    pub total_box_office_fees_in_cents: i64,
    #[sql_type = "BigInt"]
    pub total_online_fees_in_cents: i64,
    #[sql_type = "BigInt"]
    pub company_box_office_fees_in_cents: i64,
    #[sql_type = "BigInt"]
    pub client_box_office_fees_in_cents: i64,
    #[sql_type = "BigInt"]
    pub company_online_fees_in_cents: i64,
    #[sql_type = "BigInt"]
    pub client_online_fees_in_cents: i64,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Queryable, QueryableByName)]
pub struct TicketCountRow {
    #[sql_type = "Nullable<dUuid>"]
    pub organization_id: Option<Uuid>,
    #[sql_type = "Nullable<dUuid>"]
    pub event_id: Option<Uuid>,
    #[sql_type = "Nullable<dUuid>"]
    pub ticket_type_id: Option<Uuid>,
    #[sql_type = "Nullable<Text>"]
    pub ticket_name: Option<String>,
    #[sql_type = "Nullable<Text>"]
    pub ticket_status: Option<String>,
    #[sql_type = "Nullable<Text>"]
    pub event_name: Option<String>,
    #[sql_type = "Nullable<Text>"]
    pub organization_name: Option<String>,
    #[sql_type = "BigInt"]
    pub allocation_count_including_nullified: i64,
    #[sql_type = "BigInt"]
    pub allocation_count: i64,
    #[sql_type = "BigInt"]
    pub unallocated_count: i64,
    #[sql_type = "BigInt"]
    pub reserved_count: i64,
    #[sql_type = "BigInt"]
    pub redeemed_count: i64,
    #[sql_type = "BigInt"]
    pub purchased_count: i64,
    #[sql_type = "BigInt"]
    pub nullified_count: i64,
    #[sql_type = "BigInt"]
    pub available_for_purchase_count: i64,
    #[sql_type = "BigInt"]
    pub total_refunded_count: i64,
    #[sql_type = "BigInt"]
    pub comp_count: i64,
    #[sql_type = "BigInt"]
    pub comp_available_count: i64,
    #[sql_type = "BigInt"]
    pub comp_redeemed_count: i64,
    #[sql_type = "BigInt"]
    pub comp_purchased_count: i64,
    #[sql_type = "BigInt"]
    pub comp_reserved_count: i64,
    #[sql_type = "BigInt"]
    pub comp_nullified_count: i64,
    #[sql_type = "BigInt"]
    pub hold_count: i64,
    #[sql_type = "BigInt"]
    pub hold_available_count: i64,
    #[sql_type = "BigInt"]
    pub hold_redeemed_count: i64,
    #[sql_type = "BigInt"]
    pub hold_purchased_count: i64,
    #[sql_type = "BigInt"]
    pub hold_reserved_count: i64,
    #[sql_type = "BigInt"]
    pub hold_nullified_count: i64,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct TicketSalesAndCounts {
    counts: Vec<TicketCountRow>,
    sales: Vec<TicketSalesRow>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Queryable, QueryableByName)]
pub struct TransactionReportRow {
    #[sql_type = "Text"]
    pub event_name: String,
    #[sql_type = "Text"]
    pub ticket_name: String,
    #[sql_type = "BigInt"]
    pub quantity: i64,
    #[sql_type = "BigInt"]
    pub actual_quantity: i64,
    #[sql_type = "BigInt"]
    pub refunded_quantity: i64,
    #[sql_type = "BigInt"]
    pub unit_price_in_cents: i64,
    #[sql_type = "BigInt"]
    pub gross: i64,
    #[sql_type = "BigInt"]
    pub company_fee_in_cents: i64,
    #[sql_type = "BigInt"]
    pub client_fee_in_cents: i64,
    #[sql_type = "BigInt"]
    pub gross_fee_in_cents: i64,
    #[sql_type = "BigInt"]
    pub gross_fee_in_cents_total: i64,
    #[sql_type = "BigInt"]
    pub event_fee_company_in_cents: i64,
    #[sql_type = "BigInt"]
    pub event_fee_client_in_cents: i64,
    #[sql_type = "BigInt"]
    pub event_fee_gross_in_cents: i64,
    #[sql_type = "BigInt"]
    pub event_fee_gross_in_cents_total: i64,
    #[sql_type = "Nullable<dUuid>"]
    pub fee_range_id: Option<Uuid>,
    #[sql_type = "Text"]
    pub order_type: OrderTypes,
    #[sql_type = "Nullable<Text>"]
    pub payment_method: Option<PaymentMethods>,
    #[sql_type = "Nullable<Text>"]
    pub payment_provider: Option<String>,
    #[sql_type = "Timestamp"]
    pub transaction_date: NaiveDateTime,
    #[sql_type = "Nullable<Text>"]
    pub redemption_code: Option<String>,
    #[sql_type = "dUuid"]
    pub order_id: Uuid,
    #[sql_type = "dUuid"]
    pub event_id: Uuid,
    #[sql_type = "dUuid"]
    pub user_id: Uuid,
    #[sql_type = "Text"]
    pub first_name: String,
    #[sql_type = "Text"]
    pub last_name: String,
    #[sql_type = "Text"]
    pub email: String,
    #[sql_type = "Nullable<Timestamp>"]
    pub event_start: Option<NaiveDateTime>,
}

#[derive(Serialize, Deserialize)]
pub struct EventSummarySalesResult {
    pub event_id: Uuid,
    pub sales: Vec<EventSummarySalesRow>,
    pub ticket_fees: Vec<EventSummaryFeesRow>,
    pub other_fees: Vec<EventSummaryOtherFees>,
}

impl Default for EventSummarySalesResult {
    fn default() -> Self {
        EventSummarySalesResult {
            event_id: Uuid::nil(),
            sales: vec![],
            ticket_fees: vec![],
            other_fees: vec![],
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Queryable, QueryableByName)]
pub struct EventSummarySalesRow {
    #[sql_type = "dUuid"]
    pub event_id: Uuid,
    #[sql_type = "Text"]
    pub ticket_name: String,
    #[sql_type = "Text"]
    pub pricing_name: String,
    #[sql_type = "BigInt"]
    pub total_client_fee_in_cents: i64,
    #[sql_type = "BigInt"]
    pub total_company_fee_in_cents: i64,
    #[sql_type = "BigInt"]
    pub price_in_cents: i64,
    #[sql_type = "BigInt"]
    pub online_count: i64,
    #[sql_type = "BigInt"]
    pub box_office_count: i64,
    #[sql_type = "BigInt"]
    pub comp_count: i64,
    #[sql_type = "BigInt"]
    pub total_sold: i64,
    #[sql_type = "BigInt"]
    pub total_gross_income_in_cents: i64,
    #[sql_type = "dUuid"]
    pub ticket_type_id: Uuid,
    #[sql_type = "dUuid"]
    pub ticket_pricing_id: Uuid,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Queryable, QueryableByName)]
pub struct EventSummaryFeesRow {
    #[sql_type = "dUuid"]
    pub event_id: Uuid,

    #[sql_type = "dUuid"]
    pub ticket_type_id: Uuid,
    #[sql_type = "dUuid"]
    pub ticket_pricing_id: Uuid,
    #[sql_type = "Text"]
    pub ticket_name: String,
    #[sql_type = "Text"]
    pub pricing_name: String,
    #[sql_type = "BigInt"]
    pub total_sold: i64,
    #[sql_type = "BigInt"]
    pub comp_count: i64,
    #[sql_type = "BigInt"]
    pub online_count: i64,
    #[sql_type = "BigInt"]
    pub price_in_cents: i64,
    #[sql_type = "BigInt"]
    pub total_company_fee_in_cents: i64,
    #[sql_type = "BigInt"]
    pub company_fee_in_cents: i64,
    #[sql_type = "BigInt"]
    pub total_client_fee_in_cents: i64,
    #[sql_type = "BigInt"]
    pub client_fee_in_cents: i64,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Queryable, QueryableByName)]
pub struct EventSummaryOtherFees {
    #[sql_type = "dUuid"]
    pub event_id: Uuid,

    #[sql_type = "BigInt"]
    pub unit_price_in_cents: i64,
    #[sql_type = "BigInt"]
    pub total_company_fee_in_cents: i64,
    #[sql_type = "BigInt"]
    pub company_fee_in_cents: i64,
    #[sql_type = "BigInt"]
    pub total_client_fee_in_cents: i64,
    #[sql_type = "BigInt"]
    pub client_fee_in_cents: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ReconciliationSummaryResult {
    pub payment_method: PaymentMethods,
    pub payment_provider: String,
    pub quantity: i64,
    pub unit_price_in_cents: i64,
    pub client_fee_in_cents: i64,
    pub event_fee_in_cents: i64,
    pub sales_total: i64,
    pub refund_quantity: i64,
    pub refund_unit_price_in_cents: i64,
    pub refund_client_fee_in_cents: i64,
    pub refund_event_fee_in_cents: i64,
    pub refund_total: i64,
    pub total: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ReconciliationFeeRangeResult {
    pub fee_schedule_id: Uuid,
    pub version: i16,
    pub fee_schedule_range_id: Uuid,
    pub min_price_in_cents: i64,
    pub upper_price_in_cents: Option<i64>,
    pub client_fee_in_cents: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ReconciliationDetailResult {
    pub payment_method: PaymentMethods,
    pub payment_provider: String,
    pub quantity: i64,
    pub unit_price_in_cents: i64,
    pub client_fee_in_cents: Vec<ReconciliationFeeRangeResult>,
    pub event_fee_in_cents: i64,
    pub sales_total: i64,
    pub refund_quantity: i64,
    pub refund_unit_price_in_cents: i64,
    pub refund_client_fee_in_cents: Vec<ReconciliationFeeRangeResult>,
    pub refund_event_fee_in_cents: i64,
    pub refund_total: i64,
    pub total: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ReconciliationDetailEventResult {
    pub event_id: Uuid,
    pub event_name: String,
    pub event_start: Option<NaiveDateTime>,
    pub entries: Vec<ReconciliationDetailResult>,
}

impl TicketSalesRow {
    pub fn fetch(
        start: Option<NaiveDateTime>,
        end: Option<NaiveDateTime>,
        group_by_ticket_type: Option<bool>,
        group_by_event_id: Option<bool>,
        group_by_hold_id: Option<bool>,
        event_id: Option<Uuid>,
        organization_id: Option<Uuid>,
        conn: &PgConnection,
    ) -> Result<Vec<TicketSalesRow>, DatabaseError> {
        let query_ticket_sales = include_str!("../queries/reports/reports_tickets_sales.sql");
        let q = diesel::sql_query(query_ticket_sales)
            .bind::<Nullable<Timestamp>, _>(start)
            .bind::<Nullable<Timestamp>, _>(end)
            .bind::<Nullable<Bool>, _>(group_by_ticket_type)
            .bind::<Nullable<Bool>, _>(group_by_event_id)
            .bind::<Nullable<Bool>, _>(group_by_hold_id)
            .bind::<Nullable<dUuid>, _>(event_id)
            .bind::<Nullable<dUuid>, _>(organization_id);

        let rows: Vec<TicketSalesRow> = q
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Could not fetch ticket sales")?;
        Ok(rows)
    }
}

impl TicketCountRow {
    pub fn fetch(
        event_id: Option<Uuid>,
        organization_id: Option<Uuid>,
        group_by_event_id: Option<bool>,
        group_by_organization_id: Option<bool>,
        conn: &PgConnection,
    ) -> Result<Vec<TicketCountRow>, DatabaseError> {
        let query_ticket_counts = include_str!("../queries/reports/reports_tickets_counts.sql");
        let q = diesel::sql_query(query_ticket_counts)
            .bind::<Nullable<dUuid>, _>(event_id)
            .bind::<Nullable<dUuid>, _>(organization_id)
            .bind::<Nullable<Bool>, _>(group_by_event_id)
            .bind::<Nullable<Bool>, _>(group_by_organization_id);

        let rows: Vec<TicketCountRow> = q
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Could not fetch ticket counts")?;

        Ok(rows)
    }
}

impl Report {
    pub fn transaction_detail_report(
        event_id: Option<Uuid>,
        organization_id: Option<Uuid>,
        start: Option<NaiveDateTime>,
        end: Option<NaiveDateTime>,
        conn: &PgConnection,
    ) -> Result<Vec<TransactionReportRow>, DatabaseError> {
        let query = include_str!("../queries/reports/reports_transaction_details.sql");
        let q = diesel::sql_query(query)
            .bind::<Nullable<dUuid>, _>(event_id)
            .bind::<Nullable<dUuid>, _>(organization_id)
            .bind::<Nullable<Timestamp>, _>(start)
            .bind::<Nullable<Timestamp>, _>(end);
        let transaction_rows: Vec<TransactionReportRow> = q
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Could not fetch report results")?;
        Ok(transaction_rows)
    }

    pub fn summary_event_report(
        event_id: Uuid,
        start: Option<NaiveDateTime>,
        end: Option<NaiveDateTime>,
        conn: &PgConnection,
    ) -> Result<EventSummarySalesResult, DatabaseError> {
        let mut results =
            Report::summary_event_report_core(Some(event_id), None, start, end, conn)?;

        let result = match results.is_empty() {
            true => {
                let mut event_summary = EventSummarySalesResult {
                    ..Default::default()
                };
                event_summary.event_id = event_id;
                event_summary
            }
            false => results.pop().unwrap(),
        };

        Ok(result)
    }

    pub fn organization_summary_report(
        organization_id: Uuid,
        start: Option<NaiveDateTime>,
        end: Option<NaiveDateTime>,
        conn: &PgConnection,
    ) -> Result<Vec<EventSummarySalesResult>, DatabaseError> {
        Report::summary_event_report_core(None, Some(organization_id), start, end, conn)
    }

    fn summary_event_report_core(
        event_id: Option<Uuid>,
        organization_id: Option<Uuid>,
        start: Option<NaiveDateTime>,
        end: Option<NaiveDateTime>,
        conn: &PgConnection,
    ) -> Result<Vec<EventSummarySalesResult>, DatabaseError> {
        //First get the sales summary
        let query_sales = include_str!("../queries/reports/reports_event_summary_sales.sql");
        let q = diesel::sql_query(query_sales)
            .bind::<Nullable<dUuid>, _>(event_id)
            .bind::<Nullable<dUuid>, _>(organization_id)
            .bind::<Nullable<Timestamp>, _>(start)
            .bind::<Nullable<Timestamp>, _>(end);

        let sales_rows: Vec<EventSummarySalesRow> = q.get_results(conn).to_db_error(
            ErrorCode::QueryError,
            "Could not fetch report sales results",
        )?;

        //If there were no sales, return immediately
        if sales_rows.is_empty() {
            let empty_result: Vec<EventSummarySalesResult> = Vec::new();
            return Ok(empty_result);
        }

        let sales_rows = sales_rows.into_iter().group_by(|row| row.event_id);

        //Now get the transaction fees results
        let query_fees = include_str!("../queries/reports/reports_event_summary_fees.sql");
        let q = diesel::sql_query(query_fees)
            .bind::<Nullable<dUuid>, _>(event_id)
            .bind::<Nullable<dUuid>, _>(organization_id)
            .bind::<Nullable<Timestamp>, _>(start)
            .bind::<Nullable<Timestamp>, _>(end);

        let fees_rows: Vec<EventSummaryFeesRow> = q
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Could not fetch report fee results")?;

        let mut fees_hash: HashMap<Uuid, Vec<EventSummaryFeesRow>> = HashMap::new();
        for row in fees_rows {
            fees_hash
                .entry(row.event_id)
                .or_insert(Vec::<EventSummaryFeesRow>::new())
                .push(row);
        }

        //Now get the other fees results
        let query_other_fees =
            include_str!("../queries/reports/reports_event_summary_other_fees.sql");
        let q = diesel::sql_query(query_other_fees)
            .bind::<Nullable<dUuid>, _>(event_id)
            .bind::<Nullable<dUuid>, _>(organization_id)
            .bind::<Nullable<Timestamp>, _>(start)
            .bind::<Nullable<Timestamp>, _>(end);

        let other_fees_rows: Vec<EventSummaryOtherFees> = q
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Could not fetch report fee results")?;

        let mut other_fees_hash: HashMap<Uuid, Vec<EventSummaryOtherFees>> = HashMap::new();
        for row in other_fees_rows {
            other_fees_hash
                .entry(row.event_id)
                .or_insert(Vec::<EventSummaryOtherFees>::new())
                .push(row);
        }

        let mut result = Vec::<EventSummarySalesResult>::new();

        // assume that an event must have sales in order to have other fees
        for (event_id, sales) in sales_rows.into_iter() {
            result.push(EventSummarySalesResult {
                event_id,
                sales: sales.into_iter().collect_vec(),
                ticket_fees: fees_hash.get(&event_id).unwrap_or(&vec![]).to_vec(),
                other_fees: other_fees_hash.get(&event_id).unwrap_or(&vec![]).to_vec(),
            })
        }
        //Then get the fees summary
        Ok(result)
    }

    pub fn ticket_count_report(
        event_id: Option<Uuid>,
        organization_id: Option<Uuid>,
        conn: &PgConnection,
    ) -> Result<TicketSalesAndCounts, DatabaseError> {
        Report::ticket_sales_and_counts(
            event_id,
            organization_id,
            None,
            None,
            false,
            false,
            false,
            conn,
        )
    }

    /// Fetches the generic ticket sales and counts data
    ///
    /// # Examples
    /// To retrieve all data for the entire system
    /// `Report::ticket_sales_and_counts(None, None, None, None, false, false, conn);`
    /// To retrieve all data for the entire organization
    /// `Report::ticket_sales_and_counts(None, Some(Uuid::new_v4()), None, None, false, false, conn);`
    /// To retrieve all data for the event
    /// `Report::ticket_sales_and_counts(Some(Uuid::new_v4()), Some(Uuid::new_v4()), None, None, false, false, conn);`
    /// To retrieve all data for the event grouped by ticket_type
    /// `Report::ticket_sales_and_counts(Some(Uuid::new_v4()), Some(Uuid::new_v4()), None, None, true, false, conn);`
    /// To retrieve all data for the event grouped by ticket_pricing
    /// `Report::ticket_sales_and_counts(Some(Uuid::new_v4()), Some(Uuid::new_v4()), None, None, false, true, conn);`
    pub fn ticket_sales_and_counts(
        event_id: Option<Uuid>,
        organization_id: Option<Uuid>,
        start: Option<NaiveDateTime>,
        end: Option<NaiveDateTime>,
        per_ticket_type: bool,
        _per_ticket_pricing: bool,
        per_hold: bool,
        conn: &PgConnection,
    ) -> Result<TicketSalesAndCounts, DatabaseError> {
        let sales = TicketSalesRow::fetch(
            start,
            end,
            Some(per_ticket_type),
            Some(false),
            Some(per_hold),
            event_id,
            organization_id,
            conn,
        )?;
        let counts =
            TicketCountRow::fetch(event_id, organization_id, Some(false), Some(false), conn)?;

        Ok(TicketSalesAndCounts { counts, sales })
    }

    pub fn reconciliation_summary_report(
        organization_id: Uuid,
        start: Option<NaiveDateTime>,
        end: Option<NaiveDateTime>,
        conn: &PgConnection,
    ) -> Result<Vec<ReconciliationSummaryResult>, DatabaseError> {
        let event_id: Option<Uuid> = None;
        let query = include_str!("../queries/reports/reports_transaction_details.sql");
        let q = diesel::sql_query(query)
            .bind::<Nullable<dUuid>, _>(event_id)
            .bind::<Nullable<dUuid>, _>(organization_id)
            .bind::<Nullable<Timestamp>, _>(start)
            .bind::<Nullable<Timestamp>, _>(end);

        let transaction_rows: Vec<TransactionReportRow> = q
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Could not fetch report results")?;

        //Produce Report
        let mut results: Vec<ReconciliationSummaryResult> = Vec::new();
        for row in transaction_rows {
            if row.payment_method.is_some() && row.payment_provider.is_some() {
                let entry_exists = results.iter().any(|r| {
                    r.payment_method == row.payment_method.clone().unwrap()
                        && r.payment_provider == row.payment_provider.clone().unwrap()
                });
                if entry_exists {
                    if let Some(entry) = results.iter_mut().find(|r| {
                        r.payment_method == row.payment_method.clone().unwrap()
                            && r.payment_provider == row.payment_provider.clone().unwrap()
                    }) {
                        let ticket_face = row.unit_price_in_cents * row.actual_quantity;
                        let client_fee = row.client_fee_in_cents * row.actual_quantity;
                        let event_fee = row.event_fee_client_in_cents * row.actual_quantity;
                        let refund_ticket_face = row.unit_price_in_cents * row.refunded_quantity;
                        let refund_client_fee = row.client_fee_in_cents * row.refunded_quantity;
                        let refund_event_fee =
                            row.event_fee_client_in_cents * row.refunded_quantity;
                        let sales_total = ticket_face + client_fee + event_fee;
                        let refund_total =
                            refund_ticket_face + refund_client_fee + refund_event_fee;
                        entry.quantity += row.actual_quantity;
                        entry.unit_price_in_cents += ticket_face;
                        entry.client_fee_in_cents += client_fee;
                        entry.event_fee_in_cents += event_fee;
                        entry.sales_total += sales_total;
                        entry.refund_quantity += row.refunded_quantity;
                        entry.refund_unit_price_in_cents += refund_ticket_face;
                        entry.refund_client_fee_in_cents += refund_client_fee;
                        entry.refund_event_fee_in_cents += refund_event_fee;
                        entry.refund_total += refund_total;
                        entry.total += sales_total - refund_total;
                    }
                } else {
                    let ticket_face = row.unit_price_in_cents * row.actual_quantity;
                    let client_fee = row.client_fee_in_cents * row.actual_quantity;
                    let event_fee = row.event_fee_client_in_cents * row.actual_quantity;
                    let refund_ticket_face = row.unit_price_in_cents * row.refunded_quantity;
                    let refund_client_fee = row.client_fee_in_cents * row.refunded_quantity;
                    let refund_event_fee = row.event_fee_client_in_cents * row.refunded_quantity;
                    let sales_total = ticket_face + client_fee + event_fee;
                    let refund_total = refund_ticket_face + refund_client_fee + refund_event_fee;
                    results.push(ReconciliationSummaryResult {
                        payment_method: row.payment_method.unwrap(),
                        payment_provider: row.payment_provider.unwrap(),
                        quantity: row.actual_quantity,
                        unit_price_in_cents: ticket_face,
                        client_fee_in_cents: client_fee,
                        event_fee_in_cents: event_fee,
                        sales_total,
                        refund_quantity: row.refunded_quantity,
                        refund_unit_price_in_cents: refund_ticket_face,
                        refund_client_fee_in_cents: refund_client_fee,
                        refund_event_fee_in_cents: refund_event_fee,
                        refund_total,
                        total: sales_total - refund_total,
                    });
                }
            }
        }

        Ok(results)
    }

    pub fn reconciliation_detail_report(
        organization_id: Uuid,
        start: Option<NaiveDateTime>,
        end: Option<NaiveDateTime>,
        conn: &PgConnection,
    ) -> Result<Vec<ReconciliationDetailEventResult>, DatabaseError> {
        let event_id: Option<Uuid> = None;
        let query = include_str!("../queries/reports/reports_transaction_details.sql");
        let q = diesel::sql_query(query)
            .bind::<Nullable<dUuid>, _>(event_id)
            .bind::<Nullable<dUuid>, _>(organization_id)
            .bind::<Nullable<Timestamp>, _>(start)
            .bind::<Nullable<Timestamp>, _>(end);

        let transaction_rows: Vec<TransactionReportRow> = q
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Could not fetch report results")?;

        //Get the fee schedule ranges for this org and construct an easy to use list of them
        let fee_schedules = FeeSchedule::find_for_organization(
            Organization::find(organization_id, conn)?.id,
            conn,
        )?;

        struct FeeScheduleWithRange {
            fee_schedule_id: Uuid,
            version: i16,
            ranges: Vec<FeeScheduleRange>,
        }

        let mut fee_schedule_with_ranges: Vec<FeeScheduleWithRange> = Vec::new();

        for fs in fee_schedules {
            fee_schedule_with_ranges.push(FeeScheduleWithRange {
                fee_schedule_id: fs.id,
                version: fs.version,
                ranges: fs.ranges(conn)?,
            });
            if fee_schedule_with_ranges.last().unwrap().ranges.len() < 1 {
                return DatabaseError::no_results(
                    "Could not find fee schedule range for this organization",
                );
            }
        }

        //Generate a vector with all the columns, this will be clones for each payment type
        let mut fee_schedule_range_columns: Vec<ReconciliationFeeRangeResult> = Vec::new();

        for fsr in fee_schedule_with_ranges {
            for idx in 0..fsr.ranges.len() {
                if idx == fsr.ranges.len() - 1 {
                    fee_schedule_range_columns.push(ReconciliationFeeRangeResult {
                        fee_schedule_id: fsr.fee_schedule_id,
                        version: fsr.version,
                        fee_schedule_range_id: fsr.ranges[idx].id,
                        min_price_in_cents: fsr.ranges[idx].min_price_in_cents,
                        upper_price_in_cents: None,
                        client_fee_in_cents: 0,
                    });
                } else {
                    fee_schedule_range_columns.push(ReconciliationFeeRangeResult {
                        fee_schedule_id: fsr.fee_schedule_id,
                        version: fsr.version,
                        fee_schedule_range_id: fsr.ranges[idx].id,
                        min_price_in_cents: fsr.ranges[idx].min_price_in_cents,
                        upper_price_in_cents: Some(fsr.ranges[idx + 1].min_price_in_cents - 1),
                        client_fee_in_cents: 0,
                    });
                }
            }
        }

        //Produce Report
        let mut results: Vec<ReconciliationDetailEventResult> = Vec::new();

        for row in transaction_rows {
            let event_exists = results.iter().any(|r| r.event_id == row.event_id);

            if !event_exists {
                results.push(ReconciliationDetailEventResult {
                    event_id: row.event_id.clone(),
                    event_name: row.event_name.clone(),
                    event_start: row.event_start.clone(),
                    entries: Vec::new(),
                });
            }

            if let Some(event_entry) = results.iter_mut().find(|ref r| r.event_id == row.event_id) {
                if row.payment_method.is_some() && row.payment_provider.is_some() {
                    let entry_exists = event_entry.entries.iter().any(|r| {
                        r.payment_method == row.payment_method.clone().unwrap()
                            && r.payment_provider == row.payment_provider.clone().unwrap()
                    });

                    //Which fee range column does this row's transaction fall in?
                    let mut column_idx: Option<usize> = None;
                    if let Some(fee_range_id) = row.fee_range_id {
                        for (idx, frc) in fee_schedule_range_columns.iter().enumerate() {
                            if frc.fee_schedule_range_id == fee_range_id {
                                column_idx = Some(idx);
                                break;
                            }
                        }
                    }

                    if let Some(column_idx) = column_idx {
                        if entry_exists {
                            if let Some(entry) = event_entry.entries.iter_mut().find(|r| {
                                r.payment_method == row.payment_method.clone().unwrap()
                                    && r.payment_provider == row.payment_provider.clone().unwrap()
                            }) {
                                let ticket_face = row.unit_price_in_cents * row.actual_quantity;
                                let client_fee = row.client_fee_in_cents * row.actual_quantity;
                                let event_fee = row.event_fee_client_in_cents * row.actual_quantity;
                                let refund_ticket_face =
                                    row.unit_price_in_cents * row.refunded_quantity;
                                let refund_client_fee =
                                    row.client_fee_in_cents * row.refunded_quantity;
                                let refund_event_fee =
                                    row.event_fee_client_in_cents * row.refunded_quantity;
                                let sales_total = ticket_face + client_fee + event_fee;
                                let refund_total =
                                    refund_ticket_face + refund_client_fee + refund_event_fee;

                                entry.quantity += row.actual_quantity;
                                entry.unit_price_in_cents += ticket_face;
                                entry.client_fee_in_cents[column_idx].client_fee_in_cents +=
                                    client_fee;
                                entry.event_fee_in_cents += event_fee;
                                entry.sales_total += sales_total;
                                entry.refund_quantity += row.refunded_quantity;
                                entry.refund_unit_price_in_cents += refund_ticket_face;
                                entry.refund_client_fee_in_cents[column_idx].client_fee_in_cents +=
                                    refund_client_fee;
                                entry.refund_event_fee_in_cents += refund_event_fee;
                                entry.refund_total += refund_total;
                                entry.total += sales_total - refund_total;
                            }
                        } else {
                            let ticket_face = row.unit_price_in_cents * row.actual_quantity;
                            let client_fee = row.client_fee_in_cents * row.actual_quantity;
                            let event_fee = row.event_fee_client_in_cents * row.actual_quantity;
                            let refund_ticket_face =
                                row.unit_price_in_cents * row.refunded_quantity;
                            let refund_client_fee = row.client_fee_in_cents * row.refunded_quantity;
                            let refund_event_fee =
                                row.event_fee_client_in_cents * row.refunded_quantity;
                            let sales_total = ticket_face + client_fee + event_fee;
                            let refund_total =
                                refund_ticket_face + refund_client_fee + refund_event_fee;

                            let mut client_fee_in_cents = fee_schedule_range_columns.clone();
                            let mut refund_client_fee_in_cents = fee_schedule_range_columns.clone();

                            client_fee_in_cents[column_idx].client_fee_in_cents = client_fee;
                            refund_client_fee_in_cents[column_idx].client_fee_in_cents =
                                refund_client_fee;

                            event_entry.entries.push(ReconciliationDetailResult {
                                payment_method: row.payment_method.unwrap(),
                                payment_provider: row.payment_provider.unwrap(),
                                quantity: row.actual_quantity,
                                unit_price_in_cents: ticket_face,
                                client_fee_in_cents,
                                event_fee_in_cents: event_fee,
                                sales_total,
                                refund_quantity: row.refunded_quantity,
                                refund_unit_price_in_cents: refund_ticket_face,
                                refund_client_fee_in_cents,
                                refund_event_fee_in_cents: refund_event_fee,
                                refund_total,
                                total: sales_total - refund_total,
                            });
                        }
                    }
                }
            }
        }

        Ok(results)
    }
}
