use chrono::prelude::*;
use diesel;
use diesel::dsl::*;
use diesel::expression::dsl;
use diesel::prelude::*;
use diesel::sql_types;
use diesel::sql_types::{BigInt, Bool, Integer, Nullable, Text, Timestamp, Uuid as dUuid};
use itertools::Itertools;
use log::Level::Debug;
use models::*;
use rand;
use rand::Rng;
use schema::{
    assets, events, order_items, orders, ticket_instances, ticket_types, users, venues, wallets,
};
use std::cmp;
use tari_client::*;
use time::Duration;
use utils::errors::*;
use uuid::Uuid;

#[derive(Debug, Identifiable, PartialEq, Deserialize, Serialize, Queryable, QueryableByName)]
#[table_name = "ticket_instances"]
pub struct TicketInstance {
    pub id: Uuid,
    pub asset_id: Uuid,
    pub token_id: i32,
    pub hold_id: Option<Uuid>,
    pub order_item_id: Option<Uuid>,
    pub wallet_id: Uuid,
    pub reserved_until: Option<NaiveDateTime>,
    pub redeem_key: Option<String>,
    pub transfer_key: Option<Uuid>,
    pub transfer_expiry_date: Option<NaiveDateTime>,
    pub status: TicketInstanceStatus,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
}

impl TicketInstance {
    pub fn ticket_type(&self, conn: &PgConnection) -> Result<TicketType, DatabaseError> {
        ticket_instances::table
            .inner_join(assets::table.on(ticket_instances::asset_id.eq(assets::id)))
            .inner_join(ticket_types::table.on(assets::ticket_type_id.eq(ticket_types::id)))
            .filter(ticket_instances::id.eq(self.id))
            .select(ticket_types::all_columns)
            .first(conn)
            .to_db_error(
                ErrorCode::QueryError,
                "Unable to load ticket type for ticket instance",
            )
    }

    pub fn find(id: Uuid, conn: &PgConnection) -> Result<TicketInstance, DatabaseError> {
        ticket_instances::table
            .find(id)
            .first(conn)
            .to_db_error(ErrorCode::QueryError, "Unable to load ticket")
    }

    pub fn release(&self, user_id: Uuid, conn: &PgConnection) -> Result<(), DatabaseError> {
        let query = include_str!("../queries/release_tickets.sql");
        let new_status = if self.ticket_type(conn)?.status == TicketTypeStatus::Cancelled {
            TicketInstanceStatus::Nullified
        } else {
            TicketInstanceStatus::Available
        };

        let tickets: Vec<TicketInstance> = diesel::sql_query(query)
            .bind::<Nullable<dUuid>, _>(self.order_item_id)
            .bind::<BigInt, _>(1)
            .bind::<Text, _>(TicketInstanceStatus::Purchased)
            .bind::<dUuid, _>(self.id)
            .bind::<Text, _>(new_status)
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Could not release ticket")?;

        if tickets.len() != 1 {
            return DatabaseError::validation_error("quantity", "Could not release the ticket");
        }

        if new_status == TicketInstanceStatus::Nullified {
            tickets[0].create_nullified_domain_event(user_id, conn)?;
        }

        Ok(())
    }

    pub fn was_transferred(&self, conn: &PgConnection) -> Result<bool, DatabaseError> {
        ticket_instances::table
            .inner_join(wallets::table.on(ticket_instances::wallet_id.eq(wallets::id)))
            .left_join(
                order_items::table
                    .on(ticket_instances::order_item_id.eq(order_items::id.nullable())),
            )
            .left_join(
                orders::table.on(order_items::order_id.eq(orders::id)),
            )
            .filter(ticket_instances::id.eq(self.id))
            .select(
                sql::<Bool>("case when order_items.id is null then true else orders.user_id <> wallets.user_id end")
            )
            .get_result(conn)
            .to_db_error(ErrorCode::QueryError, "Unable to check if ticket instance was transferred")
    }

