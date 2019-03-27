use actix_web::HttpResponse;
use actix_web::Path;
use bigneon_db::prelude::*;
use chrono::NaiveDateTime;
use db::Connection;
use errors::BigNeonError;
use helpers::application;
use models::*;

#[derive(Deserialize)]
pub struct PathParameters {
    pub code: String,
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum RedemptionCodeResponse {
    Hold {
        ticket_type: UserDisplayTicketType,
        redemption_code: Option<String>,
        max_per_user: Option<i64>,
        discount_in_cents: Option<i64>,
        hold_type: HoldTypes,
    },
    Code {
        ticket_types: Vec<UserDisplayTicketType>,
        redemption_code: String,
        max_uses: i64,
        discount_in_cents: Option<i64>,
        code_type: CodeTypes,
        start_date: NaiveDateTime,
        end_date: NaiveDateTime,
        max_tickets_per_user: Option<i64>,
    },
}

pub fn show(
    (connection, path): (Connection, Path<PathParameters>),
) -> Result<HttpResponse, BigNeonError> {
    let conn = connection.get();

    let response =
        if let Some(hold) = Hold::find_by_redemption_code(&path.code.clone(), conn).optional()? {
            let ticket_type = UserDisplayTicketType::from_ticket_type(
                &TicketType::find(hold.ticket_type_id, conn)?,
                &FeeSchedule::find(
                    Organization::find_for_event(hold.event_id, conn)?.fee_schedule_id,
                    conn,
                )?,
                false,
                hold.redemption_code.clone(),
                conn,
            )?;

            let discount_in_cents = ticket_type
                .ticket_pricing
                .clone()
                .map(|tp| tp.discount_in_cents);
            RedemptionCodeResponse::Hold {
                ticket_type,
                redemption_code: hold.redemption_code.clone(),
                max_per_user: hold.max_per_user,
                discount_in_cents,
                hold_type: hold.hold_type,
            }
        } else if let Some(code) = Code::find_by_redemption_code(&path.code, conn).optional()? {
            let mut ticket_types = Vec::new();
            for ticket_type in TicketType::find_for_code(code.id, conn)? {
                ticket_types.push(UserDisplayTicketType::from_ticket_type(
                    &ticket_type,
                    &FeeSchedule::find(
                        Organization::find_for_event(code.event_id, conn)?.fee_schedule_id,
                        conn,
                    )?,
                    false,
                    // Passing None for redemption_code as it makes this discount inclusive and we're breaking apart discount here
                    Some(code.redemption_code.clone()),
                    conn,
                )?);
            }
            RedemptionCodeResponse::Code {
                ticket_types,
                redemption_code: code.redemption_code.clone(),
                max_uses: code.max_uses,
                discount_in_cents: code.discount_in_cents,
                code_type: code.code_type,
                start_date: code.start_date,
                end_date: code.end_date,
                max_tickets_per_user: code.max_tickets_per_user,
            }
        } else {
            return application::not_found();
        };
    return Ok(HttpResponse::Ok().json(response));
}
