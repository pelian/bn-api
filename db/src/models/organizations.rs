use chrono::{NaiveDateTime, Utc};
use diesel;
use diesel::dsl::{exists, select};
use diesel::expression::dsl;
use diesel::expression::sql_literal::sql;
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Text, Timestamp};
use models::scopes;
use models::*;
use schema::{
    assets, events, fee_schedules, order_items, organization_users, organizations, ticket_types,
    users, venues,
};
use std::collections::HashMap;
use utils::encryption::*;
use utils::errors::*;
use utils::text;
use uuid::Uuid;

#[derive(
    Identifiable,
    Associations,
    Queryable,
    QueryableByName,
    AsChangeset,
    Clone,
    Serialize,
    Deserialize,
    PartialEq,
    Debug,
)]
#[table_name = "organizations"]
pub struct Organization {
    pub id: Uuid,
    pub name: String,
    pub address: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub country: Option<String>,
    pub postal_code: Option<String>,
    pub phone: Option<String>,
    pub event_fee_in_cents: i64,
    pub sendgrid_api_key: Option<String>,
    pub google_ga_key: Option<String>,
    pub facebook_pixel_key: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub fee_schedule_id: Uuid,
    pub client_event_fee_in_cents: i64,
    pub company_event_fee_in_cents: i64,
    pub allowed_payment_providers: Vec<PaymentProviders>,
    pub timezone: Option<String>,
    pub cc_fee_percent: f32,
    pub globee_api_key: Option<String>,
}

#[derive(Serialize)]
pub struct DisplayOrganizationLink {
    pub id: Uuid,
    pub name: String,
    pub role: Vec<Roles>,
}

#[derive(Default, Insertable, Serialize, Deserialize, PartialEq, Debug, Clone)]
#[table_name = "organizations"]
pub struct NewOrganization {
    pub name: String,
    pub fee_schedule_id: Uuid,
    pub address: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub country: Option<String>,
    pub postal_code: Option<String>,
    pub phone: Option<String>,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub sendgrid_api_key: Option<String>,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub google_ga_key: Option<String>,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub facebook_pixel_key: Option<String>,
    pub client_event_fee_in_cents: Option<i64>,
    pub company_event_fee_in_cents: Option<i64>,
    pub allowed_payment_providers: Option<Vec<PaymentProviders>>,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub timezone: Option<String>,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub globee_api_key: Option<String>,
}

#[derive(Default, Serialize, Clone)]
pub struct TrackingKeys {
    pub google_ga_key: Option<String>,
    pub facebook_pixel_key: Option<String>,
}

impl NewOrganization {
    pub fn commit(
        self,
        encryption_key: &str,
        current_user_id: Uuid,
        conn: &PgConnection,
    ) -> Result<Organization, DatabaseError> {
        let mut updated_organisation = self;
        if encryption_key.len() > 0 {
            if let Some(key) = updated_organisation.sendgrid_api_key.clone() {
                updated_organisation.sendgrid_api_key = Some(encrypt(&key, encryption_key)?);
            }
            if let Some(key) = updated_organisation.google_ga_key.clone() {
                updated_organisation.google_ga_key = Some(encrypt(&key, encryption_key)?);
            }
            if let Some(key) = updated_organisation.facebook_pixel_key.clone() {
                updated_organisation.facebook_pixel_key = Some(encrypt(&key, encryption_key)?);
            }
            if let Some(key) = updated_organisation.globee_api_key.clone() {
                updated_organisation.globee_api_key = Some(encrypt(&key, encryption_key)?);
            }
        }

        let org: Organization = diesel::insert_into(organizations::table)
            .values((
                &updated_organisation,
                organizations::event_fee_in_cents
                    .eq(updated_organisation.client_event_fee_in_cents.unwrap_or(0)
                        + updated_organisation.company_event_fee_in_cents.unwrap_or(0)),
            ))
            .get_result(conn)
            .to_db_error(ErrorCode::InsertError, "Could not create new organization")?;

        diesel::update(fee_schedules::table.filter(fee_schedules::id.eq(org.fee_schedule_id)))
            .set((
                fee_schedules::organization_id.eq(org.id),
                fee_schedules::updated_at.eq(dsl::now),
            ))
            .execute(conn)
            .to_db_error(
                ErrorCode::UpdateError,
                "Could not set the fee schedule for this organization",
            )?;

        DomainEvent::create(
            DomainEventTypes::OrganizationCreated,
            "Organization created".to_string(),
            Tables::Organizations,
            Some(org.id),
            Some(current_user_id),
            Some(json!(updated_organisation)),
        )
        .commit(conn)?;

        Ok(org)
    }
}