    pub fn find_for_display(
        id: Uuid,
        conn: &PgConnection,
    ) -> Result<(DisplayEvent, Option<DisplayUser>, DisplayTicket), DatabaseError> {
        let ticket_intermediary = ticket_instances::table
            .inner_join(assets::table.on(ticket_instances::asset_id.eq(assets::id)))
            .inner_join(
                order_items::table
                    .on(ticket_instances::order_item_id.eq(order_items::id.nullable())),
            )
            .inner_join(ticket_types::table.on(assets::ticket_type_id.eq(ticket_types::id)))
            .inner_join(wallets::table.on(ticket_instances::wallet_id.eq(wallets::id)))
            .inner_join(events::table.on(ticket_types::event_id.eq(events::id)))
            .filter(ticket_instances::id.eq(id))
            .select((
                ticket_instances::id,
                order_items::order_id,
                sql::<BigInt>(
                    "cast(unit_price_in_cents +
                    coalesce((
                        select sum(unit_price_in_cents)
                        from order_items
                        where parent_id = ticket_instances.order_item_id),
                    0) as BigInt)
                    ",
                ),
                assets::ticket_type_id,
                ticket_types::name,
                wallets::user_id,
                events::id,
                events::venue_id,
                ticket_instances::status,
                ticket_instances::redeem_key,
                events::redeem_date,
                sql::<Bool>(
                    "CAST(CASE WHEN
                            ticket_instances.transfer_key IS NULL
                            OR ticket_instances.transfer_expiry_date IS NULL
                            OR ticket_instances.transfer_expiry_date < NOW()
                            THEN false
                        ELSE
                           true
                        END
                        AS BOOLEAN)
                             AS pending_transfer",
                ),
            ))
            .first::<DisplayTicketIntermediary>(conn)
            .to_db_error(ErrorCode::QueryError, "Unable to load ticket")?;
        let event = Event::find(ticket_intermediary.event_id, conn)?.for_display(conn)?;
        let user: Option<DisplayUser> = match ticket_intermediary.user_id {
            Some(uid) => Some(User::find(uid, conn)?.into()),
            None => None,
        };
        Ok((event, user, ticket_intermediary.into()))
    }

    pub fn find_for_processing(
        id: Uuid,
        event_id: Uuid,
        conn: &PgConnection,
    ) -> Result<ProcessingTicketIntermediary, DatabaseError> {
        let ticket_intermediary = ticket_instances::table
            .inner_join(assets::table.on(ticket_instances::asset_id.eq(assets::id)))
            .inner_join(ticket_types::table.on(assets::ticket_type_id.eq(ticket_types::id)))
            .inner_join(wallets::table.on(ticket_instances::wallet_id.eq(wallets::id)))
            .inner_join(events::table.on(ticket_types::event_id.eq(events::id)))
            .filter(ticket_instances::id.eq(id))
            .filter(events::id.eq(event_id))
            .select((
                ticket_instances::id,
                ticket_instances::asset_id,
                ticket_instances::token_id,
                ticket_instances::wallet_id,
                ticket_types::name,
                wallets::user_id,
                events::id,
                events::venue_id,
            ))
            .first::<ProcessingTicketIntermediary>(conn)
            .to_db_error(ErrorCode::QueryError, "Unable to load ticket")?;

        //let event = Event::find(ticket_intermediary.event_id, conn)?.for_display(conn)?;
        Ok(ticket_intermediary)
    }

    pub fn find_for_user_for_display(
        user_id: Uuid,
        event_id: Option<Uuid>,
        start_time: Option<NaiveDateTime>,
        end_time: Option<NaiveDateTime>,
        conn: &PgConnection,
    ) -> Result<Vec<(DisplayEvent, Vec<DisplayTicket>)>, DatabaseError> {
        let mut query =
            ticket_instances::table
                .inner_join(assets::table.on(ticket_instances::asset_id.eq(assets::id)))
                .inner_join(
                    order_items::table
                        .on(ticket_instances::order_item_id.eq(order_items::id.nullable())),
                )
                .inner_join(ticket_types::table.on(assets::ticket_type_id.eq(ticket_types::id)))
                .inner_join(wallets::table.on(ticket_instances::wallet_id.eq(wallets::id)))
                .inner_join(events::table.on(ticket_types::event_id.eq(events::id)))
                .filter(events::event_start.ge(
                    start_time.unwrap_or_else(|| NaiveDate::from_ymd(1970, 1, 1).and_hms(0, 0, 0)),
                ))
                .filter(events::event_end.le(
                    end_time.unwrap_or_else(|| NaiveDate::from_ymd(3970, 1, 1).and_hms(0, 0, 0)),
                ))
                .filter(wallets::user_id.eq(user_id))
                .into_boxed();

        if let Some(event_id) = event_id {
            query = query.filter(events::id.eq(event_id));
        }

        let tickets = query
            .select((
                ticket_instances::id,
                order_items::order_id,
                sql::<BigInt>(
                    "cast(unit_price_in_cents +
                    coalesce((
                        select sum(unit_price_in_cents)
                        from order_items
                        where parent_id = ticket_instances.order_item_id),
                    0) as BigInt)
                    ",
                ),
                assets::ticket_type_id,
                ticket_types::name,
                wallets::user_id,
                events::id,
                events::venue_id,
                ticket_instances::status,
                ticket_instances::redeem_key,
                events::redeem_date,
                sql::<Bool>(
                    "CAST(CASE WHEN
                            ticket_instances.transfer_key IS NULL
                            OR ticket_instances.transfer_expiry_date IS NULL
                            OR ticket_instances.transfer_expiry_date < NOW()
                            THEN false
                        ELSE
                           true
                        END AS BOOLEAN)
                             AS pending_transfer",
                ),
            ))
            .order_by(events::event_start.asc())
            .then_order_by(events::name.asc())
            .load::<DisplayTicketIntermediary>(conn)
            .to_db_error(ErrorCode::QueryError, "Unable to load user tickets")?;

        let mut grouped_display_tickets = Vec::new();
        for (key, group) in &tickets.into_iter().group_by(|ticket| ticket.event_id) {
            let event = Event::find(key, conn)?.for_display(conn)?;
            let display_tickets: Vec<DisplayTicket> =
                group.into_iter().map(|ticket| ticket.into()).collect();
            grouped_display_tickets.push((event, display_tickets));
        }

        Ok(grouped_display_tickets)
    }

    pub fn find_for_user(
        user_id: Uuid,
        conn: &PgConnection,
    ) -> Result<Vec<TicketInstance>, DatabaseError> {
        ticket_instances::table
            .inner_join(wallets::table.on(ticket_instances::wallet_id.eq(wallets::id)))
            .filter(wallets::user_id.eq(user_id))
            .select(ticket_instances::all_columns)
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load ticket instances")
    }

    pub fn create_multiple(
        asset_id: Uuid,
        starting_tari_id: u32,
        quantity: u32,
        wallet_id: Uuid,
        conn: &PgConnection,
    ) -> Result<(), DatabaseError> {
        let mut new_rows = Vec::<NewTicketInstance>::new();
        new_rows.reserve(1_000);
        for x in 0..quantity {
            new_rows.push(NewTicketInstance {
                asset_id,
                token_id: (starting_tari_id + x) as i32,
                wallet_id,
            });

            if x % 1_000 == 0 {
                diesel::insert_into(ticket_instances::table)
                    .values(&new_rows)
                    .execute(conn)
                    .to_db_error(ErrorCode::InsertError, "Could not create ticket instances")?;
                new_rows.truncate(0);
            }
        }

        if !new_rows.is_empty() {
            diesel::insert_into(ticket_instances::table)
                .values(&new_rows)
                .execute(conn)
                .to_db_error(ErrorCode::InsertError, "Could not create ticket instances")?;
        }
        Ok(())
    }

    pub fn reserve_tickets(
        order_item: &OrderItem,
        expires_at: Option<NaiveDateTime>,
        ticket_type_id: Uuid,
        ticket_holding_id: Option<Uuid>,
        quantity: u32,
        conn: &PgConnection,
    ) -> Result<Vec<TicketInstance>, DatabaseError> {
        let order_expires_at = expires_at.ok_or(DatabaseError::new(
            ErrorCode::BusinessProcessError,
            Some("Expiration date was not set on cart prior to reserving tickets".to_string()),
        ))?;

        let query = include_str!("../queries/reserve_tickets.sql");
        let q = diesel::sql_query(query)
            .bind::<sql_types::Uuid, _>(order_item.id)
            .bind::<sql_types::Timestamp, _>(order_expires_at)
            .bind::<sql_types::Uuid, _>(ticket_type_id)
            .bind::<sql_types::Nullable<sql_types::Uuid>, _>(ticket_holding_id)
            .bind::<BigInt, _>(quantity as i64);
        let tickets: Vec<TicketInstance> = q
            .get_results(conn)
            .to_db_error(ErrorCode::UpdateError, "Could not reserve tickets")?;

        if tickets.len() as u32 != quantity {
            return DatabaseError::validation_error(
                "quantity",
                "Could not reserve the correct amount of tickets",
            );
        }

        Ok(tickets)
    }

    pub fn release_tickets(
        order_item: &OrderItem,
        quantity: u32,
        conn: &PgConnection,
    ) -> Result<Vec<TicketInstance>, DatabaseError> {
        let query = include_str!("../queries/release_tickets.sql");
        let ticket_instance_id: Option<Uuid> = None;
        let q = diesel::sql_query(query)
            .bind::<sql_types::Uuid, _>(order_item.id)
            .bind::<BigInt, _>(quantity as i64)
            .bind::<Text, _>(TicketInstanceStatus::Reserved)
            .bind::<Nullable<dUuid>, _>(ticket_instance_id)
            .bind::<Text, _>(TicketInstanceStatus::Available);
        let tickets: Vec<TicketInstance> = q
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Could not release tickets")?;

        if tickets.len() as u32 != quantity {
            return DatabaseError::validation_error(
                "quantity",
                "Could not release the correct amount of tickets",
            );
        }
        Ok(tickets)
    }

    pub(crate) fn add_to_hold(
        hold_id: Uuid,
        ticket_type_id: Uuid,
        quantity: u32,
        from_hold_id: Option<Uuid>,
        conn: &PgConnection,
    ) -> Result<Vec<TicketInstance>, DatabaseError> {
        let query = include_str!("../queries/add_tickets_to_hold.sql");
        let q = diesel::sql_query(query)
            .bind::<sql_types::Uuid, _>(hold_id)
            .bind::<sql_types::Uuid, _>(ticket_type_id)
            .bind::<BigInt, _>(quantity as i64)
            .bind::<Nullable<sql_types::Uuid>, _>(from_hold_id);

        let tickets: Vec<TicketInstance> = q
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Could not add tickets to the hold")?;

        if tickets.len() as u32 != quantity {
            return DatabaseError::validation_error(
                "quantity",
                "Could not reserve the correct amount of tickets",
            );
        }

        Ok(tickets)
    }

    pub fn release_from_hold(
        hold_id: Uuid,
        ticket_type_id: Uuid,
        quantity: u32,
        conn: &PgConnection,
    ) -> Result<Vec<TicketInstance>, DatabaseError> {
        let query = include_str!("../queries/release_tickets_from_hold.sql");
        let q = diesel::sql_query(query)
            .bind::<sql_types::Uuid, _>(hold_id)
            .bind::<sql_types::Uuid, _>(ticket_type_id)
            .bind::<BigInt, _>(quantity as i64);

        let tickets: Vec<TicketInstance> = q.get_results(conn).to_db_error(
            ErrorCode::QueryError,
            "Could not release tickets from the hold",
        )?;

        if tickets.len() as u32 != quantity {
            return DatabaseError::validation_error(
                "quantity",
                "Could not release the correct amount of tickets",
            );
        }

        Ok(tickets)
    }

    pub fn count_for_hold(
        hold_id: Uuid,
        ticket_type_id: Uuid,
        conn: &PgConnection,
    ) -> Result<(u32, u32), DatabaseError> {
        #[derive(Queryable)]
        struct R {
            ticket_count: Option<i64>,
            available_count: Option<i64>,
        };

        match ticket_instances::table
            .inner_join(assets::table)
            .filter(ticket_instances::hold_id.eq(hold_id))
            .filter(assets::ticket_type_id.eq(ticket_type_id))
            .select((
                sql::<sql_types::Nullable<sql_types::BigInt>>(
                    "COUNT(DISTINCT ticket_instances.id)",
                ),
                sql::<sql_types::Nullable<sql_types::BigInt>>(
                    "SUM(CASE WHEN ticket_instances.status IN ('Available', 'Reserved') THEN 1 ELSE 0 END)",
                ),
            ))
            .first::<R>(conn)
            .to_db_error(
                ErrorCode::QueryError,
                "Could not retrieve the number of tickets in this hold",
            )
            .optional()?
            {
                Some(r) => Ok((
                    r.ticket_count.unwrap_or(0) as u32,
                    r.available_count.unwrap_or(0) as u32,
                )),
                None => Ok((0, 0)),
            }
    }

    pub fn find_for_order_item(
        order_item_id: Uuid,
        conn: &PgConnection,
    ) -> Result<Vec<TicketInstance>, DatabaseError> {
        ticket_instances::table
            .filter(ticket_instances::order_item_id.eq(order_item_id))
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load Ticket Instances")
    }

    pub fn update_reserved_time(
        order_item: &OrderItem,
        reserved_time: NaiveDateTime,
        conn: &PgConnection,
    ) -> Result<(), DatabaseError> {
        if order_item.item_type != OrderItemTypes::Tickets {
            return Ok(());
        }
        let rows_affected = diesel::update(
            ticket_instances::table.filter(
                ticket_instances::order_item_id
                    .eq(order_item.id)
                    .and(ticket_instances::reserved_until.gt(dsl::now.nullable())),
            ),
        )
        .set((
            ticket_instances::reserved_until.eq(reserved_time),
            ticket_instances::updated_at.eq(dsl::now),
        ))
        .execute(conn)
        .to_db_error(
            ErrorCode::UpdateError,
            "Could not update ticket_instance reserved time.",
        )?;
        if rows_affected == 0 {
            jlog!(Debug, "Could not update reserved ticket time", { "order_item_id": order_item.id, "reserved_time": reserved_time});
            return DatabaseError::concurrency_error("Could not update reserved ticket time");
        };
        Ok(())
    }

    // Note: Transfer mechanism should be used in most cases over this method
    pub fn set_wallet(&self, wallet: &Wallet, conn: &PgConnection) -> Result<(), DatabaseError> {
        diesel::update(ticket_instances::table.filter(ticket_instances::id.eq(self.id)))
            .set((
                ticket_instances::wallet_id.eq(wallet.id()),
                ticket_instances::updated_at.eq(dsl::now),
            ))
            .execute(conn)
            .to_db_error(
                ErrorCode::UpdateError,
                "Could not update ticket_instance wallet.",
            )?;
        Ok(())
    }

    pub fn mark_as_purchased(
        order_item: &OrderItem,
        user_id: Uuid,
        conn: &PgConnection,
    ) -> Result<(), DatabaseError> {
        let wallet = Wallet::find_for_user(user_id, conn)?;

        if wallet.is_empty() {
            return Err(DatabaseError::new(
                ErrorCode::InternalError,
                Some("User does not have a wallet associated with them".to_string()),
            ));
        }

        let tickets = diesel::update(
            ticket_instances::table.filter(ticket_instances::order_item_id.eq(order_item.id)),
        )
        .set((
            ticket_instances::wallet_id.eq(wallet[0].id()),
            ticket_instances::status.eq(TicketInstanceStatus::Purchased),
            ticket_instances::updated_at.eq(dsl::now),
        ))
        .get_results::<TicketInstance>(conn)
        .to_db_error(
            ErrorCode::UpdateError,
            "Could not update ticket_instance status to purchased.",
        )?;

        //Generate redeem codes for the tickets
        for t in &tickets {
            let key = generate_redeem_key(9);

            diesel::update(t)
                .set(ticket_instances::redeem_key.eq(key.clone()))
                .execute(conn)
                .to_db_error(ErrorCode::InternalError, "Could not write redeem key")?;
            DomainEvent::create(
                DomainEventTypes::TicketInstancePurchased,
                "Ticket purchased".to_string(),
                Tables::TicketInstances,
                Some(t.id),
                Some(user_id),
                Some(json!({
                "order_id" : order_item.order_id, "wallet_id": wallet[0].id(), "order_item_id": order_item.id, "redeem_key": key}
            ))).commit(conn)?;
        }
        Ok(())
    }

    pub fn redeem_ticket(
        ticket_id: Uuid,
        redeem_key: String,
        user_id: Uuid,
        conn: &PgConnection,
    ) -> Result<RedeemResults, DatabaseError> {
        let ticket: TicketInstance = ticket_instances::table
            .find(ticket_id)
            .first(conn)
            .to_db_error(ErrorCode::QueryError, "Unable to load ticket")?;

        if ticket.status == TicketInstanceStatus::Purchased
            && ticket.redeem_key.is_some()
            && ticket.redeem_key.unwrap() == redeem_key
        {
            diesel::update(ticket_instances::table.filter(ticket_instances::id.eq(ticket_id)))
                .set(ticket_instances::status.eq(TicketInstanceStatus::Redeemed))
                .execute(conn)
                .to_db_error(ErrorCode::UpdateError, "Could not set ticket to Redeemed")?;

            DomainEvent::create(
                DomainEventTypes::TicketInstanceRedeemed,
                "Ticket redeemed".to_string(),
                Tables::TicketInstances,
                Some(ticket.id),
                Some(user_id),
                None,
            )
            .commit(conn)?;
        } else if ticket.status == TicketInstanceStatus::Redeemed {
            return Ok(RedeemResults::TicketAlreadyRedeemed);
        } else {
            return Ok(RedeemResults::TicketInvalid);
        }
        Ok(RedeemResults::TicketRedeemSuccess)
    }

    pub fn show_redeemable_ticket(
        ticket_id: Uuid,
        conn: &PgConnection,
    ) -> Result<RedeemableTicket, DatabaseError> {
        let mut ticket_data = ticket_instances::table
            .inner_join(
                order_items::table
                    .on(ticket_instances::order_item_id.eq(order_items::id.nullable())),
            )
            .inner_join(orders::table.on(order_items::order_id.eq(orders::id)))
            .inner_join(assets::table.on(ticket_instances::asset_id.eq(assets::id)))
            .inner_join(ticket_types::table.on(assets::ticket_type_id.eq(ticket_types::id)))
            .inner_join(wallets::table.on(ticket_instances::wallet_id.eq(wallets::id)))
            .inner_join(events::table.on(ticket_types::event_id.eq(events::id)))
            .left_join(venues::table.on(events::venue_id.eq(venues::id.nullable())))
            .inner_join(users::table.on(sql(
                "coalesce(orders.on_behalf_of_user_id, wallets.user_id) = users.id",
            )))
            .filter(ticket_instances::id.eq(ticket_id))
            .select((
                ticket_instances::id,
                ticket_types::name,
                sql::<Nullable<dUuid>>("users.id as user_id"),
                order_items::order_id,
                sql::<dUuid>("order_items.id as order_item_id"),
                sql::<BigInt>(
                    "cast(unit_price_in_cents +
                    coalesce((
                        select sum(unit_price_in_cents)
                        from order_items
                        where parent_id = ticket_instances.order_item_id),
                    0) as BigInt) AS price_in_cents
                    ",
                ),
                users::first_name,
                users::last_name,
                users::email,
                users::phone,
                ticket_instances::redeem_key,
                events::redeem_date,
                ticket_instances::status,
                events::id,
                events::name,
                events::door_time,
                events::event_start,
                events::venue_id,
                venues::name.nullable(),
            ))
            .first::<RedeemableTicket>(conn)
            .to_db_error(ErrorCode::QueryError, "Unable to load ticket")?;

        // Ensure that redeem_key is returned either 24 hours before event start or at redeem_date (whichever is earliest)
        let day_before_event_start = ticket_data
            .event_start
            .map(|event_start| event_start - Duration::hours(24));

        let bounded_redeem_date = ticket_data
            .redeem_date
            .map(|redeem_date| {
                if day_before_event_start.is_none() {
                    redeem_date
                } else {
                    // Whichever one is earlier
                    cmp::min(day_before_event_start.unwrap(), redeem_date)
                }
            })
            .or(day_before_event_start)
            .unwrap();

        if bounded_redeem_date > Utc::now().naive_utc() {
            ticket_data.redeem_key = None; //Redeem key not available yet. Should this be an error?
        }

        Ok(ticket_data)
    }

    pub fn authorize_ticket_transfer(
        user_id: Uuid,
        ticket_ids: Vec<Uuid>,
        validity_period_in_seconds: u32,
        conn: &PgConnection,
    ) -> Result<TransferAuthorization, DatabaseError> {
        //Confirm that tickets are purchased and owned by user

        let tickets = TicketInstance::find_for_user(user_id, conn)?;
        let mut ticket_ids_and_updated_at: Vec<(Uuid, NaiveDateTime)> = Vec::new();
        let mut all_tickets_valid = true;
        let mut wallet_id = Uuid::nil();

        for ti in &ticket_ids {
            let mut found_and_purchased = false;
            for t in &tickets {
                if t.id == *ti && t.status == TicketInstanceStatus::Purchased {
                    found_and_purchased = true;
                    ticket_ids_and_updated_at.push((*ti, t.updated_at));
                    wallet_id = t.wallet_id;
                    break;
                }
            }
            if !found_and_purchased {
                all_tickets_valid = false;
                break;
            }
        }

        if !all_tickets_valid || tickets.len() == 0 {
            return Err(DatabaseError::new(
                ErrorCode::BusinessProcessError,
                Some("User does not own all requested tickets".to_string()),
            ));
        }

        //Generate transfer_key and store keys and set transfer_expiry date
        let transfer_key = Uuid::new_v4();
        let transfer_expiry_date =
            Utc::now().naive_utc() + Duration::seconds(validity_period_in_seconds as i64);

        let mut update_count = 0;
        for (t_id, t_updated_at) in &ticket_ids_and_updated_at {
            update_count += diesel::update(
                ticket_instances::table
                    .filter(ticket_instances::id.eq(t_id))
                    .filter(ticket_instances::updated_at.eq(t_updated_at))
                    .filter(ticket_instances::wallet_id.eq(wallet_id)),
            )
            .set((
                ticket_instances::transfer_key.eq(&transfer_key),
                ticket_instances::transfer_expiry_date.eq(&transfer_expiry_date),
                ticket_instances::updated_at.eq(dsl::now),
            ))
            .execute(conn)
            .to_db_error(ErrorCode::UpdateError, "Could not update ticket instance")?;
            DomainEvent::create(
                DomainEventTypes::TransferTicketStarted,
                "Transfer ticket started".to_string(),
                Tables::TicketInstances,
                Some(t_id.clone()),
                Some(user_id.clone()),
                None,
            )
            .commit(conn)?;
        }

        if update_count != ticket_ids.len() {
            return Err(DatabaseError::new(
                ErrorCode::UpdateError,
                Some("Could not update ticket instances".to_string()),
            ));
        }
        //Build Authorization message with signature
        let mut message: String = transfer_key.to_string();
        message.push_str(user_id.to_string().as_str());
        message.push_str((ticket_ids.len() as u32).to_string().as_str());
        let secret_key = Wallet::find_default_for_user(user_id, conn)?.secret_key;
        Ok(TransferAuthorization {
            transfer_key: transfer_key,
            sender_user_id: user_id,
            num_tickets: ticket_ids.len() as u32,
            signature: convert_bytes_to_hexstring(&cryptographic_signature(
                &message,
                &convert_hexstring_to_bytes(&secret_key),
            )?),
        })
    }

    pub fn receive_ticket_transfer(
        transfer_authorization: TransferAuthorization,
        sender_wallet: &Wallet,
        receiver_wallet_id: &Uuid,
        conn: &PgConnection,
    ) -> Result<Vec<TicketInstance>, DatabaseError> {
        //Validate signature
        let mut header: String = transfer_authorization.transfer_key.to_string();
        header.push_str(transfer_authorization.sender_user_id.to_string().as_str());
        header.push_str(transfer_authorization.num_tickets.to_string().as_str());
        if !cryptographic_verify(
            &convert_hexstring_to_bytes(&transfer_authorization.signature),
            &header,
            &convert_hexstring_to_bytes(&sender_wallet.public_key),
        ) {
            return Err(DatabaseError::new(
                ErrorCode::InternalError,
                Some("ECDSA Signature is not valid".to_string()),
            ));
        }
        //Confirm that transfer authorization time has not passed and that the sender still owns the tickets
        //being transfered
        let tickets: Vec<TicketInstance> = ticket_instances::table
            .filter(ticket_instances::transfer_key.eq(transfer_authorization.transfer_key))
            .filter(ticket_instances::transfer_expiry_date.gt(dsl::now.nullable()))
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load ticket instances")?;

        let mut own_all = true;
        let mut ticket_ids_to_transfer: Vec<(Uuid, NaiveDateTime)> = Vec::new();
        for t in &tickets {
            if t.wallet_id != sender_wallet.id {
                own_all = false;
                break;
            }
            ticket_ids_to_transfer.push((t.id, t.updated_at));
        }

        if !own_all || tickets.len() != transfer_authorization.num_tickets as usize {
            return Err(DatabaseError::new(
                ErrorCode::BusinessProcessError,
                Some("These tickets have already been transferred.".to_string()),
            ));
        }
        #[derive(AsChangeset)]
        #[changeset_options(treat_none_as_null = "true")]
        #[table_name = "ticket_instances"]
        struct Update {
            transfer_key: Option<Uuid>,
            transfer_expiry_date: Option<NaiveDateTime>,
        };

        //Perform transfer
        let mut update_count = 0;
        for (t_id, updated_at) in &ticket_ids_to_transfer {
            update_count += diesel::update(
                ticket_instances::table
                    .filter(ticket_instances::id.eq(t_id))
                    .filter(ticket_instances::updated_at.eq(updated_at)),
            )
            .set((
                Update {
                    transfer_key: None,
                    transfer_expiry_date: None,
                },
                ticket_instances::wallet_id.eq(receiver_wallet_id),
                ticket_instances::updated_at.eq(dsl::now),
            ))
            .execute(conn)
            .to_db_error(ErrorCode::UpdateError, "Could not update ticket instance")?;
            DomainEvent::create(
                DomainEventTypes::TransferTicketStarted,
                "Transfer ticket completed".to_string(),
                Tables::TicketInstances,
                Some(t_id.clone()),
                None,
                Some(json!({"receiver_wallet_id": receiver_wallet_id.clone()})),
            )
            .commit(conn)?;
        }

        if update_count != transfer_authorization.num_tickets as usize {
            return Err(DatabaseError::new(
                ErrorCode::UpdateError,
                Some("Could not update ticket instances".to_string()),
            ));
        }

        Ok(tickets)
    }

    fn create_nullified_domain_event(
        &self,
        user_id: Uuid,
        conn: &PgConnection,
    ) -> Result<(), DatabaseError> {
        DomainEvent::create(
            DomainEventTypes::TicketInstanceNullified,
            "Ticket nullified".to_string(),
            Tables::TicketInstances,
            Some(self.id),
            Some(user_id),
            None,
        )
        .commit(conn)?;

        Ok(())
    }

    pub fn nullify_tickets(
        asset_id: Uuid,
        quantity: u32,
        user_id: Uuid,
        conn: &PgConnection,
    ) -> Result<Vec<TicketInstance>, DatabaseError> {
        let query = include_str!("../queries/nullify_tickets.sql");
        let q = diesel::sql_query(query)
            .bind::<sql_types::Uuid, _>(asset_id)
            .bind::<sql_types::BigInt, _>(quantity as i64);
        let updated_ticket_instances: Vec<TicketInstance> = q
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Could not nullify tickets")?;

        for ticket_instance in &updated_ticket_instances {
            ticket_instance.create_nullified_domain_event(user_id, conn)?;
        }
        Ok(updated_ticket_instances)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransferAuthorization {
    pub transfer_key: Uuid,
    pub sender_user_id: Uuid,
    pub num_tickets: u32,
    pub signature: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct DisplayTicket {
    pub id: Uuid,
    pub order_id: Uuid,
    pub price_in_cents: u32,
    pub ticket_type_id: Uuid,
    pub ticket_type_name: String,
    pub status: TicketInstanceStatus,
    pub redeem_key: Option<String>,
    pub pending_transfer: bool,
}

#[derive(Queryable, QueryableByName)]
pub struct DisplayTicketIntermediary {
    #[sql_type = "dUuid"]
    pub id: Uuid,
    #[sql_type = "dUuid"]
    pub order_id: Uuid,
    #[sql_type = "BigInt"]
    pub unit_price_in_cents: i64,
    #[sql_type = "dUuid"]
    pub ticket_type_id: Uuid,
    #[sql_type = "Text"]
    pub name: String,
    #[sql_type = "Nullable<dUuid>"]
    pub user_id: Option<Uuid>,
    #[sql_type = "dUuid"]
    pub event_id: Uuid,
    #[sql_type = "Nullable<dUuid>"]
    pub venue_id: Option<Uuid>,
    #[sql_type = "Text"]
    pub status: TicketInstanceStatus,
    #[sql_type = "Nullable<Text>"]
    pub redeem_key: Option<String>,
    #[sql_type = "Nullable<Timestamp>"]
    pub redeem_date: Option<NaiveDateTime>,
    #[sql_type = "Bool"]
    pub pending_transfer: bool,
}

impl From<DisplayTicketIntermediary> for DisplayTicket {
    fn from(ticket_intermediary: DisplayTicketIntermediary) -> Self {
        let redeem_key = if ticket_intermediary.redeem_date.is_some()
            && ticket_intermediary.redeem_date.unwrap() > Utc::now().naive_utc()
        {
            None //Redeem key not available yet. Should this be an error?
        } else {
            ticket_intermediary.redeem_key.clone()
        };

        DisplayTicket {
            id: ticket_intermediary.id,
            order_id: ticket_intermediary.order_id,
            price_in_cents: ticket_intermediary.unit_price_in_cents as u32,
            ticket_type_id: ticket_intermediary.ticket_type_id,
            ticket_type_name: ticket_intermediary.name.clone(),
            status: ticket_intermediary.status.clone(),
            pending_transfer: ticket_intermediary.pending_transfer,
            redeem_key,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ProcessingTicket {
    pub id: Uuid,
    pub asset_id: Uuid,
    pub token_id: i32,
    pub wallet_id: Uuid,
    pub ticket_type_name: String,
}

#[derive(Queryable, QueryableByName)]
pub struct ProcessingTicketIntermediary {
    #[sql_type = "dUuid"]
    pub id: Uuid,
    #[sql_type = "dUuid"]
    pub asset_id: Uuid,
    #[sql_type = "Integer"]
    pub token_id: i32,
    #[sql_type = "dUuid"]
    pub wallet_id: Uuid,
    #[sql_type = "Text"]
    pub name: String,
    #[sql_type = "Nullable<dUuid>"]
    pub user_id: Option<Uuid>,
    #[sql_type = "dUuid"]
    pub event_id: Uuid,
    #[sql_type = "Nullable<dUuid>"]
    pub venue_id: Option<Uuid>,
}

impl From<ProcessingTicketIntermediary> for ProcessingTicket {
    fn from(ticket_intermediary: ProcessingTicketIntermediary) -> Self {
        ProcessingTicket {
            id: ticket_intermediary.id.clone(),
            asset_id: ticket_intermediary.asset_id.clone(),
            token_id: ticket_intermediary.token_id.clone(),
            wallet_id: ticket_intermediary.wallet_id.clone(),
            ticket_type_name: ticket_intermediary.name.clone(),
        }
    }
}

#[derive(Insertable)]
#[table_name = "ticket_instances"]
struct NewTicketInstance {
    asset_id: Uuid,
    token_id: i32,
    wallet_id: Uuid,
}

#[derive(Debug, PartialEq)]
pub enum RedeemResults {
    TicketRedeemSuccess,
    TicketAlreadyRedeemed,
    TicketInvalid,
}

fn generate_redeem_key(len: u32) -> String {
    let hash_char_list = vec![
        '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J',
        'K', 'M', 'N', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
    ];
    (0..len)
        .map(|_| hash_char_list[rand::thread_rng().gen_range(0, hash_char_list.len())])
        .collect()
}
