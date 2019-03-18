use actix_web::State;
use actix_web::{HttpResponse, Path, Query};
use auth::user::User;
use bigneon_db::models::User as DbUser;
use bigneon_db::prelude::*;
use chrono::prelude::*;
use communications::{mailers, pushers, smsers};
use db::Connection;
use diesel::pg::PgConnection;
use errors::*;
use extractors::*;
use helpers::application;
use itertools::Itertools;
use models::{OptionalPathParameters, PathParameters};
use regex::Regex;
use serde_json::Value;
use server::AppState;
use std::collections::HashMap;
use tari_client::TariClient;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct SearchParameters {
    pub start_utc: Option<NaiveDateTime>,
    pub end_utc: Option<NaiveDateTime>,
}
impl From<SearchParameters> for Paging {
    fn from(s: SearchParameters) -> Paging {
        let mut default_tags: HashMap<String, Value> = HashMap::new();

        if let Some(ref i) = s.start_utc {
            default_tags.insert("start_utc".to_owned(), json!(i));
        }
        if let Some(ref i) = s.end_utc {
            default_tags.insert("end_utc".to_owned(), json!(i));
        }

        Paging {
            page: 0,
            limit: 100,
            sort: "".to_owned(),
            dir: SortingDir::Asc,
            total: 0,
            tags: default_tags,
        }
    }
}