#[derive(AsChangeset, Default, Deserialize)]
#[table_name = "organizations"]
pub struct OrganizationEditableAttributes {
    pub name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub address: Option<String>,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub city: Option<String>,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub state: Option<String>,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub country: Option<String>,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub postal_code: Option<String>,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub phone: Option<String>,
    #[serde(default, deserialize_with = "double_option_deserialize_unless_blank")]
    pub sendgrid_api_key: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option_deserialize_unless_blank")]
    pub google_ga_key: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option_deserialize_unless_blank")]
    pub facebook_pixel_key: Option<Option<String>>,
    pub client_event_fee_in_cents: Option<i64>,
    pub company_event_fee_in_cents: Option<i64>,
    pub allowed_payment_providers: Option<Vec<PaymentProviders>>,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub timezone: Option<String>,
    pub cc_fee_percent: Option<f32>,
    #[serde(default, deserialize_with = "double_option_deserialize_unless_blank")]
    pub globee_api_key: Option<Option<String>>,
}

impl Organization {
    pub fn create(name: &str, fee_schedule_id: Uuid) -> NewOrganization {
        NewOrganization {
            name: name.into(),
            fee_schedule_id,
            ..Default::default()
        }
    }

    pub fn update(
        &self,
        mut attributes: OrganizationEditableAttributes,
        encryption_key: &String,
        conn: &PgConnection,
    ) -> Result<Organization, DatabaseError> {
        if encryption_key.len() > 0 {
            if let Some(Some(key)) = attributes.sendgrid_api_key {
                attributes.sendgrid_api_key = Some(Some(encrypt(&key, encryption_key)?));
            }
            if let Some(Some(key)) = attributes.google_ga_key {
                attributes.google_ga_key = Some(Some(encrypt(&key, encryption_key)?));
            }
            if let Some(Some(key)) = attributes.facebook_pixel_key {
                attributes.facebook_pixel_key = Some(Some(encrypt(&key, encryption_key)?));
            }
            if let Some(Some(key)) = attributes.globee_api_key {
                attributes.globee_api_key = Some(Some(encrypt(&key, encryption_key)?));
            }
        }

        let event_fee = attributes
            .client_event_fee_in_cents
            .clone()
            .unwrap_or(self.client_event_fee_in_cents)
            + attributes
                .company_event_fee_in_cents
                .clone()
                .unwrap_or(self.company_event_fee_in_cents);

        diesel::update(&*self)
            .set((
                attributes,
                organizations::updated_at.eq(dsl::now),
                organizations::event_fee_in_cents.eq(event_fee),
            ))
            .get_result(conn)
            .to_db_error(ErrorCode::UpdateError, "Could not update organization")
    }

    pub fn find_by_asset_id(
        asset_id: Uuid,
        conn: &PgConnection,
    ) -> Result<Organization, DatabaseError> {
        organizations::table
            .inner_join(events::table.on(events::organization_id.eq(organizations::id)))
            .inner_join(ticket_types::table.on(ticket_types::event_id.eq(events::id)))
            .inner_join(assets::table.on(assets::ticket_type_id.eq(ticket_types::id)))
            .filter(assets::id.eq(asset_id))
            .select(organizations::all_columns)
            .get_result(conn)
            .to_db_error(
                ErrorCode::QueryError,
                "Could not retrieve organization by asset id",
            )
    }

