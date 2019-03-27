use chrono::prelude::*;
use diesel;
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Nullable, Text, Timestamp, Uuid as dUuid};
use itertools::Itertools;
use models::*;
use std::collections::HashMap;
use utils::errors::*;
use uuid::Uuid;

sql_function!(fn ticket_sales_per_ticket_pricing(start: Nullable<Timestamp>, end: Nullable<Timestamp>, group_by: Option<Text>) -> Vec<TicketSalesRow>);
sql_function!(fn ticket_count_per_ticket_type(event_id: Nullable<dUuid>, organization_id: Nullable<dUuid>, group_by: Option<Text>) -> Vec<TicketCountRow>);
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
    #[sql_type = "Nullable<BigInt>"]
    pub ticket_pricing_price_in_cents: Option<i64>,
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
    #[sql_type = "BigInt"]
    pub per_order_company_online_fees: i64,
    #[sql_type = "BigInt"]
    pub per_order_client_online_fees: i64,
    #[sql_type = "BigInt"]
    pub per_order_total_fees_in_cents: i64,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize, PartialEq, Queryable, QueryableByName)]
pub struct TicketSalesPerEventFees {
    #[sql_type = "Nullable<dUuid>"]
    pub organization_id: Option<Uuid>,
    #[sql_type = "Nullable<dUuid>"]
    pub event_id: Option<Uuid>,
    #[sql_type = "BigInt"]
    pub per_order_company_online_fees: i64,
    #[sql_type = "BigInt"]
    pub per_order_client_online_fees: i64,
    #[sql_type = "BigInt"]
    pub per_order_total_fees_in_cents: i64,
}

