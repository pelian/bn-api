use chrono::prelude::*;
use diesel;
use diesel::dsl::{exists, select};
use diesel::expression::dsl;
use diesel::expression::sql_literal::sql;
use diesel::pg::types::sql_types::Array;
use diesel::prelude::*;
use diesel::sql_types;
use diesel::sql_types::{BigInt, Bool, Integer, Nullable, Text, Uuid as dUuid};
use itertools::Itertools;
use log::Level::{self, Debug};
use models::*;
use schema::{events, order_items, orders, organization_users, organizations, payments, users};
use serde_json;
use serde_json::Value;
use std::borrow::Cow;
use std::cmp;
use std::collections::HashMap;
use time::Duration;
use utils::dates::*;
use utils::errors::*;
use utils::iterators::intersect_set;
use uuid::Uuid;
use validator::*;
use validators::*;

const CART_EXPIRY_TIME_MINUTES: i64 = 15;
const ORDER_NUMBER_LENGTH: usize = 8;

#[derive(Associations, Debug, Identifiable, PartialEq, Queryable, Serialize)]
#[belongs_to(User)]
pub struct Order {
    pub id: Uuid,
    pub user_id: Uuid,
    pub status: OrderStatus,
    order_type: String,
    pub order_date: NaiveDateTime,
    pub expires_at: Option<NaiveDateTime>,
    pub version: i64,
    pub note: Option<String>,
    pub on_behalf_of_user_id: Option<Uuid>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub paid_at: Option<NaiveDateTime>,
    pub box_office_pricing: bool,
    pub checkout_url: Option<String>,
    pub checkout_url_expires: Option<NaiveDateTime>,
    pub create_user_agent: Option<String>,
    pub purchase_user_agent: Option<String>,
    pub external_payment_type: Option<ExternalPaymentType>,
}

#[derive(Insertable)]
#[table_name = "orders"]
pub struct NewOrder {
    user_id: Uuid,
    status: OrderStatus,
    expires_at: Option<NaiveDateTime>,
    order_type: String,
    create_user_agent: Option<String>,
}

impl NewOrder {
    pub fn commit(&self, conn: &PgConnection) -> Result<Order, DatabaseError> {
        use schema::orders;
        DatabaseError::wrap(
            ErrorCode::InsertError,
            "Could not create new order",
            diesel::insert_into(orders::table)
                .values(self)
                .get_result(conn),
        )
    }
}

#[derive(AsChangeset, Deserialize, Serialize)]
#[table_name = "orders"]
pub struct UpdateOrderAttributes {
    #[serde(default, deserialize_with = "double_option_deserialize_unless_blank")]
    pub note: Option<Option<String>>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct RefundItem {
    pub order_item_id: Uuid,
    pub ticket_instance_id: Option<Uuid>,
}

#[derive(Debug, Deserialize, PartialEq, Queryable, QueryableByName, Serialize)]
pub struct OrderDetailsLineItem {
    #[sql_type = "Nullable<dUuid>"]
    pub ticket_instance_id: Option<Uuid>,
    #[sql_type = "dUuid"]
    pub order_item_id: Uuid,
    #[sql_type = "Text"]
    pub description: String,
    #[sql_type = "BigInt"]
    pub ticket_price_in_cents: i64,
    #[sql_type = "BigInt"]
    pub fees_price_in_cents: i64,
    #[sql_type = "BigInt"]
    pub total_price_in_cents: i64,
    #[sql_type = "Text"]
    pub status: String,
    #[sql_type = "Bool"]
    pub refundable: bool,
}

impl Order {
    pub fn validate_record(&self, conn: &PgConnection) -> Result<(), DatabaseError> {
        let validation_errors = append_validation_error(
            Ok(()),
            "event_id",
            Order::order_contains_items_from_only_one_event(self.id, conn)?,
        );

        Ok(validation_errors?)
    }

    pub fn order_contains_items_from_only_one_event(
        id: Uuid,
        conn: &PgConnection,
    ) -> Result<Result<(), ValidationError>, DatabaseError> {
        let event_count = order_items::table
            .filter(order_items::order_id.eq(id))
            .filter(order_items::event_id.is_not_null())
            .select(sql::<BigInt>("count(distinct event_id) AS event_count"))
            .get_result::<i64>(conn)
            .to_db_error(
                ErrorCode::QueryError,
                "Could not get count of unique events in cart",
            )?;

        if event_count > 1 {
            let mut validation_error = create_validation_error(
                "cart_event_limit_reached",
                "Cart limited to one event for purchasing",
            );
            validation_error.add_param(Cow::from("order_id"), &id);
            return Ok(Err(validation_error.into()));
        }
        Ok(Ok(()))
    }

    pub fn destroy(&self, conn: &PgConnection) -> Result<usize, DatabaseError> {
        let cart_user: Option<User> = users::table
            .filter(users::last_cart_id.eq(self.id))
            .get_result(conn)
            .to_db_error(
                ErrorCode::QueryError,
                "Could not find user attached to this cart",
            )
            .optional()?;

        if let Some(user) = cart_user {
            user.update_last_cart(None, conn)?;
        }

        DatabaseError::wrap(
            ErrorCode::DeleteError,
            "Failed to delete order record",
            diesel::delete(self).execute(conn),
        )
    }

    pub(crate) fn destroy_item(
        &self,
        item_id: Uuid,
        conn: &PgConnection,
    ) -> Result<(), DatabaseError> {
        if self.status != OrderStatus::Draft {
            return DatabaseError::business_process_error(
                "Cannot delete an order item for an order that is not in draft",
            );
        }

        // delete children order items
        diesel::delete(order_items::table.filter(order_items::parent_id.eq(item_id)))
            .execute(conn)
            .map(|_| ())
            .to_db_error(ErrorCode::DeleteError, "Could not delete child order item")?;

        diesel::delete(order_items::table.filter(order_items::id.eq(item_id)))
            .execute(conn)
            .map(|_| ())
            .to_db_error(ErrorCode::DeleteError, "Could not delete order item")
    }

    pub fn main_event_id(&self, conn: &PgConnection) -> Result<Uuid, DatabaseError> {
        for item in self.items(conn)? {
            if let Some(event_id) = item.event_id {
                return Ok(event_id);
            }
        }

        DatabaseError::no_results("Could not find any event for this order")
    }

    pub fn refund(
        &self,
        refund_items: Vec<RefundItem>,
        user_id: Uuid,
        conn: &PgConnection,
    ) -> Result<u32, DatabaseError> {
        let mut total_to_be_refunded: u32 = 0;
        for refund_item in refund_items {
            let mut order_item = OrderItem::find(refund_item.order_item_id, conn)?;

            if order_item.order_id != self.id {
                return DatabaseError::business_process_error(
                    "Order item id does not belong to this order",
                );
            }

            let ticket_instance = match refund_item.ticket_instance_id {
                Some(id) => Some(TicketInstance::find(id, conn)?),
                None => None,
            };

            if order_item.item_type == OrderItemTypes::Tickets
                || order_item.item_type == OrderItemTypes::PerUnitFees
            {
                match ticket_instance {
                    None => {
                        return DatabaseError::business_process_error(
                            "Ticket id required when refunding ticket related order item",
                        );
                    }
                    Some(ref ticket_instance) => {
                        let mut refunded_ticket =
                            RefundedTicket::find_or_create_by_ticket_instance(
                                ticket_instance,
                                conn,
                            )?;

                        if refunded_ticket.ticket_refunded_at.is_some()
                            || (refunded_ticket.fee_refunded_at.is_some()
                                && order_item.item_type == OrderItemTypes::PerUnitFees)
                        {
                            return DatabaseError::business_process_error("Already refunded");
                        } else if ticket_instance.was_transferred(conn)? {
                            return DatabaseError::business_process_error(
                                "Ticket was transferred so ineligible for refund",
                            );
                        }

                        let only_refund_fees = order_item.item_type == OrderItemTypes::PerUnitFees;
                        let refund_fees = refunded_ticket.fee_refunded_at.is_none();
                        refunded_ticket.mark_refunded(only_refund_fees, conn)?;

                        // Do not release redeemed tickets back into inventory
                        if ticket_instance.status == TicketInstanceStatus::Purchased {
                            ticket_instance.release(
                                TicketInstanceStatus::Purchased,
                                user_id,
                                conn,
                            )?;
                        }

                        total_to_be_refunded += order_item.refund_one_unit(refund_fees, conn)?;
                    }
                }
            } else {
                total_to_be_refunded += order_item.refund_one_unit(true, conn)?;
            }
        }

        for mut event_fee_item in self.event_fee_items_with_no_associated_items(conn)? {
            total_to_be_refunded += event_fee_item.refund_one_unit(true, conn)?;
        }

        Ok(total_to_be_refunded)
    }

