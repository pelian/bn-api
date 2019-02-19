use bigneon_db::prelude::*;
use chrono::NaiveDateTime;
use diesel::PgConnection;
use models::DisplayTicketPricing;
use uuid::Uuid;

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct AdminDisplayTicketType {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub capacity: u32,
    pub status: TicketTypeStatus,
    pub start_date: NaiveDateTime,
    pub end_date: NaiveDateTime,
    pub available: u32,
    pub increment: u32,
    pub limit_per_person: u32,
    pub ticket_pricing: Vec<DisplayTicketPricing>,
    pub price_in_cents: i64,
    pub sold_out_behavior: SoldOutBehavior,
}

impl AdminDisplayTicketType {
    pub fn from_ticket_type(
        ticket_type: &TicketType,
        fee_schedule: &FeeSchedule,
        conn: &PgConnection,
    ) -> Result<AdminDisplayTicketType, DatabaseError> {
        let available = ticket_type.valid_available_ticket_count(conn)?;
        let capacity = ticket_type.valid_ticket_count(conn)?;
        let mut ticket_pricing_list = Vec::new();
        for ticket_pricing in ticket_type.valid_ticket_pricing(false, conn)? {
            ticket_pricing_list.push(DisplayTicketPricing::from_ticket_pricing(
                &ticket_pricing,
                fee_schedule,
                None,
                false,
                conn,
            )?);
        }

        Ok(AdminDisplayTicketType {
            id: ticket_type.id,
            name: ticket_type.name.clone(),
            description: ticket_type.description.clone(),
            status: ticket_type.status,
            start_date: ticket_type.start_date,
            end_date: ticket_type.end_date,
            ticket_pricing: ticket_pricing_list,
            available,
            capacity,
            increment: ticket_type.increment as u32,
            limit_per_person: ticket_type.limit_per_person as u32,
            price_in_cents: ticket_type.price_in_cents,
            sold_out_behavior: ticket_type.sold_out_behavior,
        })
    }
}