    pub fn find_by_ticket_type_ids(
        ticket_type_ids: Vec<Uuid>,
        conn: &PgConnection,
    ) -> Result<Vec<Organization>, DatabaseError> {
        organizations::table
            .inner_join(events::table.on(events::organization_id.eq(organizations::id)))
            .inner_join(ticket_types::table.on(ticket_types::event_id.eq(events::id)))
            .filter(ticket_types::id.eq_any(ticket_type_ids))
            .select(organizations::all_columns)
            .distinct()
            .get_results(conn)
            .to_db_error(
                ErrorCode::QueryError,
                "Could not retrieve organizations by ticket type ids",
            )
    }

    pub fn find_by_order_item_ids(
        order_item_ids: &Vec<Uuid>,
        conn: &PgConnection,
    ) -> Result<Vec<Organization>, DatabaseError> {
        organizations::table
            .inner_join(events::table.on(events::organization_id.eq(organizations::id)))
            .inner_join(ticket_types::table.on(ticket_types::event_id.eq(events::id)))
            .inner_join(
                order_items::table.on(order_items::ticket_type_id.eq(ticket_types::id.nullable())),
            )
            .filter(order_items::id.eq_any(order_item_ids))
            .select(organizations::all_columns)
            .order_by(organizations::name.asc())
            .distinct()
            .load(conn)
            .to_db_error(ErrorCode::QueryError, "Error loading organizations")
    }

    pub fn users(
        &self,
        event_id: Option<Uuid>,
        conn: &PgConnection,
    ) -> Result<Vec<(OrganizationUser, User)>, DatabaseError> {
        let query = organization_users::table
            .inner_join(users::table)
            .filter(organization_users::organization_id.eq(self.id))
            .into_boxed();

        let query = match event_id {
            Some(id) => query.filter(organization_users::event_ids.contains(vec![id])),
            None => query.filter(organization_users::event_ids.eq(Vec::<Uuid>::new())),
        };

        let users = query
            .select(organization_users::all_columns)
            .order_by(users::last_name.asc())
            .then_order_by(users::first_name.asc())
            .load::<OrganizationUser>(conn)
            .to_db_error(
                ErrorCode::QueryError,
                "Could not retrieve organization users",
            )?;

        let mut result = vec![];

        for u in users {
            let user = User::find(u.user_id, conn)?;
            result.push((u, user));
        }

        Ok(result)
    }

    pub fn pending_invites(
        &self,
        event_id: Option<Uuid>,
        conn: &PgConnection,
    ) -> Result<Vec<OrganizationInvite>, DatabaseError> {
        OrganizationInvite::find_pending_by_organization(self.id, event_id, conn)
    }

    pub fn events(&self, conn: &PgConnection) -> Result<Vec<Event>, DatabaseError> {
        events::table
            .filter(events::organization_id.eq(self.id))
            .order_by(events::created_at)
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Could not retrieve events")
    }

    pub fn upcoming_events(&self, conn: &PgConnection) -> Result<Vec<Event>, DatabaseError> {
        events::table
            .filter(events::organization_id.eq(self.id))
            .filter(events::status.eq(EventStatus::Published))
            .filter(events::event_start.ge(Utc::now().naive_utc()))
            .order_by(events::created_at)
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Could not retrieve upcoming events")
    }

    pub fn venues(&self, conn: &PgConnection) -> Result<Vec<Venue>, DatabaseError> {
        venues::table
            .filter(venues::organization_id.eq(self.id))
            .order_by(venues::name)
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Could not retrieve venues")
    }

