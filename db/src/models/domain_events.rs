use chrono::prelude::*;
use diesel;
use diesel::expression::dsl;
use diesel::prelude::*;
use log::Level::Info;
use models::enums::*;
use schema::*;
use serde_json;
use std::cmp::Ordering;
use utils::errors::*;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Identifiable, Queryable, Serialize, Deserialize)]
pub struct DomainEvent {
    pub id: Uuid,
    pub event_type: DomainEventTypes,
    pub display_text: String,
    pub event_data: Option<serde_json::Value>,
    pub main_table: Tables,
    pub main_id: Option<Uuid>,
    pub published_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub user_id: Option<Uuid>,
}

impl PartialOrd for DomainEvent {
    fn partial_cmp(&self, other: &DomainEvent) -> Option<Ordering> {
        Some(self.id.cmp(&other.id))
    }
}

impl DomainEvent {
    pub fn create(
        event_type: DomainEventTypes,
        display_text: String,
        main_table: Tables,
        main_id: Option<Uuid>,
        user_id: Option<Uuid>,
        event_data: Option<serde_json::Value>,
    ) -> NewDomainEvent {
        NewDomainEvent {
            event_type,
            display_text,
            event_data,
            main_table,
            main_id,
            user_id,
        }
    }

    pub fn find(
        main_table: Tables,
        main_id: Option<Uuid>,
        event_type: Option<DomainEventTypes>,
        conn: &PgConnection,
    ) -> Result<Vec<DomainEvent>, DatabaseError> {
        let mut query = domain_events::table
            .filter(domain_events::main_table.eq(main_table))
            .filter(domain_events::main_id.eq(main_id))
            .into_boxed();

        if let Some(event_type) = event_type {
            query = query.filter(domain_events::event_type.eq(event_type));
        }

        query
            .order_by(domain_events::created_at)
            .load(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load domain events")
    }

    pub fn find_unpublished(
        limit: u32,
        conn: &PgConnection,
    ) -> Result<Vec<DomainEvent>, DatabaseError> {
        domain_events::table
            .filter(domain_events::published_at.is_null())
            .order_by(domain_events::created_at)
            .limit(limit as i64)
            .get_results(conn)
            .to_db_error(
                ErrorCode::QueryError,
                "Could not load unpublished domain events",
            )
    }

    pub fn mark_as_published(self, conn: &PgConnection) -> Result<DomainEvent, DatabaseError> {
        diesel::update(&self)
            .set(domain_events::published_at.eq(dsl::now.nullable()))
            .get_result(conn)
            .to_db_error(
                ErrorCode::UpdateError,
                "Could not mark domain event as published",
            )
    }
}

#[derive(Insertable, Clone)]
#[table_name = "domain_events"]
pub struct NewDomainEvent {
    pub event_type: DomainEventTypes,
    pub display_text: String,
    pub event_data: Option<serde_json::Value>,
    pub main_table: Tables,
    pub main_id: Option<Uuid>,
    pub user_id: Option<Uuid>,
}

impl NewDomainEvent {
    pub fn commit(self, conn: &PgConnection) -> Result<DomainEvent, DatabaseError> {
        let result: DomainEvent = diesel::insert_into(domain_events::table)
            .values(&self)
            .get_result(conn)
            .to_db_error(ErrorCode::InsertError, "Could not insert domain event")?;

        jlog!(Info, &format!("Domain Event: {} `{}` on {}:{}", self.event_type,
            self.display_text, self.main_table, self.main_id.map(|i| i.to_string()).unwrap_or_default()),{"domain_event_id": result.id,


            "event_type": self.event_type.clone(), "main_table": self.main_table.clone(), "main_id": self.main_id, "event_data": self.event_data });

        Ok(result)
    }
}
