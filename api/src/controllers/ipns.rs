use actix_web::HttpResponse;
use bigneon_db::prelude::*;
use chrono::prelude::*;
use chrono::Duration;
use db::Connection;
use errors::BigNeonError;
use extractors::Json;
use globee::GlobeeIpnRequest;
use log::Level::Debug;
use uuid::Uuid;

pub fn globee(
    (data, conn): (Json<GlobeeIpnRequest>, Connection),
) -> Result<HttpResponse, BigNeonError> {
    let data = data.into_inner();
    jlog!(Debug, "Globee IPN received", { "data": &data });
    let order_id = match data.custom_payment_id {
        Some(ref id) => Some(id.parse::<Uuid>()?),
        None => None,
    };
    // At this point, just log the action so that we can retry it locally.
    // The order associated with this payment may expire before the IPN is received, but we must
    // still record that it happened. If the order has already expired or paid, the payment will be
    // recorded, so that it can be used as store credit in future.
    DomainAction::create(
        None,
        DomainActionTypes::PaymentProviderIPN,
        None,
        json!(data),
        Some(Tables::Orders.to_string()),
        order_id,
        Utc::now().naive_utc(),
        // Technically this IPN must be processed and should never expire
        (Utc::now().naive_utc())
            .checked_add_signed(Duration::days(30))
            .unwrap(),
        5,
    )
    .commit(conn.get())?;

    Ok(HttpResponse::Ok().finish())
}