    pub fn has_fan(&self, user: &User, conn: &PgConnection) -> Result<bool, DatabaseError> {
        use schema::*;

        select(exists(
            order_items::table
                .inner_join(orders::table.on(order_items::order_id.eq(orders::id)))
                .inner_join(events::table.on(order_items::event_id.eq(events::id.nullable())))
                .filter(orders::status.eq(OrderStatus::Paid))
                .filter(events::organization_id.eq(self.id))
                .filter(orders::user_id.eq(user.id)),
        ))
        .get_result(conn)
        .to_db_error(
            ErrorCode::QueryError,
            "Could not check if organization has fan",
        )
    }

    pub fn get_scopes_for_user(
        &self,
        user: &User,
        conn: &PgConnection,
    ) -> Result<Vec<Scopes>, DatabaseError> {
        Ok(scopes::get_scopes(self.get_roles_for_user(user, conn)?))
    }

    pub fn get_roles_for_user(
        &self,
        user: &User,
        conn: &PgConnection,
    ) -> Result<Vec<Roles>, DatabaseError> {
        if user.is_admin() {
            let mut roles = Vec::new();

            roles.push(Roles::OrgOwner);

            Ok(roles)
        } else {
            let org_member =
                OrganizationUser::find_by_user_id(user.id, self.id, conn).optional()?;
            match org_member {
                Some(member) => Ok(member.role),
                None => Ok(vec![]),
            }
        }
    }

    pub fn find(id: Uuid, conn: &PgConnection) -> Result<Organization, DatabaseError> {
        organizations::table
            .find(id)
            .first::<Organization>(conn)
            .to_db_error(ErrorCode::QueryError, "Error loading organization")
    }

    pub fn find_for_event(
        event_id: Uuid,
        conn: &PgConnection,
    ) -> Result<Organization, DatabaseError> {
        events::table
            .inner_join(organizations::table)
            .filter(events::id.eq(event_id))
            .select(organizations::all_columns)
            .first(conn)
            .to_db_error(
                ErrorCode::QueryError,
                "Could not find organization for this event",
            )
    }

    pub fn tracking_keys_for_ids(
        org_ids: Vec<Uuid>,
        encryption_key: &String,
        conn: &PgConnection,
    ) -> Result<HashMap<Uuid, TrackingKeys>, DatabaseError> {
        #[derive(Queryable)]
        struct TrackingKeyFields {
            pub id: Uuid,
            pub google_ga_key: Option<String>,
            pub facebook_pixel_key: Option<String>,
        };

        let orgs: Vec<TrackingKeyFields> = organizations::table
            .filter(organizations::id.eq_any(org_ids))
            .select((
                organizations::id,
                organizations::google_ga_key,
                organizations::facebook_pixel_key,
            ))
            .get_results(conn)
            .to_db_error(
                ErrorCode::QueryError,
                "Unable to load all organization tracking keys",
            )?;

        let mut result: HashMap<Uuid, TrackingKeys> = HashMap::new();

        for org in orgs {
            let mut google_ga_key = org.google_ga_key;
            if let Some(key) = google_ga_key.clone() {
                google_ga_key = Some(decrypt(&key, &encryption_key)?);
            }
            let mut facebook_pixel_key = org.facebook_pixel_key;
            if let Some(key) = facebook_pixel_key.clone() {
                facebook_pixel_key = Some(decrypt(&key, &encryption_key)?);
            }

            result.insert(
                org.id,
                TrackingKeys {
                    google_ga_key,
                    facebook_pixel_key,
                },
            );
        }

        Ok(result)
    }

    pub fn all(conn: &PgConnection) -> Result<Vec<Organization>, DatabaseError> {
        DatabaseError::wrap(
            ErrorCode::QueryError,
            "Unable to load all organizations",
            organizations::table
                .order_by(organizations::name)
                .load(conn),
        )
    }

    pub fn all_linked_to_user(
        user_id: Uuid,
        conn: &PgConnection,
    ) -> Result<Vec<Organization>, DatabaseError> {
        let orgs = organization_users::table
            .filter(organization_users::user_id.eq(user_id))
            .inner_join(organizations::table)
            .select(organizations::all_columns)
            .order_by(organizations::name.asc())
            .load::<Organization>(conn)
            .to_db_error(ErrorCode::QueryError, "Unable to load all organizations")?;

        Ok(orgs)
    }

