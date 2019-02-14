use chrono::NaiveDateTime;
use diesel;
use diesel::prelude::*;
use models::enums::Roles;
use models::{Organization, User};
use schema::organization_users;
use utils::errors::DatabaseError;
use utils::errors::ErrorCode;
use utils::errors::*;
use uuid::Uuid;
use validators::{self, *};

#[derive(Associations, Identifiable, Queryable, Serialize)]
#[belongs_to(User)]
#[belongs_to(Organization)]
#[table_name = "organization_users"]
pub struct OrganizationUser {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub user_id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub role: Vec<Roles>,
    pub event_ids: Vec<Uuid>,
}

#[derive(Insertable)]
#[table_name = "organization_users"]
pub struct NewOrganizationUser {
    pub organization_id: Uuid,
    pub user_id: Uuid,
    role: Vec<Roles>,
    event_ids: Vec<Uuid>,
}

impl NewOrganizationUser {
    pub fn validate_record(&self, conn: &PgConnection) -> Result<(), DatabaseError> {
        let validation_errors = validators::append_validation_error(
            Ok(()),
            "event_ids",
            event_ids_belong_to_organization_validation(
                true,
                self.organization_id,
                &self.event_ids,
                conn,
            )?,
        );

        Ok(validation_errors?)
    }

    pub fn commit(mut self, conn: &PgConnection) -> Result<OrganizationUser, DatabaseError> {
        // Clear limited access events list if role does not contain event limited access
        if Roles::get_event_limited_roles()
            .iter()
            .find(|r| self.role.contains(&r))
            .is_none()
        {
            self.event_ids = Vec::new();
        }

        let existing_user =
            OrganizationUser::find_by_user_id(self.user_id, self.organization_id, conn)
                .optional()?;
        match existing_user {
            Some(mut user) => {
                user.role = self.role;
                user.event_ids = self.event_ids;
                user.validate_record(conn)?;
                diesel::update(organization_users::table.filter(organization_users::id.eq(user.id)))
                    .set((
                        organization_users::role.eq(user.role),
                        organization_users::event_ids.eq(user.event_ids),
                    ))
                    .get_result(conn)
                    .to_db_error(
                        ErrorCode::UpdateError,
                        "Could not add role to existing organization user",
                    )
            }
            None => {
                self.validate_record(conn)?;

                DatabaseError::wrap(
                    ErrorCode::InsertError,
                    "Could not create new organization user",
                    diesel::insert_into(organization_users::table)
                        .values(self)
                        .get_result(conn),
                )
            }
        }
    }
}

impl OrganizationUser {
    pub fn create(
        organization_id: Uuid,
        user_id: Uuid,
        role: Vec<Roles>,
        event_ids: Vec<Uuid>,
    ) -> NewOrganizationUser {
        NewOrganizationUser {
            organization_id,
            user_id,
            role,
            event_ids,
        }
    }

    pub fn validate_record(&self, conn: &PgConnection) -> Result<(), DatabaseError> {
        let validation_errors = validators::append_validation_error(
            Ok(()),
            "event_ids",
            event_ids_belong_to_organization_validation(
                false,
                self.organization_id,
                &self.event_ids,
                conn,
            )?,
        );

        Ok(validation_errors?)
    }

    pub fn find_users_by_organization(
        organization_id: Uuid,
        conn: &PgConnection,
    ) -> Result<Vec<OrganizationUser>, DatabaseError> {
        organization_users::table
            .filter(organization_users::organization_id.eq(organization_id))
            .load(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load organization users")
    }

    pub fn find_by_user_id(
        user_id: Uuid,
        organization_id: Uuid,
        conn: &PgConnection,
    ) -> Result<OrganizationUser, DatabaseError> {
        organization_users::table
            .filter(
                organization_users::user_id
                    .eq(user_id)
                    .and(organization_users::organization_id.eq(organization_id)),
            )
            .first(conn)
            .to_db_error(
                ErrorCode::QueryError,
                "Could not find user for organization",
            )
    }
}
