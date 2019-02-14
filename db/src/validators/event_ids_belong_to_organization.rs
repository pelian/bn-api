use diesel::dsl::select;
use diesel::pg::types::sql_types::Array;
use diesel::prelude::*;
use diesel::sql_types::Uuid as dUuid;
use std::borrow::Cow;
use utils::errors::*;
use uuid::Uuid;
use validator::*;
use validators::*;

sql_function!(fn event_ids_belong_to_organization(organization_id: dUuid, event_ids: Array<dUuid>) -> Bool);

pub fn event_ids_belong_to_organization_validation(
    new_record: bool,
    organization_id: Uuid,
    event_ids: &Vec<Uuid>,
    conn: &PgConnection,
) -> Result<Result<(), ValidationError>, DatabaseError> {
    let result = select(event_ids_belong_to_organization(organization_id, event_ids))
        .get_result::<bool>(conn)
        .to_db_error(
            if new_record {
                ErrorCode::InsertError
            } else {
                ErrorCode::UpdateError
            },
            "Could not confirm if event ids for organization user belong to organization",
        )?;
    if !result {
        let mut validation_error = create_validation_error(
            "event_ids_do_not_belong_to_organization",
            "Event ids invalid for organization user",
        );
        validation_error.add_param(Cow::from("event_ids"), &event_ids);
        validation_error.add_param(Cow::from("organization_id"), &organization_id);
        return Ok(Err(validation_error));
    }
    Ok(Ok(()))
}