pub fn index(
    (connection, path, query, auth_user): (
        Connection,
        Path<OptionalPathParameters>,
        Query<SearchParameters>,
        User,
    ),
) -> Result<HttpResponse, BigNeonError> {
    //todo convert to use pagingparams

    let connection = connection.get();

    let tickets = TicketInstance::find_for_user_for_display(
        auth_user.id(),
        path.id,
        query.start_utc,
        query.end_utc,
        connection,
    )?;
    let query: Paging = query.into_inner().into();

    let payload = Payload::new(tickets, query.clone());
    // If specifying event drill into tuple vector to return tickets alone
    if path.id.is_some() && !payload.data.is_empty() {
        let payload2 = Payload::new(payload.data[0].1.clone(), query);
        return Ok(HttpResponse::Ok().json(&payload2));
    }

    Ok(HttpResponse::Ok().json(&payload))
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct ShowTicketResponse {
    pub event: DisplayEvent,
    pub user: Option<DisplayUser>,
    pub ticket: DisplayTicket,
}

pub fn show(
    (connection, parameters, auth_user): (Connection, Path<PathParameters>, User),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let (event, user, ticket) = TicketInstance::find_for_display(parameters.id, connection)?;
    let db_event = Event::find(event.id, connection)?;
    let organization = db_event.organization(connection)?;

    if user.as_ref().map_or(false, |u| u.id != auth_user.id()) {
        auth_user.requires_scope_for_organization(Scopes::TicketRead, &organization, connection)?;
    }

    let ticket_response = ShowTicketResponse {
        event,
        user,
        ticket,
    };

    Ok(HttpResponse::Ok().json(&ticket_response))
}

pub fn show_redeemable_ticket(
    (connection, parameters, auth_user): (Connection, Path<PathParameters>, User),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let (event, user, _ticket) = TicketInstance::find_for_display(parameters.id, connection)?;
    let db_event = Event::find(event.id, connection)?;
    let organization = db_event.organization(connection)?;

    if user.as_ref().map_or(false, |u| u.id != auth_user.id()) {
        auth_user.requires_scope_for_organization(Scopes::TicketRead, &organization, connection)?;
    }

    let redeemable_ticket = TicketInstance::show_redeemable_ticket(parameters.id, connection)?;

    Ok(HttpResponse::Ok().json(&redeemable_ticket))
}

pub fn send_via_email_or_phone(
    (connection, send_tickets_request, auth_user, state): (
        Connection,
        Json<SendTicketsRequest>,
        User,
        State<AppState>,
    ),
) -> Result<HttpResponse, BigNeonError> {
    auth_user.requires_scope(Scopes::TicketTransfer)?;
    let connection = connection.get();

    let re = Regex::new(r"[^0-9\+]+").unwrap();
    let numbers_only = re.replace_all(&send_tickets_request.email_or_phone, "");

    if send_tickets_request.email_or_phone.contains("@") {
        let existing_user =
            DbUser::find_by_email(&send_tickets_request.email_or_phone, connection).optional()?;
        if let Some(user) = existing_user {
            let ticket_instances =
                TicketInstance::find_by_ids(&send_tickets_request.ticket_ids, connection)?;

            TicketInstance::direct_transfer(
                auth_user.id(),
                &send_tickets_request.ticket_ids,
                &send_tickets_request.email_or_phone,
                "Email",
                user.id,
                connection,
            )?;

            let receiver_wallet = user.default_wallet(connection)?;

            // TODO change blockchain client to do transfers from multiple wallets at once
            for (sender_wallet_id, tickets) in
                &ticket_instances.into_iter().group_by(|ti| ti.wallet_id)
            {
                let sender_wallet = Wallet::find(sender_wallet_id, connection)?;
                transfer_tickets_on_blockchain(
                    &tickets.collect_vec(),
                    connection,
                    &*state.config.tari_client,
                    &sender_wallet,
                    &receiver_wallet,
                )?;
            }

            pushers::tickets_received(&user, &auth_user.user, connection)?;
        } else {
            let authorization = TicketInstance::authorize_ticket_transfer(
                auth_user.id(),
                &send_tickets_request.ticket_ids,
                send_tickets_request
                    .validity_period_in_seconds
                    .unwrap_or(604_800) as u32,
                Some(&send_tickets_request.email_or_phone),
                Some("Email"),
                connection,
            )?;
            mailers::tickets::send_tickets(
                &state.config,
                send_tickets_request.email_or_phone.clone(),
                &authorization.sender_user_id.to_string(),
                authorization.num_tickets,
                &authorization.transfer_key.to_string(),
                &authorization.signature,
                &auth_user.user,
                connection,
            )?;
        }
    } else if numbers_only.len() > 7 {
        let authorization = TicketInstance::authorize_ticket_transfer(
            auth_user.id(),
            &send_tickets_request.ticket_ids,
            send_tickets_request
                .validity_period_in_seconds
                .unwrap_or(604_800) as u32,
            Some(&send_tickets_request.email_or_phone),
            Some("Phone"),
            connection,
        )?;
        smsers::tickets::send_tickets(
            &state.config,
            send_tickets_request.email_or_phone.clone(),
            &authorization.sender_user_id.to_string(),
            authorization.num_tickets,
            &authorization.transfer_key.to_string(),
            &authorization.signature,
            &auth_user.user,
            connection,
            &*state.service_locator.create_deep_linker()?,
        )?;
    } else {
        return application::unprocessable(
            "Invalid destination, please supply valid phone number or email address.",
        );
    }

    Ok(HttpResponse::Ok().finish())
}

#[derive(Clone, Deserialize, Serialize)]
pub struct SendTicketsRequest {
    pub ticket_ids: Vec<Uuid>,
    pub validity_period_in_seconds: Option<i64>,
    pub email_or_phone: String,
}

pub fn transfer_authorization(
    (connection, transfer_tickets_request, auth_user): (
        Connection,
        Json<TransferTicketRequest>,
        User,
    ),
) -> Result<HttpResponse, BigNeonError> {
    auth_user.requires_scope(Scopes::TicketTransfer)?;
    let connection = connection.get();

    let transfer_authorization = TicketInstance::authorize_ticket_transfer(
        auth_user.id(),
        &transfer_tickets_request.ticket_ids,
        transfer_tickets_request.validity_period_in_seconds as u32,
        None,
        None,
        connection,
    )?;

    Ok(HttpResponse::Ok().json(&transfer_authorization))
}

pub fn receive_transfer(
    (connection, transfer_authorization, auth_user, state): (
        Connection,
        Json<TransferAuthorization>,
        User,
        State<AppState>,
    ),
) -> Result<HttpResponse, BigNeonError> {
    auth_user.requires_scope(Scopes::TicketTransfer)?;
    let connection = connection.get();

    let sender_wallet =
        Wallet::find_default_for_user(transfer_authorization.sender_user_id, connection)?;
    let receiver_wallet = Wallet::find_default_for_user(auth_user.id(), connection)?;

    let tickets = TicketInstance::receive_ticket_transfer(
        transfer_authorization.into_inner(),
        &sender_wallet,
        receiver_wallet.id,
        connection,
    )?;

    transfer_tickets_on_blockchain(
        &tickets,
        connection,
        &*state.config.tari_client,
        &sender_wallet,
        &receiver_wallet,
    )?;

    Ok(HttpResponse::Ok().finish())
}

fn transfer_tickets_on_blockchain(
    tickets: &[TicketInstance],
    connection: &PgConnection,
    tari_client: &TariClient,
    sender_wallet: &Wallet,
    receiver_wallet: &Wallet,
) -> Result<(), BigNeonError> {
    //Assemble token ids and ticket instance ids for each asset in the order
    let mut tokens_per_asset: HashMap<Uuid, Vec<u64>> = HashMap::new();
    for ticket in tickets {
        tokens_per_asset
            .entry(ticket.asset_id)
            .or_insert_with(|| Vec::new())
            .push(ticket.token_id as u64);
    }

    //Transfer each ticket on chain in batches per asset
    for (asset_id, token_ids) in &tokens_per_asset {
        let asset = Asset::find(*asset_id, connection)?;
        match asset.blockchain_asset_id {
            Some(a) => {
                tari_client.transfer_tokens(&sender_wallet.secret_key, &sender_wallet.public_key,
                                                         &a,
                                                         token_ids.clone(),
                                                         receiver_wallet.public_key.clone(),
                )?
            },
            None => return Err(ApplicationError::new(
                "Could not complete ticket transfer because the asset has not been assigned on the blockchain".to_string()
            ).into()),
        }
    }
    Ok(())
}

#[derive(Clone, Deserialize, Serialize)]
pub struct TransferTicketRequest {
    pub ticket_ids: Vec<Uuid>,
    pub validity_period_in_seconds: i64,
}