impl From<TicketSalesPerEventFees> for TicketSalesRow {
    fn from(ticket_sales_per_event_fees: TicketSalesPerEventFees) -> Self {
        TicketSalesRow {
            organization_id: ticket_sales_per_event_fees.organization_id,
            event_id: ticket_sales_per_event_fees.event_id,
            per_order_company_online_fees: ticket_sales_per_event_fees
                .per_order_company_online_fees,
            per_order_client_online_fees: ticket_sales_per_event_fees.per_order_client_online_fees,
            per_order_total_fees_in_cents: ticket_sales_per_event_fees
                .per_order_total_fees_in_cents,
            ..Default::default()
        }
    }
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
    pub counts: Vec<TicketCountRow>,
    pub sales: Vec<TicketSalesRow>,
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
pub struct BoxOfficeSalesSummaryReportRow {
    #[sql_type = "dUuid"]
    pub operator_id: Uuid,
    #[sql_type = "Text"]
    pub operator_name: String,
    #[sql_type = "Nullable<Text>"]
    pub event_name: Option<String>,
    #[sql_type = "Nullable<Timestamp>"]
    pub event_date: Option<NaiveDateTime>,
    #[sql_type = "Text"]
    pub external_payment_type: ExternalPaymentType,
    #[sql_type = "BigInt"]
    pub number_of_tickets: i64,
    #[sql_type = "BigInt"]
    pub face_value_in_cents: i64,
    #[sql_type = "BigInt"]
    pub total_fees_in_cents: i64,
    #[sql_type = "BigInt"]
    pub total_sales_in_cents: i64,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct BoxOfficeSalesSummaryReport {
    pub operators: Vec<BoxOfficeSalesSummaryOperatorRow>,
    pub payments: Vec<BoxOfficeSalesSummaryPaymentRow>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct BoxOfficeSalesSummaryPaymentRow {
    pub payment_type: String,
    pub quantity: u32,
    pub total_sales_in_cents: u32,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct BoxOfficeSalesSummaryOperatorRow {
    pub operator_id: Uuid,
    pub operator_name: String,
    pub events: Vec<BoxOfficeSalesSummaryOperatorEventRow>,
    pub payments: Vec<BoxOfficeSalesSummaryPaymentRow>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct BoxOfficeSalesSummaryOperatorEventRow {
    pub event_name: Option<String>,
    pub event_date: Option<NaiveDateTime>,
    pub number_of_tickets: u32,
    pub face_value_in_cents: u32,
    pub total_fees_in_cents: u32,
    pub total_sales_in_cents: u32,
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

pub fn group_by_string(
    group_by_ticket_type: bool,
    group_by_ticket_pricing: bool,
    group_by_hold: bool,
    group_by_event: bool,
) -> Option<String> {
    let mut group_by = vec![];
    if group_by_ticket_type {
        group_by.push("ticket_type".to_string());
    }
    if group_by_ticket_pricing {
        group_by.push("ticket_pricing".to_string());
    }
    if group_by_hold {
        group_by.push("hold".to_string());
    }
    if group_by_event {
        group_by.push("event".to_string());
    }
    match group_by.len() {
        0 => None,
        _ => Some(group_by.join("|")),
    }
}

impl TicketSalesRow {
    pub fn fetch(
        start: Option<NaiveDateTime>,
        end: Option<NaiveDateTime>,
        group_by_ticket_type: bool,
        group_by_ticket_pricing: bool,
        group_by_hold: bool,
        event_id: Option<Uuid>,
        organization_id: Option<Uuid>,
        conn: &PgConnection,
    ) -> Result<Vec<TicketSalesRow>, DatabaseError> {
        let group_by = group_by_string(
            group_by_ticket_type,
            group_by_ticket_pricing,
            group_by_hold,
            false,
        );
        let query_ticket_sales = include_str!("../queries/reports/reports_tickets_sales.sql");
        let q = diesel::sql_query(query_ticket_sales)
            .bind::<Nullable<Timestamp>, _>(start)
            .bind::<Nullable<Timestamp>, _>(end)
            .bind::<Nullable<Text>, _>(group_by)
            .bind::<Nullable<dUuid>, _>(event_id)
            .bind::<Nullable<dUuid>, _>(organization_id);

        let mut rows: Vec<TicketSalesRow> = q
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Could not fetch ticket sales")?;

        let event_fees = TicketSalesRow::fetch_per_event_fees(
            start,
            end,
            true,
            event_id,
            organization_id,
            conn,
        )?;
        rows.extend(event_fees);
        Ok(rows)
    }

    pub fn fetch_per_event_fees(
        start: Option<NaiveDateTime>,
        end: Option<NaiveDateTime>,
        group_by_event: bool,
        event_id: Option<Uuid>,
        organization_id: Option<Uuid>,
        conn: &PgConnection,
    ) -> Result<Vec<TicketSalesRow>, DatabaseError> {
        let group_by = group_by_string(false, false, false, group_by_event);
        let query_ticket_sales = include_str!("../queries/reports/reports_event_fees.sql");
        let q = diesel::sql_query(query_ticket_sales)
            .bind::<Nullable<Timestamp>, _>(start)
            .bind::<Nullable<Timestamp>, _>(end)
            .bind::<Nullable<Text>, _>(group_by)
            .bind::<Nullable<dUuid>, _>(event_id)
            .bind::<Nullable<dUuid>, _>(organization_id);

        let rows: Vec<TicketSalesPerEventFees> = q
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Could not fetch ticket sales")?;
        let rows = rows.into_iter().map(|x| x.into()).collect();
        Ok(rows)
    }
}

impl TicketCountRow {
    pub fn fetch(
        event_id: Option<Uuid>,
        organization_id: Option<Uuid>,
        group_by_event: bool,
        group_by_ticket_type: bool,
        conn: &PgConnection,
    ) -> Result<Vec<TicketCountRow>, DatabaseError> {
        let group_by = group_by_string(group_by_ticket_type, false, false, group_by_event);
        let query_ticket_counts = include_str!("../queries/reports/reports_tickets_counts.sql");
        let q = diesel::sql_query(query_ticket_counts)
            .bind::<Nullable<dUuid>, _>(event_id)
            .bind::<Nullable<dUuid>, _>(organization_id)
            .bind::<Nullable<Text>, _>(group_by);

        let rows: Vec<TicketCountRow> = q
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Could not fetch ticket counts")?;

        Ok(rows)
    }
}

impl Report {
    fn external_payment_type_row_title(external_payment_type: ExternalPaymentType) -> String {
        match external_payment_type {
            ExternalPaymentType::Cash => "Cash".to_string(),
            ExternalPaymentType::CreditCard => "Credit Card".to_string(),
            ExternalPaymentType::Voucher => "Voucher".to_string(),
        }
    }

    pub fn box_office_sales_summary_report(
        organization_id: Uuid,
        start: Option<NaiveDateTime>,
        end: Option<NaiveDateTime>,
        conn: &PgConnection,
    ) -> Result<BoxOfficeSalesSummaryReport, DatabaseError> {
        let query = include_str!("../queries/reports/reports_box_office_sales_summary_report.sql");
        let q = diesel::sql_query(query)
            .bind::<dUuid, _>(organization_id)
            .bind::<Nullable<Timestamp>, _>(start)
            .bind::<Nullable<Timestamp>, _>(end);
        let box_office_summary_rows: Vec<BoxOfficeSalesSummaryReportRow> = q
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Could not fetch report results")?;

        let mut payment_totals: HashMap<ExternalPaymentType, BoxOfficeSalesSummaryPaymentRow> =
            HashMap::new();
        for external_payment_type in vec![
            ExternalPaymentType::Cash,
            ExternalPaymentType::CreditCard,
            ExternalPaymentType::Voucher,
        ] {
            payment_totals.insert(
                external_payment_type,
                BoxOfficeSalesSummaryPaymentRow {
                    payment_type: Report::external_payment_type_row_title(external_payment_type),
                    quantity: 0,
                    total_sales_in_cents: 0,
                },
            );
        }

        let mut operator_data: Vec<BoxOfficeSalesSummaryOperatorRow> = Vec::new();
        for (operator_id, group) in &box_office_summary_rows
            .into_iter()
            .group_by(|row| row.operator_id)
        {
            let mut operator_name = None;
            let mut event_date = None;
            let mut payments: HashMap<ExternalPaymentType, BoxOfficeSalesSummaryPaymentRow> =
                HashMap::new();
            for external_payment_type in vec![
                ExternalPaymentType::Cash,
                ExternalPaymentType::CreditCard,
                ExternalPaymentType::Voucher,
            ] {
                payments.insert(
                    external_payment_type,
                    BoxOfficeSalesSummaryPaymentRow {
                        payment_type: Report::external_payment_type_row_title(
                            external_payment_type,
                        ),
                        quantity: 0,
                        total_sales_in_cents: 0,
                    },
                );
            }

            let mut events: Vec<BoxOfficeSalesSummaryOperatorEventRow> = Vec::new();
            for (event_name, event_group) in
                &group.into_iter().group_by(|row| row.event_name.clone())
            {
                let mut number_of_tickets = 0;
                let mut face_value_in_cents = 0;
                let mut total_fees_in_cents = 0;
                let mut total_sales_in_cents = 0;
                for event_group_item in event_group {
                    operator_name = Some(event_group_item.operator_name.clone());
                    event_date = event_group_item.event_date.clone();
                    number_of_tickets += event_group_item.number_of_tickets as u32;
                    face_value_in_cents = event_group_item.face_value_in_cents as u32;
                    total_fees_in_cents += event_group_item.total_fees_in_cents as u32;
                    total_sales_in_cents += event_group_item.total_sales_in_cents as u32;
                    match payment_totals.get_mut(&event_group_item.external_payment_type) {
                        Some(totals) => {
                            totals.quantity += event_group_item.number_of_tickets as u32;
                            totals.total_sales_in_cents +=
                                event_group_item.total_sales_in_cents as u32;
                        }
                        None => {
                            return DatabaseError::business_process_error(
                                "Cannot delete an order item for an order that is not in draft",
                            );
                        }
                    }

                    match payments.get_mut(&event_group_item.external_payment_type) {
                        Some(totals) => {
                            totals.quantity += event_group_item.number_of_tickets as u32;
                            totals.total_sales_in_cents +=
                                event_group_item.total_sales_in_cents as u32;
                        }
                        None => {
                            return DatabaseError::business_process_error(
                                "Cannot delete an order item for an order that is not in draft",
                            );
                        }
                    }
                }

                events.push(BoxOfficeSalesSummaryOperatorEventRow {
                    event_name,
                    event_date,
                    number_of_tickets,
                    face_value_in_cents,
                    total_fees_in_cents,
                    total_sales_in_cents,
                });
            }

            let mut payments = payments
                .values()
                .map(|v| (*v).clone())
                .collect::<Vec<BoxOfficeSalesSummaryPaymentRow>>();
            payments.sort_by_key(|p| p.payment_type.to_string());

            operator_data.push(BoxOfficeSalesSummaryOperatorRow {
                operator_id,
                operator_name: operator_name.unwrap_or("".to_string()),
                events,
                payments,
            })
        }

        let mut payment_totals = payment_totals
            .values()
            .map(|v| (*v).clone())
            .collect::<Vec<BoxOfficeSalesSummaryPaymentRow>>();
        payment_totals.sort_by_key(|p| p.payment_type.to_string());

        Ok(BoxOfficeSalesSummaryReport {
            operators: operator_data,
            payments: payment_totals,
        })
    }

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
            true,
            true,
            true,
            false,
            conn,
        )
    }

    /// Fetches the generic ticket sales and counts data
    pub fn ticket_sales_and_counts(
        event_id: Option<Uuid>,
        organization_id: Option<Uuid>,
        start: Option<NaiveDateTime>,
        end: Option<NaiveDateTime>,
        group_by_ticket_type: bool,
        group_by_ticket_pricing: bool,
        group_by_hold: bool,
        group_by_event: bool,
        conn: &PgConnection,
    ) -> Result<TicketSalesAndCounts, DatabaseError> {
        let sales = TicketSalesRow::fetch(
            start,
            end,
            group_by_ticket_type,
            group_by_ticket_pricing,
            group_by_hold,
            event_id,
            organization_id,
            conn,
        )?;

        let counts = TicketCountRow::fetch(
            event_id,
            organization_id,
            group_by_event,
            group_by_ticket_type,
            conn,
        )?;

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
