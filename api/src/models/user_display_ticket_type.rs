use bigneon_db::prelude::*;
use chrono::{NaiveDateTime, Utc};
use diesel::PgConnection;
use models::DisplayTicketPricing;
use uuid::Uuid;

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct UserDisplayTicketType {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub status: TicketTypeStatus,
    pub available: u32,
    pub start_date: NaiveDateTime,
    pub end_date: NaiveDateTime,
    pub increment: i32,
    pub limit_per_person: u32,
    pub ticket_pricing: Option<DisplayTicketPricing>,
    pub redemption_code: Option<String>,
    pub event_id: Uuid,
}

impl UserDisplayTicketType {
    pub fn from_ticket_type(
        ticket_type: &TicketType,
        fee_schedule: &FeeSchedule,
        box_office_pricing: bool,
        redemption_code: Option<String>,
        conn: &PgConnection,
    ) -> Result<UserDisplayTicketType, DatabaseError> {
        let status = ticket_type.status;
        let available = ticket_type.valid_available_ticket_count(conn)?;

        let ticket_pricing = match ticket_type
            .current_ticket_pricing(box_office_pricing, conn)
            .optional()?
        {
            Some(ticket_pricing) => Some(DisplayTicketPricing::from_ticket_pricing(
                &ticket_pricing,
                fee_schedule,
                redemption_code.clone(),
                box_office_pricing,
                conn,
            )?),
            None => None,
        };

        let mut result = UserDisplayTicketType {
            id: ticket_type.id,
            event_id: ticket_type.event_id,
            name: ticket_type.name.clone(),
            description: ticket_type.description.clone(),
            status,
            start_date: ticket_type.start_date,
            end_date: ticket_type.end_date,
            ticket_pricing,
            available,
            redemption_code: None,
            increment: ticket_type.increment,
            limit_per_person: ticket_type.limit_per_person as u32,
        };

        if let Some(ref redemption_code) = redemption_code {
            if let Some(hold) = Hold::find_by_redemption_code(redemption_code, conn).optional()? {
                if hold.ticket_type_id == ticket_type.id {
                    result.description = Some(format!("Using promo code: {}", redemption_code));
                    result.limit_per_person = hold.max_per_user.unwrap_or(0) as u32;
                    result.available = hold.quantity(conn)?.1;
                    result.redemption_code = Some(redemption_code.clone());
                }
            } else if let Some(code) =
                Code::find_by_redemption_code(redemption_code, conn).optional()?
            {
                let now = Utc::now().naive_utc();
                if now >= code.start_date && now <= code.end_date {
                    if TicketType::find_for_code(code.id, conn)?.contains(&ticket_type) {
                        result.description = Some(format!("Using promo code: {}", redemption_code));
                        result.limit_per_person = code.max_tickets_per_user.unwrap_or(0) as u32;
                        result.redemption_code = Some(redemption_code.clone());
                    }
                }
            }
        }

        if ticket_type.status == TicketTypeStatus::Published {
            if result.available == 0 {
                result.status = TicketTypeStatus::SoldOut;
            } else if result.ticket_pricing.is_none() {
                result.status = TicketTypeStatus::NoActivePricing;
            }
        }

        Ok(result)
    }
}
