use actix_web::{HttpResponse, Path, Query, State};
use auth::user::User;
use bigneon_db::models::User as DbUser;
use bigneon_db::models::*;
use communications::mailers;
use config::Environment;
use db::Connection;
use diesel::Connection as DieselConnection;
use errors::BigNeonError;
use extractors::*;
use helpers::application;
use log::Level::Debug;
use models::PathParameters;
use server::AppState;
use std::cmp;
use std::collections::HashMap;
use uuid::Uuid;

pub fn index(
    (conn, query_parameters, user): (Connection, Query<PagingParameters>, User),
) -> Result<HttpResponse, BigNeonError> {
    //@TODO Implement proper paging on db

    user.requires_scope(Scopes::OrderReadOwn)?;
    let orders = Order::find_for_user_for_display(user.id(), conn.get())?;

    Ok(HttpResponse::Ok().json(&Payload::new(orders, query_parameters.into_inner().into())))
}

pub fn show(
    (conn, path, user): (Connection, Path<PathParameters>, User),
) -> Result<HttpResponse, BigNeonError> {
    let connection = conn.get();
    user.requires_scope(Scopes::OrderReadOwn)?;
    let order = Order::find(path.id, connection)?;

    let mut organization_ids = Vec::new();
    if order.user_id != user.id() || order.status == OrderStatus::Draft {
        for organization in order.organizations(connection)? {
            if user.has_scope_for_organization(Scopes::OrderRead, &organization, connection)? {
                organization_ids.push(organization.id);
            }
        }

        if organization_ids.is_empty() {
            return application::forbidden("You do not have access to this order");
        }
    }

    let organization_id_filter = if order.user_id != user.id() {
        Some(organization_ids)
    } else {
        None
    };
    Ok(HttpResponse::Ok().json(json!(order.for_display(organization_id_filter, connection)?)))
}

#[derive(Deserialize, Serialize)]
pub struct DetailsResponse {
    pub items: Vec<OrderDetailsLineItem>,
    pub order_contains_tickets_for_other_organizations: bool,
}

pub fn details(
    (conn, path, user): (Connection, Path<PathParameters>, User),
) -> Result<HttpResponse, BigNeonError> {
    let connection = conn.get();
    let order = Order::find(path.id, connection)?;

    // Confirm that the authorized user has read order details access to at least one of the order's associated organizations
    let mut organization_ids = Vec::new();
    let mut order_contains_tickets_for_other_organizations = false;
    for organization in order.organizations(connection)? {
        if user.has_scope_for_organization(Scopes::OrderRead, &organization, connection)? {
            organization_ids.push(organization.id);
        } else {
            order_contains_tickets_for_other_organizations = true;
        }
    }
    if organization_ids.is_empty() {
        let mut details_data = HashMap::new();
        details_data.insert("order_id", json!(path.id));
        return application::unauthorized(Some(user), Some(details_data));
    }

    Ok(HttpResponse::Ok().json(json!(DetailsResponse {
        items: order.details(organization_ids, connection)?,
        order_contains_tickets_for_other_organizations
    })))
}

#[derive(Deserialize, Serialize, Clone)]
pub struct RefundAttributes {
    pub items: Vec<RefundItem>,
}

#[derive(Deserialize, Serialize)]
pub struct RefundResponse {
    pub amount_refunded: u32,
    pub refund_breakdown: HashMap<PaymentMethods, u32>,
}