    fn event_fee_items_with_no_associated_items(
        &self,
        conn: &PgConnection,
    ) -> Result<Vec<OrderItem>, DatabaseError> {
        order_items::table
            .filter(order_items::refunded_quantity.ne(order_items::quantity))
            .filter(order_items::order_id.eq(self.id))
            .filter(order_items::item_type.eq(OrderItemTypes::EventFees))
            .filter(sql("not exists(
                select id from order_items oi2
                where oi2.order_id = order_items.order_id
                and oi2.event_id = order_items.event_id
                and item_type <> 'EventFees'
                and oi2.refunded_quantity <> oi2.quantity
            )"))
            .select(order_items::all_columns)
            .load(conn)
            .to_db_error(
                ErrorCode::QueryError,
                "Could not check if order only contains event fees",
            )
    }

    pub fn details(
        &self,
        organization_ids: &Vec<Uuid>,
        user_id: Uuid,
        conn: &PgConnection,
    ) -> Result<Vec<OrderDetailsLineItem>, DatabaseError> {
        let query = include_str!("../queries/order_details.sql");
        diesel::sql_query(query)
            .bind::<dUuid, _>(self.id)
            .bind::<Array<dUuid>, _>(organization_ids)
            .bind::<dUuid, _>(user_id)
            .load(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load order items")
    }

    pub fn partially_visible_order(
        &self,
        organization_ids: &Vec<Uuid>,
        user_id: Uuid,
        conn: &PgConnection,
    ) -> Result<bool, DatabaseError> {
        select(exists(
            order_items::table
                .inner_join(events::table.on(order_items::event_id.eq(events::id.nullable())))
                .left_join(
                    organization_users::table.on(events::organization_id
                        .eq(organization_users::organization_id)
                        .and(organization_users::user_id.eq(user_id))),
                )
                .filter(events::organization_id.ne_all(organization_ids).or(sql("(
                        NOT events.id = ANY(organization_users.event_ids)
                        AND (
                            'Promoter' = ANY(organization_users.role)
                            OR 'PromoterReadOnly' = ANY(organization_users.role)
                        )
                    )")))
                .filter(order_items::order_id.eq(self.id)),
        ))
        .get_result(conn)
        .to_db_error(
            ErrorCode::QueryError,
            "Could not check if order items exist",
        )
    }

    pub fn organizations(&self, conn: &PgConnection) -> Result<Vec<Organization>, DatabaseError> {
        organizations::table
            .inner_join(events::table.on(events::organization_id.eq(organizations::id)))
            .inner_join(order_items::table.on(order_items::event_id.eq(events::id.nullable())))
            .filter(order_items::order_id.eq(self.id))
            .select(organizations::all_columns)
            .order_by(organizations::name.asc())
            .distinct()
            .load(conn)
            .to_db_error(ErrorCode::QueryError, "Error loading organizations")
    }

    pub fn payments(&self, conn: &PgConnection) -> Result<Vec<Payment>, DatabaseError> {
        payments::table
            .filter(payments::order_id.eq(self.id))
            .load(conn)
            .to_db_error(ErrorCode::QueryError, "Error loading payments")
    }

    pub fn set_user_agent(
        &mut self,
        user_agent: Option<String>,
        purchase_completed: bool,
        conn: &PgConnection,
    ) -> Result<(), DatabaseError> {
        self.lock_version(conn)?;
        self.updated_at = Utc::now().naive_utc();
        if purchase_completed {
            self.purchase_user_agent = user_agent;
        } else {
            self.create_user_agent = user_agent;
        }

        let affected_rows = diesel::update(
            orders::table.filter(orders::id.eq(self.id).and(orders::version.eq(self.version))),
        )
        .set((
            orders::purchase_user_agent.eq(self.purchase_user_agent.clone()),
            orders::create_user_agent.eq(self.create_user_agent.clone()),
            orders::updated_at.eq(self.updated_at),
        ))
        .execute(conn)
        .to_db_error(ErrorCode::UpdateError, "Could not update user agent")?;
        if affected_rows != 1 {
            return DatabaseError::concurrency_error("Could not update user agent.");
        }

        Ok(())
    }

    pub fn find_or_create_cart(user: &User, conn: &PgConnection) -> Result<Order, DatabaseError> {
        // Do a quick check to find the cart linked to the user.
        let cart = Order::find_cart_for_user(user.id, conn)?;

        if let Some(cart) = cart {
            return Ok(cart);
        }

        // Cart either does not exist, expired or was paid up.
        // A number of threads might reach here at the same time, so we
        // need to do a bit of concurrency checking.

        let query = r#"
            INSERT INTO Orders (user_id, status, expires_at, order_type)
            SELECT $1 as user_id, 'Draft' as status, null as expires_at, 'Cart' as order_type
            WHERE NOT EXISTS
            ( SELECT o.id FROM orders o
                WHERE o.user_id = $1
                AND o.status = 'Draft'
                AND o.order_type = 'Cart'
                AND o.expires_at > now())
            RETURNING id;
        "#;

        #[derive(QueryableByName)]
        struct R {
            #[sql_type = "Nullable<dUuid>"]
            id: Option<Uuid>,
        }

        let cart_id: Vec<R> = diesel::sql_query(query)
            .bind::<sql_types::Uuid, _>(user.id)
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Could not find or create cart")?;

        if cart_id.is_empty() || cart_id[0].id.is_none() || cart_id.len() > 1 {
            // Another thread has created a cart
            return DatabaseError::concurrency_error(&format!(
                "Possible race condition when creating a cart for a user. Number of carts returned: {}",
                cart_id.len()
            ));
        }

        let cart_id = cart_id[0].id;

        // This will also row lock the user row to detect that another thread has not
        // created another cart in the mean time
        user.update_last_cart(cart_id, conn)?;

        let order = Order::find(cart_id.unwrap(), conn)?;

        DomainEvent::create(
            DomainEventTypes::OrderCreated,
            "Order created".into(),
            Tables::Orders,
            Some(order.id),
            Some(user.id),
            Some(json!(order)),
        )
        .commit(conn)?;

        Ok(order)
    }

    pub fn find_cart_for_user(
        user_id: Uuid,
        conn: &PgConnection,
    ) -> Result<Option<Order>, DatabaseError> {
        users::table
            .inner_join(orders::table.on(users::last_cart_id.eq(orders::id.nullable())))
            .filter(users::id.eq(user_id))
            .filter(orders::user_id.eq(user_id))
            .filter(orders::status.eq("Draft"))
            .filter(orders::order_type.eq("Cart"))
            .filter(
                orders::expires_at
                    .is_null()
                    .or(orders::expires_at.ge(dsl::now.nullable())),
            )
            .select(orders::all_columns)
            .first(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load cart for user")
            .optional()
    }

    pub fn find(id: Uuid, conn: &PgConnection) -> Result<Order, DatabaseError> {
        orders::table
            .filter(orders::id.eq(id))
            .first(conn)
            .to_db_error(ErrorCode::QueryError, "Could not find order")
    }

    /// Sets the expiry time of an order. All tickets in the current order are also updated
    /// to reflect the new expiry
    pub fn set_expiry(
        &mut self,
        current_user_id: Option<Uuid>,
        expires_at: Option<NaiveDateTime>,
        conn: &PgConnection,
    ) -> Result<(), DatabaseError> {
        let old_expiry = self.expires_at;
        let expires_at = if expires_at.is_some() {
            expires_at.unwrap()
        } else {
            Utc::now().naive_utc() + Duration::minutes(CART_EXPIRY_TIME_MINUTES)
        };
        self.expires_at = Some(expires_at);
        self.updated_at = Utc::now().naive_utc();

        let affected_rows = diesel::update(
            orders::table.filter(
                orders::id
                    .eq(self.id)
                    .and(orders::version.eq(self.version))
                    .and(sql("COALESCE(expires_at, '31 Dec 9999') > now()")),
            ),
        )
        .set((
            orders::expires_at.eq(self.expires_at),
            orders::updated_at.eq(self.updated_at),
        ))
        .execute(conn)
        .to_db_error(ErrorCode::UpdateError, "Could not update expiry time")?;
        if affected_rows != 1 {
            return DatabaseError::concurrency_error("Could not update expiry time.");
        }

        DomainEvent::create(
            DomainEventTypes::OrderUpdated,
            format!(
                "Order expiry time updated from {:?} to {:?}",
                &old_expiry.map(|e| e.to_string()).unwrap_or("null".into()),
                &expires_at.to_string()
            ),
            Tables::Orders,
            Some(self.id),
            current_user_id,
            Some(json!({
                "old_expires_at": &old_expiry,
                "new_expires_at": &expires_at
            })),
        )
        .commit(conn)?;

        // Extend the tickets expiry
        let order_items = OrderItem::find_for_order(self.id, conn)?;

        for item in &order_items {
            TicketInstance::update_reserved_time(item, expires_at, conn)?;
        }

        Ok(())
    }

    /// Removes the expiry time for an order. This can only be done when there are no
    /// tickets in the order, otherwise the tickets will remain reserved until the expiry
    pub fn remove_expiry(
        &mut self,
        current_user_id: Uuid,
        conn: &PgConnection,
    ) -> Result<(), DatabaseError> {
        if self.items(conn)?.len() > 0 {
            return DatabaseError::business_process_error(
                "Cannot clear the expiry of an order when there are items in it",
            );
        }
        let old_expiry = self.expires_at;
        self.updated_at = Utc::now().naive_utc();
        self.expires_at = None;
        let affected_rows = diesel::update(
            orders::table.filter(
                orders::id
                    .eq(self.id)
                    .and(orders::version.eq(self.version))
                    .and(
                        orders::expires_at
                            .is_null()
                            .or(orders::expires_at.gt(Some(Utc::now().naive_utc()))),
                    ),
            ),
        )
        .set((
            orders::expires_at.eq(self.expires_at),
            orders::updated_at.eq(self.updated_at),
        ))
        .execute(conn)
        .to_db_error(ErrorCode::UpdateError, "Could not update expiry time")?;
        if affected_rows != 1 {
            return DatabaseError::concurrency_error("Could not update expiry time.");
        }
        DomainEvent::create(
            DomainEventTypes::OrderUpdated,
            format!(
                "Order expiry time removed was {:?}",
                &old_expiry.map(|e| e.to_string()).unwrap_or("null".into())
            ),
            Tables::Orders,
            Some(self.id),
            Some(current_user_id),
            Some(json!({
                "old_expires_at": &old_expiry,
                "new_expires_at": self.expires_at
            })),
        )
        .commit(conn)?;

        Ok(())
    }

    pub fn order_number(&self) -> String {
        Order::parse_order_number(self.id)
    }

    pub fn parse_order_number(id: Uuid) -> String {
        let id_string = id.to_string();
        id_string[id_string.len() - ORDER_NUMBER_LENGTH..].to_string()
    }

    pub fn update(
        self,
        attrs: UpdateOrderAttributes,
        current_user_id: Uuid,
        conn: &PgConnection,
    ) -> Result<Order, DatabaseError> {
        DomainEvent::create(
            DomainEventTypes::OrderUpdated,
            "Order updated".into(),
            Tables::Orders,
            Some(self.id),
            Some(current_user_id),
            Some(json!(attrs)),
        )
        .commit(conn)?;

        diesel::update(&self)
            .set((attrs, orders::updated_at.eq(dsl::now)))
            .execute(conn)
            .to_db_error(ErrorCode::UpdateError, "Could not update order")?;
        Order::find(self.id, conn)
    }

    pub fn clear_cart(&mut self, user_id: Uuid, conn: &PgConnection) -> Result<(), DatabaseError> {
        jlog!(Level::Debug, "Clearing cart");
        self.lock_version(conn)?;

        for mut current_line in self.items(conn)? {
            if current_line.item_type != OrderItemTypes::Tickets {
                continue;
            }
            // Use calculated quantity as reserved may have been taken in the meantime no longer pointing to this order item
            let quantity = current_line.calculate_quantity(conn)?;
            TicketInstance::release_tickets(&current_line, quantity as u32, user_id, conn)?;
            self.destroy_item(current_line.id, conn)?;
        }
        Ok(())
    }

    pub fn update_quantities(
        &mut self,
        current_user_id: Uuid,
        items: &[UpdateOrderItem],
        box_office_pricing: bool,
        remove_others: bool,
        conn: &PgConnection,
    ) -> Result<(), DatabaseError> {
        self.lock_version(conn)?;

        jlog!(Debug, "Update order quantities", {"items": items,"remove_others":remove_others, "user_id": current_user_id, "box_office_pricing":box_office_pricing });

        if box_office_pricing != self.box_office_pricing {
            self.clear_cart(current_user_id, conn)?;
            self.update_box_office_pricing(box_office_pricing, current_user_id, conn)?;
        }

        let current_items = self.items(conn)?;

        #[derive(Debug)]
        struct LimitCheck {
            ticket_type_id: Uuid,
            hold_id: Option<Uuid>,
            limit_per_person: u32,
        }

        struct MatchData<'a> {
            index: Option<usize>,
            hold_id: Option<Uuid>,
            hold: Option<Hold>,
            code_id: Option<Uuid>,
            code: Option<Code>,
            update_order_item: &'a UpdateOrderItem,
        }

        let mut check_ticket_limits: Vec<LimitCheck> = vec![];

        let mut mapped = vec![];
        for (index, item) in items.iter().enumerate() {
            mapped.push(match &item.redemption_code {
                Some(r) => match Hold::find_by_redemption_code(r, conn).optional()? {
                    Some(hold) => {
                        hold.confirm_hold_valid()?;
                        MatchData {
                            index: Some(index),
                            hold_id: Some(hold.id),
                            hold: Some(hold),
                            code_id: None,
                            code: None,
                            update_order_item: item,
                        }
                    }
                    None => match Code::find_by_redemption_code_with_availability(r, None, conn)
                        .optional()?
                    {
                        Some(code_availability) => {
                            code_availability.code.confirm_code_valid()?;
                            MatchData {
                                index: Some(index),
                                hold_id: None,
                                hold: None,
                                code_id: Some(code_availability.code.id),
                                code: Some(code_availability.code),
                                update_order_item: item,
                            }
                        }
                        None => {
                            return DatabaseError::validation_error(
                                "redemption_code",
                                "Redemption code is not valid",
                            );
                        }
                    },
                },
                None => MatchData {
                    index: Some(index),
                    hold_id: None,
                    hold: None,
                    code_id: None,
                    code: None,
                    update_order_item: item,
                },
            });
        }

        for mut current_line in current_items {
            if current_line.item_type != OrderItemTypes::Tickets {
                continue;
            }

            let mut index_to_remove: Option<usize> = None;
            {
                let matching_result: Option<&MatchData> = mapped.iter().find(|match_data| {
                    match_data.index.is_some()
                        && Some(match_data.update_order_item.ticket_type_id)
                            == current_line.ticket_type_id
                        && match_data.hold_id == current_line.hold_id
                        && match_data.code_id == current_line.code_id
                });

                if let Some(match_data) = matching_result {
                    jlog!(Level::Debug, "Found an existing cart item, replacing");
                    index_to_remove = match_data.index;
                    if current_line.quantity as u32 > match_data.update_order_item.quantity {
                        jlog!(Level::Debug, "Reducing quantity of cart item");
                        TicketInstance::release_tickets(
                            &current_line,
                            current_line.quantity as u32 - match_data.update_order_item.quantity,
                            current_user_id,
                            conn,
                        )?;
                        current_line.quantity = match_data.update_order_item.quantity as i64;
                        current_line.update(conn)?;
                        if current_line.quantity == 0 {
                            jlog!(Level::Debug, "Cart item has 0 quantity, deleting it");
                            self.destroy_item(current_line.id, conn)?;
                        }
                    } else if (current_line.quantity as u32) < match_data.update_order_item.quantity
                    {
                        jlog!(Level::Debug, "Increasing quantity of cart item");
                        // Ticket pricing might have changed since we added the previous item.
                        // In future we may want to use the ticket pricing at the time the order was created.

                        // TODO: Fetch the ticket type and pricing in one go.
                        let ticket_type_id = current_line.ticket_type_id.unwrap();
                        let ticket_pricing = TicketPricing::get_current_ticket_pricing(
                            ticket_type_id,
                            box_office_pricing,
                            false,
                            conn,
                        )?;
                        let ticket_type = TicketType::find(ticket_type_id, conn)?;

                        let limit_per_person = match match_data.hold {
                            Some(ref hold) => hold.max_per_user.unwrap_or(0) as u32,
                            None => ticket_type.limit_per_person as u32,
                        };
                        check_ticket_limits.push(LimitCheck {
                            ticket_type_id: ticket_type.id,
                            hold_id: match_data.hold_id,
                            limit_per_person,
                        });

                        // TODO: Move this to an external processer
                        if Some(ticket_pricing.id) != current_line.ticket_pricing_id {
                            let mut price_in_cents = ticket_pricing.price_in_cents;
                            if let Some(h) = match_data.hold.as_ref() {
                                let discount = h.discount_in_cents;
                                let hold_type = h.hold_type;
                                price_in_cents = match hold_type {
                                    HoldTypes::Discount => {
                                        cmp::max(0, price_in_cents - discount.unwrap_or(0))
                                    }
                                    HoldTypes::Comp => 0,
                                };
                            } else if let Some(c) = match_data.code.as_ref() {
                                if c.code_type == CodeTypes::Access {
                                    price_in_cents = cmp::max(
                                        0,
                                        price_in_cents - c.discount_in_cents.unwrap_or(0),
                                    );
                                }
                            }

                            let order_item = NewTicketsOrderItem {
                                order_id: self.id,
                                item_type: OrderItemTypes::Tickets,
                                quantity: match_data.update_order_item.quantity as i64,
                                ticket_type_id: ticket_type.id,
                                ticket_pricing_id: ticket_pricing.id,
                                event_id: Some(ticket_type.event_id),
                                unit_price_in_cents: price_in_cents,
                                hold_id: match_data.hold_id,
                                code_id: match_data.code_id,
                            }
                            .commit(conn)?;
                            TicketInstance::reserve_tickets(
                                &order_item,
                                self.expires_at,
                                ticket_type_id,
                                match_data.hold_id,
                                match_data.update_order_item.quantity
                                    - current_line.quantity as u32,
                                conn,
                            )?;
                        } else {
                            TicketInstance::reserve_tickets(
                                &current_line,
                                self.expires_at,
                                ticket_type_id,
                                match_data.hold_id,
                                match_data.update_order_item.quantity
                                    - current_line.quantity as u32,
                                conn,
                            )?;
                            current_line.quantity = match_data.update_order_item.quantity as i64;
                            current_line.update(conn)?;
                        }
                    }
                } else if remove_others {
                    jlog!(Level::Debug, "Removing extra tickets because remove others was called.", { "order_item.id": current_line.id, "ticket_type_id": current_line.ticket_type_id});
                    jlog!(Level::Debug, "Reducing quantity of cart item");
                    TicketInstance::release_tickets(
                        &current_line,
                        current_line.quantity as u32,
                        current_user_id,
                        conn,
                    )?;
                    self.destroy_item(current_line.id, conn)?;
                }
            }
            if let Some(index) = index_to_remove {
                mapped[index].index = None;
            }
        }

        // Set cart expiration time if not currently set (empty carts have no expiration)
        if self.expires_at.is_none() {
            self.set_expiry(Some(current_user_id), None, conn)?;
        }

        for match_data in mapped {
            if match_data.update_order_item.quantity == 0 || match_data.index.is_none() {
                continue;
            }

            jlog!(Level::Debug, "Adding new cart items");
            let ticket_pricing = TicketPricing::get_current_ticket_pricing(
                match_data.update_order_item.ticket_type_id,
                box_office_pricing,
                false,
                conn,
            )?;
            let ticket_type = TicketType::find(match_data.update_order_item.ticket_type_id, conn)?;

            let limit_per_person = match match_data.hold {
                Some(ref hold) => hold.max_per_user.unwrap_or(0) as u32,
                None => ticket_type.limit_per_person as u32,
            };
            check_ticket_limits.push(LimitCheck {
                ticket_type_id: ticket_type.id,
                hold_id: match_data.hold_id,
                limit_per_person,
            });

            let mut price_in_cents = ticket_pricing.price_in_cents;
            if let Some(h) = match_data.hold.as_ref() {
                let discount = h.discount_in_cents;
                let hold_type = h.hold_type;
                price_in_cents = match hold_type {
                    HoldTypes::Discount => cmp::max(0, price_in_cents - discount.unwrap_or(0)),
                    HoldTypes::Comp => 0,
                }
            } else if let Some(c) = match_data.code.as_ref() {
                if c.code_type == CodeTypes::Access {
                    price_in_cents = cmp::max(0, price_in_cents - c.discount_in_cents.unwrap_or(0));
                }
            }

            // TODO: Move this to an external processer
            let order_item = NewTicketsOrderItem {
                order_id: self.id,
                item_type: OrderItemTypes::Tickets,
                quantity: match_data.update_order_item.quantity as i64,
                ticket_type_id: ticket_type.id,
                ticket_pricing_id: ticket_pricing.id,
                event_id: Some(ticket_type.event_id),
                unit_price_in_cents: price_in_cents,
                hold_id: match_data.hold_id,
                code_id: match_data.code_id,
            }
            .commit(conn)?;

            TicketInstance::reserve_tickets(
                &order_item,
                self.expires_at,
                match_data.update_order_item.ticket_type_id,
                match_data.hold_id,
                match_data.update_order_item.quantity,
                conn,
            )?;
        }

        // if the cart is empty at this point, it is effectively a new cart, remove expiration
        if self.items(conn)?.len() == 0 {
            self.remove_expiry(current_user_id, conn)?;
        }
        for limit_check in check_ticket_limits {
            let ordered_quantity = Order::quantity_for_user_for_ticket_type_by_hold(
                self.user_id,
                limit_check.ticket_type_id,
                limit_check.hold_id,
                &conn,
            )?;

            if limit_check.limit_per_person > 0
                && ordered_quantity > limit_check.limit_per_person.into()
            {
                let mut error = create_validation_error(
                    "limit_per_person_exceeded",
                    if limit_check.hold_id.is_some() {
                        "Exceeded limit per person per hold"
                    } else {
                        "Exceeded limit per person per event"
                    },
                );
                error.add_param(Cow::from("limit_per_person"), &limit_check.limit_per_person);
                error.add_param(Cow::from("ticket_type_id"), &limit_check.ticket_type_id);
                if let Some(hold_id) = limit_check.hold_id {
                    error.add_param(Cow::from("hold_id"), &hold_id);
                }
                error.add_param(Cow::from("attempted_quantity"), &ordered_quantity);
                let mut errors = ValidationErrors::new();
                errors.add("quantity", error);
                return Err(errors.into());
            }
        }
        self.update_fees(conn)?;
        self.validate_record(conn)?;

        Ok(())
    }

    pub fn has_items(&self, conn: &PgConnection) -> Result<bool, DatabaseError> {
        select(exists(
            order_items::table.filter(order_items::order_id.eq(self.id)),
        ))
        .get_result(conn)
        .to_db_error(
            ErrorCode::QueryError,
            "Could not check if order items exist",
        )
    }

    pub fn update_fees(&self, conn: &PgConnection) -> Result<(), DatabaseError> {
        let items = self.items(conn)?;

        for o in items {
            o.update_discount(&self, conn)?;
            match o.item_type {
                OrderItemTypes::EventFees => self.destroy_item(o.id, conn)?,
                _ => {}
            }
        }

        // Box office purchased tickets do not have fees at this time
        if self.box_office_pricing {
            return Ok(());
        }

        let mut per_event_fees_included: HashMap<Uuid, bool> = HashMap::new();

        for ((event_id, hold_id), items) in self
            .items(conn)?
            .iter()
            .filter(|i| i.event_id.is_some())
            .group_by(|i| (i.event_id, i.hold_id))
            .into_iter()
        {
            if event_id.is_none() {
                continue;
            }
            if let Some(hold_id) = hold_id {
                let hold = Hold::find(hold_id, conn)?;
                if hold.hold_type == HoldTypes::Comp {
                    continue;
                }
            }

            let event_id = event_id.unwrap();
            let event = Event::find(event_id, conn)?;

            let mut all_zero_price = true;

            for o in items {
                match o.item_type {
                    OrderItemTypes::Tickets => {
                        let discount_item = o.find_discount_item(conn)?;

                        let unit_price_with_discount = match discount_item {
                            Some(di) => o.unit_price_in_cents + di.unit_price_in_cents,
                            None => o.unit_price_in_cents,
                        };

                        o.update_fees(&self, conn)?;
                        if unit_price_with_discount > 0 {
                            all_zero_price = false;
                        }
                    }
                    _ => {}
                }
            }

            //This must only be run once for an entire order
            //The issue was that if there was a hold that was not a comp as well as normal tickets
            //in the cart the EventFees would get duplicated
            if !all_zero_price && !per_event_fees_included.contains_key(&event_id) {
                let mut new_event_fee = NewFeesOrderItem {
                    order_id: self.id,
                    item_type: OrderItemTypes::EventFees,
                    event_id: Some(event.id),
                    unit_price_in_cents: 0,
                    fee_schedule_range_id: None,
                    company_fee_in_cents: 0,
                    client_fee_in_cents: 0,
                    quantity: 1,
                    parent_id: None,
                };
                if event.fee_in_cents > 0 {
                    //we dont want to create 0 fee order item
                    new_event_fee.company_fee_in_cents = event.company_fee_in_cents;
                    new_event_fee.client_fee_in_cents = event.client_fee_in_cents;
                    new_event_fee.unit_price_in_cents =
                        event.client_fee_in_cents + event.company_fee_in_cents;
                    new_event_fee.commit(conn)?;
                    per_event_fees_included.insert(event_id, true);
                }
            }
        }

        Ok(())
    }

    fn quantity_for_user_for_ticket_type_by_hold(
        user_id: Uuid,
        ticket_type_id: Uuid,
        hold_id: Option<Uuid>,
        conn: &PgConnection,
    ) -> Result<i64, DatabaseError> {
        use schema::*;

        let mut query = orders::table
            .inner_join(order_items::table.on(order_items::order_id.eq(orders::id)))
            .inner_join(
                ticket_instances::table
                    .on(ticket_instances::order_item_id.eq(order_items::id.nullable())),
            )
            .filter(
                orders::user_id
                    .eq(user_id)
                    .and(orders::on_behalf_of_user_id.is_null())
                    .or(orders::on_behalf_of_user_id.eq(user_id)),
            )
            .filter(order_items::ticket_type_id.eq(ticket_type_id))
            .filter(
                ticket_instances::status
                    .eq(TicketInstanceStatus::Purchased)
                    .or(ticket_instances::status
                        .eq(TicketInstanceStatus::Reserved)
                        .and(ticket_instances::reserved_until.gt(Utc::now().naive_utc()))),
            )
            .into_boxed();

        match hold_id {
            Some(hold_id) => {
                query = query.filter(order_items::hold_id.nullable().eq(hold_id));
            }
            None => {
                query = query.filter(order_items::hold_id.is_null());
            }
        }

        query
            .select(dsl::count(ticket_instances::id))
            .get_result(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load total")
    }

    pub fn quantity_for_user_for_event(
        user_id: Uuid,
        event_id: Uuid,
        conn: &PgConnection,
    ) -> Result<HashMap<Uuid, i32>, DatabaseError> {
        let mut ticket_type_totals: HashMap<Uuid, i32> = HashMap::new();

        let query = include_str!("../queries/quantity_of_tickets_per_user_per_event.sql");
        let order_items_for_user: Vec<ResultForTicketTypeTotal> = diesel::sql_query(query)
            .bind::<diesel::sql_types::Uuid, _>(user_id)
            .bind::<diesel::sql_types::Uuid, _>(event_id)
            .load::<ResultForTicketTypeTotal>(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load orders")?;

        for result_for_ticket in &order_items_for_user {
            ticket_type_totals.insert(
                result_for_ticket.ticket_type_id.unwrap(),
                result_for_ticket.total_quantity,
            );
        }

        Ok(ticket_type_totals)
    }

    pub fn find_for_user_for_display(
        user_id: Uuid,
        conn: &PgConnection,
    ) -> Result<Vec<DisplayOrder>, DatabaseError> {
        use schema::*;
        let orders: Vec<Order> = orders::table
            .filter(
                orders::user_id
                    .eq(user_id)
                    .or(orders::on_behalf_of_user_id.eq(user_id)),
            )
            .filter(orders::status.ne(OrderStatus::Draft))
            .order_by(orders::order_date.desc())
            .load(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load orders")?;
        let mut r = Vec::<DisplayOrder>::new();
        for order in orders {
            r.push(order.for_display(None, user_id, conn)?);
        }
        Ok(r)
    }

    pub fn items(&self, conn: &PgConnection) -> Result<Vec<OrderItem>, DatabaseError> {
        OrderItem::find_for_order(self.id, conn)
    }

    pub fn tickets(
        &self,
        ticket_type_id: Uuid,
        conn: &PgConnection,
    ) -> Result<Vec<TicketInstance>, DatabaseError> {
        let items = self.items(conn)?;
        let mut tickets: Vec<OrderItem> = Vec::new();
        for ci in items {
            if ci.item_type == OrderItemTypes::Tickets && ci.ticket_type_id == Some(ticket_type_id)
            {
                tickets.push(ci);
            }
        }

        let mut result: Vec<TicketInstance> = vec![];
        for t in tickets {
            let mut instances = TicketInstance::find_for_order_item(t.id, conn)?;
            result.append(&mut instances);
        }

        Ok(result)
    }

    pub fn events(&self, conn: &PgConnection) -> Result<Vec<Event>, DatabaseError> {
        let mut unique_events: Vec<Uuid> = self
            .items(conn)?
            .iter()
            .filter_map(|i| i.event_id)
            .collect();
        unique_events.sort();
        unique_events.dedup();

        Event::find_by_ids(unique_events, conn)
    }

    pub fn purchase_metadata(
        &self,
        conn: &PgConnection,
    ) -> Result<Vec<(String, String)>, DatabaseError> {
        let query = r#"
            SELECT
                o.id as order_id,
                COALESCE(string_agg(distinct e.name, ', '), '') as event_names,
                COALESCE(string_agg(distinct e.event_start::date::character varying, ', '), '') as event_dates,
                COALESCE(string_agg(distinct v.name, ', '), '') as venue_names,
                o.user_id,
                CONCAT(u.first_name, ' ', u.last_name) as user_name,
                CAST(
                    SUM(COALESCE(oi.quantity, 0)) FILTER (WHERE oi.item_type = 'Tickets')
                AS BIGINT) as ticket_quantity,
                CAST(
                    SUM(
                        CASE WHEN oi.item_type = 'Tickets'
                        THEN
                            COALESCE(oi.unit_price_in_cents * (oi.quantity - oi.refunded_quantity), 0)
                        ELSE
                            0
                        END
                    )
                AS BIGINT) as face_value_in_cents,
                CAST(
                    SUM(
                        CASE WHEN fi.id IS NOT NULL
                        THEN (fi.quantity - fi.refunded_quantity)
                            * COALESCE(fi.unit_price_in_cents, 0)
                        ELSE (oi.quantity - oi.refunded_quantity)
                            * (COALESCE(oi.company_fee_in_cents, 0) + COALESCE(oi.client_fee_in_cents, 0))
                        END
                    )
                AS BIGINT) as fees_in_cents
            FROM orders o
            JOIN users u on u.id = COALESCE(o.on_behalf_of_user_id, o.user_id)
            LEFT JOIN order_items oi ON o.id = oi.order_id
            LEFT JOIN order_items fi on fi.parent_id = oi.id and fi.item_type = 'PerUnitFees'
            LEFT JOIN events e ON e.id = oi.event_id
            LEFT JOIN venues v ON v.id = e.venue_id
            WHERE o.id = $1
            AND (oi.item_type = 'Tickets' OR oi.item_type = 'EventFees')
            GROUP BY o.id, o.user_id, u.first_name, u.last_name
            ;
        "#;

        #[derive(QueryableByName)]
        struct R {
            #[sql_type = "dUuid"]
            order_id: Uuid,
            #[sql_type = "Text"]
            event_names: String,
            #[sql_type = "Text"]
            event_dates: String,
            #[sql_type = "Text"]
            venue_names: String,
            #[sql_type = "dUuid"]
            user_id: Uuid,
            #[sql_type = "Text"]
            user_name: String,
            #[sql_type = "BigInt"]
            ticket_quantity: i64,
            #[sql_type = "BigInt"]
            face_value_in_cents: i64,
            #[sql_type = "BigInt"]
            fees_in_cents: i64,
        }

        let order_metadata: R = diesel::sql_query(query)
            .bind::<sql_types::Uuid, _>(self.id)
            .get_result(conn)
            .to_db_error(ErrorCode::QueryError, "Could not find order metadata")?;

        Ok(vec![
            ("order_id".to_string(), order_metadata.order_id.to_string()),
            (
                "event_names".to_string(),
                order_metadata.event_names.clone(),
            ),
            (
                "event_dates".to_string(),
                order_metadata.event_dates.clone(),
            ),
            (
                "venue_names".to_string(),
                order_metadata.venue_names.clone(),
            ),
            ("user_id".to_string(), order_metadata.user_id.to_string()),
            ("user_name".to_string(), order_metadata.user_name.clone()),
            (
                "ticket_quantity".to_string(),
                order_metadata.ticket_quantity.to_string(),
            ),
            (
                "face_value_in_cents".to_string(),
                order_metadata.face_value_in_cents.to_string(),
            ),
            (
                "fees_in_cents".to_string(),
                order_metadata.fees_in_cents.to_string(),
            ),
        ])
    }

    pub fn for_display(
        &self,
        organization_ids: Option<Vec<Uuid>>,
        user_id: Uuid,
        conn: &PgConnection,
    ) -> Result<DisplayOrder, DatabaseError> {
        let now = Utc::now().naive_utc();
        let seconds_until_expiry = self.expires_at.map(|expires_at| {
            if expires_at >= now {
                let duration = expires_at.signed_duration_since(now);
                duration.num_seconds() as u32
            } else {
                0
            }
        });
        let mut limited_tickets_remaining: Vec<TicketsRemaining> = Vec::new();
        for e in self.events(conn)? {
            if let Some(ref organization_ids) = organization_ids {
                if !organization_ids.contains(&e.organization_id) {
                    continue;
                }
            }

            let tickets_bought = Order::quantity_for_user_for_event(self.user_id, e.id, conn)?;
            for (tt_id, num) in tickets_bought {
                let limit = TicketType::find(tt_id, conn)?.limit_per_person;
                if limit > 0 {
                    limited_tickets_remaining.push(TicketsRemaining {
                        ticket_type_id: tt_id,
                        tickets_remaining: limit - num,
                    });
                }
            }
        }
        // Check if this order contains any other organization items if a list of organization_ids is passed in
        let mut order_contains_other_tickets = false;
        if let Some(ref organization_ids) = organization_ids {
            order_contains_other_tickets = select(exists(
                order_items::table
                    .inner_join(orders::table.on(orders::id.eq(order_items::order_id)))
                    .inner_join(events::table.on(order_items::event_id.eq(events::id.nullable())))
                    .left_join(
                        organization_users::table
                            .on(organization_users::organization_id.eq(events::organization_id)),
                    )
                    .inner_join(users::table.on(users::id.eq(user_id)))
                    .filter(order_items::order_id.eq(self.id))
                    .filter(orders::user_id.ne(user_id))
                    .filter(organization_users::user_id.eq(user_id))
                    .filter(
                        sql("(
                        'Admin' = ANY(users.role)
                        OR orders.on_behalf_of_user_id = users.id
                        OR organization_users.id is NULL
                        OR (
                            NOT events.id = ANY(organization_users.event_ids)
                            AND (
                                    'Promoter' = ANY(organization_users.role)
                                    OR 'PromoterReadOnly' = ANY(organization_users.role)
                            )
                        )
                        OR NOT events.organization_id = ANY (")
                        .bind::<Array<dUuid>, _>(organization_ids)
                        .sql(
                            ")
                    )",
                        ),
                    ),
            ))
            .get_result(conn)
            .to_db_error(
                ErrorCode::QueryError,
                "Could not determine if order contains tickets for other organizations",
            )?;
        };
        let available_payment_methods: Vec<Vec<PaymentProviders>> = order_items::table
            .inner_join(events::table.inner_join(organizations::table))
            .filter(order_items::order_id.eq(self.id))
            .select(organizations::allowed_payment_providers)
            .distinct()
            .load(conn)
            .to_db_error(
                ErrorCode::QueryError,
                "Could not load organizations for order",
            )?;

        let allowed_payment_methods: Vec<AllowedPaymentMethod> =
            intersect_set(&available_payment_methods)
                .into_iter()
                .filter_map(|p| match p {
                    PaymentProviders::Stripe => Some(AllowedPaymentMethod {
                        method: "Card".to_string(),
                        provider: PaymentProviders::Stripe,
                        display_name: "Card".to_string(),
                    }),
                    PaymentProviders::Globee => Some(AllowedPaymentMethod {
                        method: "Provider".to_string(),
                        provider: PaymentProviders::Globee,
                        display_name: "Pay with crypto".to_string(),
                    }),
                    _ => None,
                })
                .collect();
        let items = self.items_for_display(organization_ids, user_id, conn)?;
        Ok(DisplayOrder {
            id: self.id,
            status: self.status.clone(),
            date: self.order_date,
            expires_at: self.expires_at,
            valid_for_purchase: DisplayOrder::valid_for_purchase(self.status, &items),
            items,
            limited_tickets_remaining,
            total_in_cents: self.calculate_total(conn)?,
            seconds_until_expiry,
            user_id: self.user_id,
            note: self.note.clone(),
            order_number: self.order_number(),
            paid_at: self.paid_at,
            checkout_url: if self
                .checkout_url_expires
                .unwrap_or(NaiveDateTime::from_timestamp(0, 0))
                > Utc::now().naive_utc()
            {
                self.checkout_url.clone()
            } else {
                None
            },
            allowed_payment_methods,
            order_contains_other_tickets,
        })
    }

    pub fn items_for_display(
        &self,
        organization_ids: Option<Vec<Uuid>>,
        user_id: Uuid,
        conn: &PgConnection,
    ) -> Result<Vec<DisplayOrderItem>, DatabaseError> {
        OrderItem::find_for_display(self.id, organization_ids, user_id, conn)
    }

    pub fn find_item(
        &self,
        cart_item_id: Uuid,
        conn: &PgConnection,
    ) -> Result<OrderItem, DatabaseError> {
        OrderItem::find_in_order(self.id, cart_item_id, conn)
    }

    pub fn find_item_by_type(
        &self,
        ticket_type_id: Uuid,
        item_type: OrderItemTypes,
        conn: &PgConnection,
    ) -> Result<OrderItem, DatabaseError> {
        let items = self.items(conn)?;
        let mut order_item: Vec<OrderItem> = items
            .into_iter()
            .filter(|i| i.ticket_type_id == Some(ticket_type_id) && i.item_type == item_type)
            .collect();

        match order_item.pop() {
            Some(o) => Ok(o),
            None => Err(DatabaseError::new(
                ErrorCode::NoResults,
                Some("Could not find item".to_string()),
            )),
        }
    }

    pub fn set_external_payment_type(
        &mut self,
        external_payment_type: ExternalPaymentType,
        current_user_id: Uuid,
        conn: &PgConnection,
    ) -> Result<(), DatabaseError> {
        if self.status != OrderStatus::Draft {
            return DatabaseError::validation_error(
                "status",
                "Cannot change the order user unless the order is in draft status",
            );
        }

        self.lock_version(conn)?;
        self.external_payment_type = Some(external_payment_type);
        diesel::update(&*self)
            .set((
                orders::external_payment_type.eq(self.external_payment_type),
                orders::updated_at.eq(dsl::now),
            ))
            .execute(conn)
            .to_db_error(
                ErrorCode::UpdateError,
                "Could not change the external payment type for this order",
            )?;

        DomainEvent::create(
            DomainEventTypes::OrderUpdated,
            "External payment type information recorded on order".to_string(),
            Tables::Orders,
            Some(self.id),
            Some(current_user_id),
            Some(json!({ "external_payment_type": external_payment_type })),
        )
        .commit(conn)?;
        Ok(())
    }

    pub fn add_free_payment(
        &mut self,
        external_payment: bool,
        current_user_id: Uuid,
        conn: &PgConnection,
    ) -> Result<Payment, DatabaseError> {
        let payment = Payment::create(
            self.id,
            Some(current_user_id),
            PaymentStatus::Completed,
            PaymentMethods::Free,
            if external_payment {
                PaymentProviders::External
            } else {
                PaymentProviders::Free
            },
            Some("Free Checkout".to_string()),
            0,
            None,
            None,
        );
        self.add_payment(payment, Some(current_user_id), conn)
    }

    pub fn add_external_payment(
        &mut self,
        external_reference: Option<String>,
        external_payment_type: ExternalPaymentType,
        current_user_id: Uuid,
        amount: i64,
        conn: &PgConnection,
    ) -> Result<Payment, DatabaseError> {
        self.set_external_payment_type(external_payment_type, current_user_id, conn)?;

        let payment = Payment::create(
            self.id,
            Some(current_user_id),
            PaymentStatus::Completed,
            PaymentMethods::External,
            PaymentProviders::External,
            external_reference,
            amount,
            None,
            None,
        );
        self.add_payment(payment, Some(current_user_id), conn)
    }

    pub fn add_provider_payment(
        &mut self,
        external_reference: Option<String>,
        provider: PaymentProviders,
        current_user_id: Option<Uuid>,
        amount: i64,
        status: PaymentStatus,
        url_nonce: Option<String>,
        data: Value,
        conn: &PgConnection,
    ) -> Result<Payment, DatabaseError> {
        let payment = Payment::create(
            self.id,
            current_user_id,
            status,
            PaymentMethods::Provider,
            provider,
            external_reference,
            amount,
            Some(data),
            url_nonce,
        );

        self.add_payment(payment, current_user_id, conn)
    }

    pub fn add_credit_card_payment(
        &mut self,
        current_user_id: Uuid,
        amount: i64,
        provider: PaymentProviders,
        external_reference: String,
        status: PaymentStatus,
        provider_data: serde_json::Value,
        conn: &PgConnection,
    ) -> Result<Payment, DatabaseError> {
        let payment = Payment::create(
            self.id,
            Some(current_user_id),
            status,
            PaymentMethods::CreditCard,
            provider,
            Some(external_reference),
            amount,
            Some(provider_data),
            None,
        );

        self.add_payment(payment, Some(current_user_id), conn)
    }

    pub fn add_checkout_url(
        &mut self,
        current_user_id: Uuid,
        checkout_url: String,
        expires: NaiveDateTime,
        conn: &PgConnection,
    ) -> Result<(), DatabaseError> {
        self.checkout_url = Some(checkout_url.clone());
        self.set_expiry(Some(current_user_id), Some(expires), conn)?;
        diesel::update(&*self)
            .set((
                orders::checkout_url.eq(&self.checkout_url),
                orders::checkout_url_expires.eq(expires),
                orders::updated_at.eq(dsl::now),
            ))
            .execute(conn)
            .to_db_error(ErrorCode::UpdateError, "Could not add checkout URL")?;

        DomainEvent::create(
            DomainEventTypes::OrderUpdated,
            format!("Order checkout URL added {:?}", &checkout_url),
            Tables::Orders,
            Some(self.id),
            Some(current_user_id),
            Some(json!({ "checkout_url": &checkout_url })),
        )
        .commit(conn)?;

        Ok(())
    }

    fn order_items_in_invalid_state(
        &self,
        conn: &PgConnection,
    ) -> Result<Vec<OrderItem>, DatabaseError> {
        let query = include_str!("../queries/order_items_in_invalid_state.sql");
        diesel::sql_query(query)
            .bind::<dUuid, _>(self.id)
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load invalid order items")
    }

    pub fn items_valid_for_purchase(&self, conn: &PgConnection) -> Result<bool, DatabaseError> {
        let invalid_items = self.order_items_in_invalid_state(conn)?;
        Ok(invalid_items.is_empty())
    }

    pub fn reset_to_draft(
        &mut self,
        current_user_id: Option<Uuid>,
        conn: &PgConnection,
    ) -> Result<(), DatabaseError> {
        match self.status {
            OrderStatus::Paid => {
                // still store the payment.
                DatabaseError::business_process_error(
                    "Cannot reset to draft, the order is already paid",
                )
            }
            OrderStatus::Cancelled => DatabaseError::business_process_error(
                "Cannot reset this order because it has been cancelled",
            ),

            OrderStatus::Draft => Ok(()),
            OrderStatus::PendingPayment => {
                self.update_status(current_user_id, OrderStatus::Draft, conn)
            }
        }
    }

    fn add_payment(
        &mut self,
        payment: NewPayment,
        current_user_id: Option<Uuid>,
        conn: &PgConnection,
    ) -> Result<Payment, DatabaseError> {
        //        match self.status {
        //            OrderStatus::Paid => {
        //                // still store the payment.
        //            }
        //            // orders can only expire if the order is in draft
        //            OrderStatus::Draft => {
        //             // Leave in draft,
        //            }
        //            OrderStatus::PendingPayment => {
        //
        //                // Will be checked for completion later
        //            }
        //            OrderStatus::Cancelled => {
        //
        //                // Still accept the payment so that the user's account can be credited
        //            }
        //        }

        // Confirm codes are still valid
        for item in self.items(conn)? {
            item.confirm_code_valid(conn)?;
        }

        let p = payment.commit(current_user_id, conn)?;
        if p.status != PaymentStatus::Requested {
            self.clear_user_cart(conn)?;
        }

        self.complete_if_fully_paid(current_user_id, conn)?;
        Ok(p)
    }

    pub(crate) fn complete_if_fully_paid(
        &mut self,
        current_user_id: Option<Uuid>,
        conn: &PgConnection,
    ) -> Result<(), DatabaseError> {
        if self.status == OrderStatus::Paid {
            return Ok(());
        }

        let total_paid = self.total_paid(conn)?;
        let total_required = self.calculate_total(conn)?;
        if total_paid >= total_required {
            self.update_status(current_user_id, OrderStatus::Paid, conn)?;
            //Mark tickets as Purchased
            let order_items = OrderItem::find_for_order(self.id, conn)?;
            for item in order_items
                .iter()
                .filter(|oi| oi.item_type == OrderItemTypes::Tickets)
                .collect_vec()
            {
                TicketInstance::mark_as_purchased(
                    item,
                    self.on_behalf_of_user_id.unwrap_or(self.user_id),
                    conn,
                )?;
            }

            let ticket_ids = TicketInstance::find_ids_for_order(self.id, conn)?;
            let domain_event = DomainEvent::create(
                DomainEventTypes::OrderCompleted,
                "Order completed".into(),
                Tables::Orders,
                Some(self.id),
                current_user_id,
                Some(json!({ "ticket_ids": ticket_ids })),
            )
            .commit(conn)?;

            let mut action = DomainAction::create(
                Some(domain_event.id),
                DomainActionTypes::SendPurchaseCompletedCommunication,
                None,
                json!({"order_id": self.id, "user_id": current_user_id}),
                Some(Tables::Orders.to_string()),
                Some(self.id),
            );
            action.expires_at = action.scheduled_at.into_builder().add_days(3).finish();
            action.commit(conn)?;
        };
        jlog!(Debug, "Order was checked for completion but was short", {"required_amount": total_required, "total_paid": total_paid, "order_id": self.id});
        Ok(())
    }

    fn clear_user_cart(&mut self, conn: &PgConnection) -> Result<(), DatabaseError> {
        let cart_user: Option<User> = users::table
            .filter(users::last_cart_id.eq(self.id))
            .get_result(conn)
            .to_db_error(
                ErrorCode::QueryError,
                "Could not find user attached to this cart",
            )
            .optional()?;
        if let Some(user) = cart_user {
            user.update_last_cart(None, conn)?;
        }
        Ok(())
    }

    pub fn total_paid(&self, conn: &PgConnection) -> Result<i64, DatabaseError> {
        #[derive(QueryableByName)]
        struct ResultForSum {
            #[sql_type = "Nullable<BigInt>"]
            s: Option<i64>,
        };
        let query = diesel::sql_query(
            "SELECT CAST(SUM(amount) as BigInt) as s FROM payments WHERE order_id = $1 AND status='Completed';",
        )
        .bind::<diesel::sql_types::Uuid, _>(self.id);

        let sum: ResultForSum = query.get_result(conn).to_db_error(
            ErrorCode::QueryError,
            "Could not get total payments for order",
        )?;
        Ok(sum.s.unwrap_or(0))
    }

    pub fn clear_invalid_items(
        &mut self,
        user_id: Uuid,
        conn: &PgConnection,
    ) -> Result<(), DatabaseError> {
        if self.status != OrderStatus::Draft {
            return DatabaseError::validation_error(
                "status",
                "Cannot change the order user unless the order is in draft status",
            );
        }

        self.lock_version(conn)?;

        let order_items = self.order_items_in_invalid_state(conn)?;
        for item in order_items {
            // Use calculated quantity as reserved may have been taken in the meantime
            let quantity = item.calculate_quantity(conn)?;
            TicketInstance::release_tickets(&item, quantity as u32, user_id, conn)?;
            self.destroy_item(item.id, conn)?;
        }

        Ok(())
    }

    fn update_box_office_pricing(
        &mut self,
        box_office_pricing: bool,
        current_user_id: Uuid,
        conn: &PgConnection,
    ) -> Result<(), DatabaseError> {
        let old_box_office_pricing = self.box_office_pricing;
        self.box_office_pricing = box_office_pricing;
        jlog!(Debug, "Changing order to use box office pricing", { "order_id": self.id});
        diesel::update(&*self)
            .set((
                orders::box_office_pricing.eq(&self.box_office_pricing),
                orders::updated_at.eq(dsl::now),
            ))
            .execute(conn)
            .to_db_error(ErrorCode::UpdateError, "Could not update order")?;

        DomainEvent::create(
            DomainEventTypes::OrderUpdated,
            format!(
                "Order box office pricing updated from {:?} to {:?}",
                old_box_office_pricing, box_office_pricing
            ),
            Tables::Orders,
            Some(self.id),
            Some(current_user_id),
            Some(json!({
                "old_box_office_pricing": old_box_office_pricing,
                "new_box_office_pricing": box_office_pricing
            })),
        )
        .commit(conn)?;
        Ok(())
    }

    pub(crate) fn update_status(
        &mut self,
        current_user_id: Option<Uuid>,
        status: OrderStatus,
        conn: &PgConnection,
    ) -> Result<(), DatabaseError> {
        let old_status = self.status;
        self.status = status;

        if status == OrderStatus::Paid {
            self.paid_at = Some(Utc::now().naive_utc());
            diesel::update(&*self)
                .set((
                    orders::paid_at.eq(self.paid_at),
                    orders::status.eq(&self.status),
                    orders::updated_at.eq(dsl::now),
                ))
                .execute(conn)
                .to_db_error(ErrorCode::UpdateError, "Could not mark order paid")?;
        } else {
            diesel::update(&*self)
                .set((
                    orders::status.eq(&self.status),
                    orders::updated_at.eq(dsl::now),
                ))
                .execute(conn)
                .to_db_error(ErrorCode::UpdateError, "Could not update order status")?;
        }

        DomainEvent::create(
            DomainEventTypes::OrderStatusUpdated,
            format!("Order status changed from {} to {}", &old_status, &status),
            Tables::Orders,
            Some(self.id),
            current_user_id,
            Some(json!({
                "old_status": &old_status,
                "new_status": &status
            })),
        )
        .commit(conn)?;

        Ok(())
    }

    pub fn calculate_total(&self, conn: &PgConnection) -> Result<i64, DatabaseError> {
        let order_items = self.items(conn)?;
        let mut total = 0;

        for item in &order_items {
            total += item.unit_price_in_cents * (item.quantity - item.refunded_quantity);
        }

        Ok(total)
    }

    /// Updates the lock version in the database and forces a Concurrency error if
    /// another process has updated it
    pub fn lock_version(&mut self, conn: &PgConnection) -> Result<(), DatabaseError> {
        let rows_affected = diesel::update(
            orders::table
                .filter(orders::id.eq(self.id))
                .filter(orders::version.eq(self.version)),
        )
        .set((
            orders::version.eq(self.version + 1),
            orders::updated_at.eq(dsl::now),
        ))
        .execute(conn)
        .to_db_error(ErrorCode::UpdateError, "Could not lock order")?;
        if rows_affected == 0 {
            return DatabaseError::concurrency_error(
                "Could not lock order, another process has updated it",
            );
        }
        self.version = self.version + 1;
        Ok(())
    }

    pub fn set_behalf_of_user(
        &mut self,
        user: User,
        current_user_id: Uuid,
        conn: &PgConnection,
    ) -> Result<(), DatabaseError> {
        if self.status != OrderStatus::Draft {
            return DatabaseError::validation_error(
                "status",
                "Cannot change the order user unless the order is in draft status",
            );
        }

        self.lock_version(conn)?;

        let old_id = self.on_behalf_of_user_id;
        self.on_behalf_of_user_id = Some(user.id);
        diesel::update(&*self)
            .set((
                orders::on_behalf_of_user_id.eq(user.id),
                orders::updated_at.eq(dsl::now),
            ))
            .execute(conn)
            .to_db_error(
                ErrorCode::UpdateError,
                "Could not change the behalf of user for this order",
            )?;

        DomainEvent::create(
            DomainEventTypes::OrderBehalfOfUserChanged,
            "Behalf of user on order was changed".to_string(),
            Tables::Orders,
            Some(self.id),
            Some(current_user_id),
            Some(json!({
            "old_user" : old_id, "new_user": user.id
            })),
        )
        .commit(conn)?;
        Ok(())
    }
}

#[derive(QueryableByName, Deserialize, Serialize, Debug)]
pub struct ResultForTicketTypeTotal {
    #[sql_type = "Nullable<dUuid>"]
    ticket_type_id: Option<Uuid>,
    #[sql_type = "Integer"]
    total_quantity: i32,
}

#[derive(Deserialize, Serialize)]
pub struct TicketsRemaining {
    pub ticket_type_id: Uuid,
    pub tickets_remaining: i32,
}

#[derive(Deserialize, Serialize)]
pub struct DisplayOrder {
    pub id: Uuid,
    pub date: NaiveDateTime,
    pub expires_at: Option<NaiveDateTime>,
    pub seconds_until_expiry: Option<u32>,
    pub status: OrderStatus,
    pub items: Vec<DisplayOrderItem>,
    pub limited_tickets_remaining: Vec<TicketsRemaining>,
    pub total_in_cents: i64,
    pub user_id: Uuid,
    pub note: Option<String>,
    pub order_number: String,
    pub paid_at: Option<NaiveDateTime>,
    pub checkout_url: Option<String>,
    pub allowed_payment_methods: Vec<AllowedPaymentMethod>,
    pub order_contains_other_tickets: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid_for_purchase: Option<bool>,
}

impl DisplayOrder {
    pub fn valid_for_purchase(status: OrderStatus, items: &Vec<DisplayOrderItem>) -> Option<bool> {
        if status != OrderStatus::Draft {
            return None;
        }

        Some(
            !items
                .iter()
                .find(|i| {
                    i.cart_item_status.is_some()
                        && i.cart_item_status != Some(CartItemStatus::Valid)
                })
                .is_some(),
        )
    }
}

#[derive(Serialize, Deserialize)]
pub struct AllowedPaymentMethod {
    method: String,
    provider: PaymentProviders,
    display_name: String,
}

#[derive(Deserialize, Serialize, PartialEq, Debug)]
pub struct UpdateOrderItem {
    pub ticket_type_id: Uuid,
    pub quantity: u32,
    pub redemption_code: Option<String>,
}

#[test]
fn parse_order_number() {
    let id = Uuid::parse_str("01234567-1234-1234-1234-1234567890ab").unwrap();
    assert_eq!("567890ab".to_string(), Order::parse_order_number(id));
}