    pub fn all_org_names_linked_to_user(
        user_id: Uuid,
        conn: &PgConnection,
    ) -> Result<Vec<DisplayOrganizationLink>, DatabaseError> {
        //Compile list of organisations where the user is a member of that organisation
        let org_member_list: Vec<OrganizationUser> = organization_users::table
            .filter(organization_users::user_id.eq(user_id))
            .inner_join(organizations::table)
            .select(organization_users::all_columns)
            .load(conn)
            .to_db_error(ErrorCode::QueryError, "Unable to load all organizations")?;
        //Compile complete list with only display information
        let mut result_list: Vec<DisplayOrganizationLink> = Vec::new();

        for curr_org_member in &org_member_list {
            let curr_entry = DisplayOrganizationLink {
                id: curr_org_member.organization_id,
                name: Organization::find(curr_org_member.organization_id, conn)?.name,
                role: curr_org_member.role.clone(),
            };
            result_list.push(curr_entry);
        }
        result_list.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(result_list)
    }

    pub fn remove_user(&self, user_id: Uuid, conn: &PgConnection) -> Result<usize, DatabaseError> {
        diesel::delete(
            organization_users::table
                .filter(organization_users::user_id.eq(user_id))
                .filter(organization_users::organization_id.eq(self.id)),
        )
        .execute(conn)
        .to_db_error(ErrorCode::DeleteError, "Error removing user")
    }

    pub fn add_user(
        &self,
        user_id: Uuid,
        role: Vec<Roles>,
        event_ids: Vec<Uuid>,
        conn: &PgConnection,
    ) -> Result<OrganizationUser, DatabaseError> {
        let org_user = OrganizationUser::create(self.id, user_id, role, event_ids).commit(conn)?;
        Ok(org_user)
    }

    pub fn is_member(&self, user: &User, conn: &PgConnection) -> Result<bool, DatabaseError> {
        let query = select(exists(
            organization_users::table
                .filter(
                    organization_users::user_id
                        .eq(user.id)
                        .and(organization_users::organization_id.eq(self.id)),
                )
                .select(organization_users::organization_id),
        ));
        query
            .get_result(conn)
            .to_db_error(ErrorCode::QueryError, "Could not check user member status")
    }

    pub fn add_fee_schedule(
        &self,
        fee_schedule: &FeeSchedule,
        conn: &PgConnection,
    ) -> Result<Organization, DatabaseError> {
        diesel::update(fee_schedule)
            .set((
                fee_schedules::organization_id.eq(self.id),
                fee_schedules::updated_at.eq(dsl::now),
            ))
            .execute(conn)
            .to_db_error(
                ErrorCode::UpdateError,
                "Could not set the fee schedule for this organization",
            )?;

        diesel::update(self)
            .set((
                organizations::fee_schedule_id.eq(fee_schedule.id),
                organizations::updated_at.eq(dsl::now),
            ))
            .get_result(conn)
            .to_db_error(
                ErrorCode::UpdateError,
                "Could not set the fee schedule for this organization",
            )
    }