pub fn refund(
    (conn, path, json, user, state): (
        Connection,
        Path<PathParameters>,
        Json<RefundAttributes>,
        User,
        State<AppState>,
    ),
) -> Result<HttpResponse, BigNeonError> {
    let refund_attributes = json.into_inner();
    jlog!(Debug, "Request to refund received", {"order_id": path.id, "request": refund_attributes.clone()});
    let connection = conn.get();
    let items = refund_attributes.items;
    let order = Order::find(path.id, connection)?;

    if order.status != OrderStatus::Paid {
        return application::internal_server_error(
            "Order must have associated payments to refund order items",
        );
    }

    // Find list of organizations related to order item id events for confirming user access
    let order_item_ids: Vec<Uuid> = items
        .iter()
        .map(|refund_item| refund_item.order_item_id)
        .collect();
    let organizations = Organization::find_by_order_item_ids(order_item_ids, connection)?;

    // Check for any organizations where user lacks order refund access
    let mut authorized_to_refund_items = !organizations.is_empty();
    for organization in organizations {
        if !user.has_scope_for_organization(Scopes::OrderRefund, &organization, connection)? {
            authorized_to_refund_items = false;
            break;
        }
    }

    if !authorized_to_refund_items {
        let mut details_data = HashMap::new();
        details_data.insert("order_id", json!(path.id));
        details_data.insert("items", json!(items));
        return application::unauthorized(Some(user), Some(details_data));
    }

    let ticket_instance_ids = items
        .iter()
        .filter(|i| i.ticket_instance_id.is_some())
        .map(|i| i.ticket_instance_id.unwrap())
        .collect::<Vec<Uuid>>();

    // Refund amount is fee inclusive if fee no longer applies to the order
    let refund_due = order.refund(items, user.id(), connection)?;

    // Transfer tickets back to the organization wallets
    let mut tokens_per_asset: HashMap<Uuid, Vec<u64>> = HashMap::new();
    let mut wallet_id_per_asset: HashMap<Uuid, Uuid> = HashMap::new();
    let mut ticket_instances_per_asset: HashMap<Uuid, Vec<TicketInstance>> = HashMap::new();
    let refunded_tickets =
        RefundedTicket::find_by_ticket_instance_ids(ticket_instance_ids, connection)?
            .into_iter()
            .filter(|refund_data| refund_data.ticket_refunded_at.is_some());
    for refunded_ticket in refunded_tickets {
        let ticket = TicketInstance::find(refunded_ticket.ticket_instance_id, connection)?;
        tokens_per_asset
            .entry(ticket.asset_id)
            .or_insert_with(|| Vec::new())
            .push(ticket.token_id as u64);
        wallet_id_per_asset
            .entry(ticket.asset_id)
            .or_insert(ticket.wallet_id);
        ticket_instances_per_asset
            .entry(ticket.asset_id)
            .or_insert_with(|| Vec::new())
            .push(ticket);
    }
    let mut modified_tokens: HashMap<Uuid, Vec<u64>> = HashMap::new();

    let mut refund_breakdown: HashMap<PaymentMethods, u32> = HashMap::new();
    let mut payment_remaining_balance_map: HashMap<Option<String>, i64> = HashMap::new();
    let mut amount_refunded = 0;

    // Begin transaction, if it fails at this point all transferred tickets are returned to wallets
    match connection.transaction::<_, BigNeonError, _>(|| {
        for (asset_id, token_ids) in &tokens_per_asset {
            let organization_id = Organization::find_by_asset_id(*asset_id, connection)?.id;
            let organization_wallet =
                Wallet::find_default_for_organization(organization_id, connection)?;
            let asset = Asset::find(*asset_id, connection)?;
            match asset.blockchain_asset_id {
                Some(a) => {
                    let wallet_id = match wallet_id_per_asset.get(asset_id) {
                        Some(w) => w.clone(),
                        None => return Err(application::internal_server_error::<HttpResponse>(
                            "Could not complete this refund because wallet id not found for asset",
                        ).unwrap_err()),
                    };
                    let user_wallet = Wallet::find(wallet_id, connection)?;
                    state.config.tari_client.transfer_tokens(&user_wallet.secret_key, &user_wallet.public_key,
                                                             &a,
                                                             token_ids.clone(),
                                                             organization_wallet.public_key.clone(),
                    )?;
                    modified_tokens.insert(*asset_id, token_ids.clone());
                    match ticket_instances_per_asset.get(asset_id) {
                        Some(ticket_instances) => {
                            for ticket_instance in ticket_instances {
                                ticket_instance.set_wallet(&organization_wallet, connection)?;
                            }
                        }
                        None => return Err(application::internal_server_error::<HttpResponse>(
                            "No ticket instances exist for transferred tokens",
                        ).unwrap_err()),
                    }
                },
                None => return Err(application::internal_server_error::<HttpResponse>(
                    "Could not complete this refund because the asset is not assigned on the blockchain",
                ).unwrap_err())
            }
        }

        // Perform refunds

        // Negative payments / refunds cancel out remaining payment balance
        for payment in order.payments(connection)? {
            // Ignore payments that were only authorized
            if payment.status == PaymentStatus::Authorized {
                continue;
            }

            *payment_remaining_balance_map
                .entry(payment.external_reference)
                .or_insert(0) += payment.amount;
        }

        for payment in order.payments(connection)? {
            if payment.status != PaymentStatus::Completed {
                continue;
            }

            let remaining_balance = payment_remaining_balance_map
                .get(&payment.external_reference)
                .map(|n| *n)
                .unwrap_or(0);
            if remaining_balance == 0 {
                continue;
            }

            let amount_to_refund = cmp::min(refund_due - amount_refunded, remaining_balance as u32);
            let mut refund_data = None;
            if payment.payment_method == PaymentMethods::CreditCard {
                let client = &state
                    .service_locator
                    .create_payment_processor(&payment.provider)?;

                refund_data = match payment.external_reference {
                    Some(ref external_reference) => Some(
                        client
                            .partial_refund(external_reference, amount_to_refund)?
                            .to_json()?,
                    ),
                    None => {
                        return Err(application::internal_server_error::<HttpResponse>(&format!(
                            "Unable to refund amount owed payment {} lacks external reference",
                            payment.id
                        )).unwrap_err())
                    }
                };
            }
            payment.log_refund(user.id(), amount_to_refund, refund_data, connection)?;
            *refund_breakdown.entry(payment.payment_method).or_insert(0) += amount_to_refund;
            amount_refunded += amount_to_refund;
        }

        if amount_refunded < refund_due {
            return Err(application::internal_server_error::<HttpResponse>(&format!(
                "Unable to refund amount owed {} refunded, {} due",
                amount_refunded, refund_due
            )).unwrap_err());
        }

        Ok(())
    }) {
        Err(error) => {
            for (asset_id, token_ids) in &modified_tokens {
                let organization_id = Organization::find_by_asset_id(*asset_id, connection)?.id;
                let organization_wallet =
                    Wallet::find_default_for_organization(organization_id, connection)?;
                let asset = Asset::find(*asset_id, connection)?;
                match asset.blockchain_asset_id {
                    Some(a) => {
                        let wallet_id = match wallet_id_per_asset.get(asset_id) {
                            Some(w) => w.clone(),
                            None => return application::internal_server_error(
                                "Could not complete this refund because wallet id not found for asset",
                            ),
                        };
                        let user_wallet = Wallet::find(wallet_id, connection)?;
                        state.config.tari_client.transfer_tokens(&organization_wallet.secret_key, &organization_wallet.public_key,
                                                                 &a,
                                                                 token_ids.clone(),
                                                                 user_wallet.public_key.clone(),
                        )?;
                    },
                    None => return application::internal_server_error(
                        "Could not complete this refund because the asset is not assigned on the blockchain",
                    ),
                }
            }

            // Return error
            return Err(error);
        }
        _ => (),
    }

    // Commit changes as payment completed
    if state.config.environment != Environment::Test {
        conn.commit_transaction()?;
        conn.begin_transaction()?;
    }

    // Reload order
    let order = Order::find(order.id, connection)?;
    let display_order = order.for_display(None, connection)?;
    let user = DbUser::find(
        order.on_behalf_of_user_id.unwrap_or(order.user_id),
        connection,
    )?;

    // Communicate refund to user
    if let (Some(first_name), Some(email)) = (user.first_name, user.email) {
        mailers::orders::refund_email(
            &first_name,
            email,
            display_order,
            amount_refunded,
            &state.config,
            connection,
        )?;
    }

    Ok(HttpResponse::Ok().json(json!(RefundResponse {
        amount_refunded,
        refund_breakdown
    })))
}

