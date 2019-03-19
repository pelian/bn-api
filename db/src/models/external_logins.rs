use chrono::NaiveDateTime;
use diesel;
use diesel::prelude::*;
use models::User;
use schema::external_logins;
use utils::errors::DatabaseError;
use utils::errors::ErrorCode;
use uuid::Uuid;

pub const FACEBOOK_SITE: &str = "facebook.com";

#[derive(Identifiable, Associations, Queryable)]
#[belongs_to(User, foreign_key = "user_id")]
#[derive(Serialize, Deserialize, PartialEq, Debug)]
#[table_name = "external_logins"]
pub struct ExternalLogin {
    pub id: Uuid,
    pub user_id: Uuid,
    pub created_at: NaiveDateTime,
    pub site: String,
    pub access_token: String,
    pub external_user_id: String,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable, Serialize, Deserialize, PartialEq, Debug)]
#[table_name = "external_logins"]
pub struct NewExternalLogin {
    pub user_id: Uuid,
    pub site: String,
    pub access_token: String,
    pub external_user_id: String,
}

impl NewExternalLogin {
    pub fn commit(self, conn: &PgConnection) -> Result<ExternalLogin, DatabaseError> {
        let res = diesel::insert_into(external_logins::table)
            .values(self)
            .get_result(conn);
        DatabaseError::wrap(
            ErrorCode::InsertError,
            "Could not create new external login",
            res,
        )
    }
}

impl ExternalLogin {
    pub fn create(
        external_user_id: String,
        site: String,
        user_id: Uuid,
        access_token: String,
    ) -> NewExternalLogin {
        NewExternalLogin {
            external_user_id,
            site,
            user_id,
            access_token,
        }
    }

    pub fn find_for_site(
        user_id: Uuid,
        site: &str,
        conn: &PgConnection,
    ) -> Result<ExternalLogin, DatabaseError> {
        DatabaseError::wrap(
            ErrorCode::QueryError,
            "Error loading external login",
            external_logins::table
                .filter(external_logins::user_id.eq(user_id))
                .filter(external_logins::site.eq(site))
                .first::<ExternalLogin>(conn),
        )
    }

    pub fn find_user(
        external_user_id: &str,
        site: &str,
        conn: &PgConnection,
    ) -> Result<Option<ExternalLogin>, DatabaseError> {
        DatabaseError::wrap(
            ErrorCode::QueryError,
            "Error loading external login",
            external_logins::table
                .filter(external_logins::external_user_id.eq(external_user_id))
                .filter(external_logins::site.eq(site))
                .first::<ExternalLogin>(conn)
                .optional(),
        )
    }
}