    pub fn search_fans(
        &self,
        query: Option<String>,
        page: u32,
        limit: u32,
        sort_field: FanSortField,
        sort_direction: SortingDir,
        conn: &PgConnection,
    ) -> Result<Payload<DisplayFan>, DatabaseError> {
        use schema::*;

        let search_filter = query
            .map(|s| text::escape_control_chars(&s))
            .map(|s| format!("%{}%", s))
            .unwrap_or("%".to_string());

        let sort_column = match sort_field {
            FanSortField::FirstName => "2",
            FanSortField::LastName => "3",
            FanSortField::Email => "4",
            FanSortField::Phone => "5",
            FanSortField::Orders => "7",
            FanSortField::FirstOrder => "8",
            FanSortField::LastOrder => "9",
            FanSortField::Revenue => "10",
        };

        let query = order_items::table
            .inner_join(orders::table.on(order_items::order_id.eq(orders::id)))
            .inner_join(users::table.on(users::id.eq(orders::user_id)))
            .inner_join(events::table.on(order_items::event_id.eq(events::id.nullable())))
            .filter(orders::status.eq(OrderStatus::Paid))
            .filter(events::organization_id.eq(self.id))
            .filter(
                sql("(")
                    .sql("users.first_name ilike ")
                    .bind::<Text, _>(&search_filter)
                    .sql(" OR users.last_name ilike ")
                    .bind::<Text, _>(&search_filter)
                    .sql(" OR users.email ilike ")
                    .bind::<Text, _>(&search_filter)
                    .sql(" OR users.phone ilike ")
                    .bind::<Text, _>(&search_filter)
                    .sql(")"),
            )
            .group_by((
                events::organization_id,
                users::first_name,
                users::last_name,
                users::email,
                users::phone,
                users::id,
            ))
            .select((
                events::organization_id,
                users::first_name,
                users::last_name,
                users::email,
                users::phone,
                users::thumb_profile_pic_url,
                users::id,
                sql::<BigInt>("count(distinct orders.id)"),
                users::created_at,
                sql::<Timestamp>("min(orders.order_date)"),
                sql::<Timestamp>("max(orders.order_date)"),
                sql::<BigInt>(
                    "cast(sum(order_items.unit_price_in_cents * (order_items.quantity - order_items.refunded_quantity)) as bigint)",
                ),
                sql::<BigInt>("count(*) over()"),
            ))
            .order_by(sql::<()>(&format!("{} {}", sort_column, sort_direction)));

        let query = query.limit(limit as i64).offset((limit * page) as i64);

        #[derive(Queryable)]
        struct R {
            organization_id: Uuid,
            first_name: Option<String>,
            last_name: Option<String>,
            email: Option<String>,
            phone: Option<String>,
            thumb_profile_pic_url: Option<String>,
            user_id: Uuid,
            order_count: i64,
            created_at: NaiveDateTime,
            first_order_time: NaiveDateTime,
            last_order_time: NaiveDateTime,
            revenue_in_cents: i64,
            total_rows: i64,
        }

        let results: Vec<R> = query.get_results(conn).to_db_error(
            ErrorCode::QueryError,
            "Could not load fans for organization",
        )?;

        let paging = Paging::new(page, limit);
        let mut total = results.len() as u64;
        if !results.is_empty() {
            total = results[0].total_rows as u64;
        }

        let fans = results
            .into_iter()
            .map(|r| DisplayFan {
                user_id: r.user_id,
                first_name: r.first_name,
                last_name: r.last_name,
                email: r.email,
                phone: r.phone,
                thumb_profile_pic_url: r.thumb_profile_pic_url,
                organization_id: r.organization_id,
                order_count: Some(r.order_count as u32),
                created_at: r.created_at,
                first_order_time: Some(r.first_order_time),
                last_order_time: Some(r.last_order_time),
                revenue_in_cents: Some(r.revenue_in_cents),
            })
            .collect();

        let mut p = Payload::new(fans, paging);
        p.paging.total = total;
        Ok(p)
    }

    pub fn decrypt(&mut self, encryption_key: &str) -> Result<(), DatabaseError> {
        if encryption_key.len() > 0 {
            if let Some(key) = self.sendgrid_api_key.clone() {
                self.sendgrid_api_key = Some(decrypt(&key, &encryption_key)?);
            }
            if let Some(key) = self.google_ga_key.clone() {
                self.google_ga_key = Some(decrypt(&key, &encryption_key)?);
            }
            if let Some(key) = self.facebook_pixel_key.clone() {
                self.facebook_pixel_key = Some(decrypt(&key, &encryption_key)?);
            }
            if let Some(key) = self.globee_api_key.clone() {
                self.globee_api_key = Some(decrypt(&key, &encryption_key)?);
            }
        }

        Ok(())
    }
}