pub fn update(
    (conn, path, json, user): (
        Connection,
        Path<PathParameters>,
        Json<UpdateOrderAttributes>,
        User,
    ),
) -> Result<HttpResponse, BigNeonError> {
    user.requires_scope(Scopes::OrderReadOwn)?;
    let conn = conn.get();

    let order = Order::find(path.id, conn)?;
    if order.user_id != user.id() {
        return application::forbidden("You do not have access to this order");
    }

    let order = order.update(json.into_inner(), conn)?;

    Ok(HttpResponse::Ok().json(order.for_display(None, conn)?))
}

pub fn tickets(
    (conn, path, user): (Connection, Path<PathParameters>, User),
) -> Result<HttpResponse, BigNeonError> {
    let conn = conn.get();
    let order = Order::find(path.id, conn)?;
    // TODO: Only show the redeem key for orgs that the user has access to redeem
    let orgs: Vec<Uuid> = user
        .user
        .organizations(conn)?
        .iter()
        .map(|o| o.id)
        .collect();
    let mut results = vec![];
    for item in order
        .items(conn)?
        .iter()
        .filter(|t| t.item_type == OrderItemTypes::Tickets)
    {
        if order.user_id != user.id() && order.on_behalf_of_user_id != Some(user.id()) {
            if item.event_id.is_none()
                || !orgs.contains(&Event::find(item.event_id.unwrap(), conn)?.organization_id)
            {
                continue;
            }
        }

        for t in TicketInstance::find_for_order_item(item.id, conn)? {
            results.push(TicketInstance::show_redeemable_ticket(t.id, conn)?);
        }
    }
    Ok(HttpResponse::Ok().json(results))
}
