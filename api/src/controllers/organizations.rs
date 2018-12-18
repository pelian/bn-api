use actix_web::{http::StatusCode, HttpResponse, Path, Query, State};
use auth::user::User;
use bigneon_db::models::*;
use chrono::NaiveDateTime;
use db::Connection;
use errors::*;
use extractors::*;
use helpers::application;
use models::WebPayload;
use models::{OrganizationUserPathParameters, PathParameters};
use server::AppState;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct AddUserRequest {
    pub user_id: Uuid,
    pub roles: Vec<Roles>,
}

#[derive(Serialize, Deserialize)]
pub struct FeeScheduleWithRanges {
    pub id: Uuid,
    pub name: String,
    pub version: i16,
    pub created_at: NaiveDateTime,
    pub ranges: Vec<FeeScheduleRange>,
}

#[derive(Serialize, Deserialize)]
pub struct NewOrganizationRequest {
    pub name: String,
    pub event_fee_in_cents: Option<i64>,
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
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub sendgrid_api_key: Option<String>,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub google_ga_key: Option<String>,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub facebook_pixel_key: Option<String>,
}

pub fn index(
    (connection, query_parameters, user): (Connection, Query<PagingParameters>, User),
) -> Result<HttpResponse, BigNeonError> {
    if user.requires_scope(Scopes::OrgAdmin).is_ok() {
        return index_for_all_orgs((connection, query_parameters, user));
    }

    //TODO remap query to use paging info
    let organizations = Organization::all_linked_to_user(user.id(), connection.get())?;

    Ok(HttpResponse::Ok().json(&Payload::from_data(
        organizations,
        query_parameters.page(),
        query_parameters.limit(),
    )))
}

pub fn index_for_all_orgs(
    (connection, query_parameters, user): (Connection, Query<PagingParameters>, User),
) -> Result<HttpResponse, BigNeonError> {
    user.requires_scope(Scopes::OrgAdmin)?;
    let connection = connection.get();
    let organizations = Organization::all(connection)?;

    Ok(HttpResponse::Ok().json(&Payload::from_data(
        organizations,
        query_parameters.page(),
        query_parameters.limit(),
    )))
}

pub fn show(
    (state, connection, parameters, user): (
        State<AppState>,
        Connection,
        Path<PathParameters>,
        User,
    ),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let mut organization = Organization::find(parameters.id, connection)?;
    user.requires_scope_for_organization(Scopes::OrgRead, &organization, connection)?;

    organization.decrypt(&state.config.api_keys_encryption_key)?;

    Ok(HttpResponse::Ok().json(&organization))
}

pub fn create(
    (state, connection, new_organization, user): (
        State<AppState>,
        Connection,
        Json<NewOrganizationRequest>,
        User,
    ),
) -> Result<HttpResponse, BigNeonError> {
    user.requires_scope(Scopes::OrgAdmin)?;
    let connection = connection.get();

    let fee_schedule = FeeSchedule::create(
        format!("{} default fees", new_organization.name),
        vec![NewFeeScheduleRange {
            min_price_in_cents: 0,
            company_fee_in_cents: 0,
            client_fee_in_cents: 0,
        }],
    )
    .commit(connection)?;

    let new_organization_with_fee_schedule = NewOrganization {
        name: new_organization.name.clone(),
        fee_schedule_id: fee_schedule.id,
        event_fee_in_cents: new_organization.event_fee_in_cents.clone(),
        address: new_organization.address.clone(),
        city: new_organization.city.clone(),
        state: new_organization.state.clone(),
        country: new_organization.country.clone(),
        postal_code: new_organization.postal_code.clone(),
        phone: new_organization.phone.clone(),
        sendgrid_api_key: new_organization.sendgrid_api_key.clone(),
        google_ga_key: new_organization.google_ga_key.clone(),
        facebook_pixel_key: new_organization.facebook_pixel_key.clone(),
    };

    let mut organization = new_organization_with_fee_schedule
        .commit(&state.config.api_keys_encryption_key, connection)?;

    organization.decrypt(&state.config.api_keys_encryption_key)?;

    Wallet::create_for_organization(organization.id, "Default".to_string(), connection)?;

    Ok(HttpResponse::Created().json(&organization))
}

pub fn update(
    (state, connection, parameters, organization_parameters, user): (
        State<AppState>,
        Connection,
        Path<PathParameters>,
        Json<OrganizationEditableAttributes>,
        User,
    ),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let organization = Organization::find(parameters.id, connection)?;
    user.requires_scope_for_organization(Scopes::OrgWrite, &organization, connection)?;
    let organization_update = organization_parameters.into_inner();
    let mut updated_organization = organization.update(
        organization_update,
        &state.config.api_keys_encryption_key,
        connection,
    )?;

    updated_organization.decrypt(&state.config.api_keys_encryption_key)?;
    Ok(HttpResponse::Ok().json(&updated_organization))
}

pub fn add_venue(
    (connection, parameters, new_venue, user): (
        Connection,
        Path<PathParameters>,
        Json<NewVenue>,
        User,
    ),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let organization = Organization::find(parameters.id, connection)?;
    user.requires_scope_for_organization(Scopes::OrgWrite, &organization, connection)?;

    let mut new_venue = new_venue.into_inner();
    new_venue.organization_id = Some(parameters.id);
    let venue = new_venue.commit(connection)?;
    Ok(HttpResponse::Created().json(&venue))
}

pub fn add_artist(
    (connection, parameters, new_artist, user): (
        Connection,
        Path<PathParameters>,
        Json<NewArtist>,
        User,
    ),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let organization = Organization::find(parameters.id, connection)?;
    user.requires_scope_for_organization(Scopes::OrgWrite, &organization, connection)?;

    let mut new_artist = new_artist.into_inner();
    new_artist.organization_id = Some(parameters.id);

    let artist = new_artist.commit(connection)?;
    Ok(HttpResponse::Created().json(&artist))
}

pub fn add_or_replace_user(
    (connection, path, json, user): (Connection, Path<PathParameters>, Json<AddUserRequest>, User),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let organization = Organization::find(path.id, connection)?;

    let req = json.into_inner();

    for role in req.roles.iter() {
        match role {
            Roles::OrgOwner => {
                user.requires_scope_for_organization(Scopes::OrgAdmin, &organization, connection)?
            }
            Roles::OrgAdmin => user.requires_scope_for_organization(
                Scopes::OrgManageAdminUsers,
                &organization,
                connection,
            )?,
            Roles::OrgMember => user.requires_scope_for_organization(
                Scopes::OrgManageUsers,
                &organization,
                connection,
            )?,
            Roles::DoorPerson => user.requires_scope_for_organization(
                Scopes::OrgManageUsers,
                &organization,
                connection,
            )?,
            Roles::OrgBoxOffice => user.requires_scope_for_organization(
                Scopes::OrgManageUsers,
                &organization,
                connection,
            )?,
            _ => return application::forbidden("Role is not allowed for this user"),
        };
    }

    organization.add_user(req.user_id, req.roles, connection)?;
    Ok(HttpResponse::Created().finish())
}

pub fn remove_user(
    (connection, parameters, user): (Connection, Path<OrganizationUserPathParameters>, User),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let organization = Organization::find(parameters.id, connection)?;
    user.requires_scope_for_organization(Scopes::OrgManageUsers, &organization, connection)?;

    let organization = organization.remove_user(parameters.user_id, connection)?;
    Ok(HttpResponse::Ok().json(&organization))
}

pub fn list_organization_members(
    (connection, path_parameters, query_parameters, user): (
        Connection,
        Path<PathParameters>,
        Query<PagingParameters>,
        User,
    ),
) -> Result<WebPayload<DisplayOrganizationUser>, BigNeonError> {
    let connection = connection.get();
    //TODO refactor Organization::find to use limits as in PagingParameters
    let organization = Organization::find(path_parameters.id, connection)?;
    user.requires_scope_for_organization(Scopes::OrgRead, &organization, connection)?;

    let mut members: Vec<DisplayOrganizationUser> = organization
        .users(connection)?
        .into_iter()
        .map(|u| DisplayOrganizationUser {
            user_id: Some(u.1.id),
            first_name: u.1.first_name,
            last_name: u.1.last_name,
            email: u.1.email,
            roles: u.0.role.iter().map(|r| r.to_string()).collect(),
            invite_or_member: "member".to_string(),
        })
        .collect();

    for inv in organization.pending_invites(connection)? {
        members.push(DisplayOrganizationUser {
            user_id: inv.user_id,
            first_name: None,
            last_name: None,
            email: Some(inv.user_email),
            roles: vec![format!("{} (Invited)", inv.role)],
            invite_or_member: "invite".to_string(),
        });
    }

    let payload = Payload::from_data(members, query_parameters.page(), query_parameters.limit());
    Ok(WebPayload::new(StatusCode::OK, payload))
}

#[derive(Serialize, PartialEq, Debug)]
pub struct DisplayOrganizationUser {
    pub user_id: Option<Uuid>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: Option<String>,
    pub roles: Vec<String>,
    pub invite_or_member: String,
}

pub fn show_fee_schedule(
    (connection, parameters, user): (Connection, Path<PathParameters>, User),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let organization = Organization::find(parameters.id, connection)?;
    user.requires_scope_for_organization(Scopes::OrgWrite, &organization, connection)?;

    //This is an OrgOwner / Admin only call so we need to show the breakdown
    let fee_schedule = FeeSchedule::find(organization.fee_schedule_id, connection)?;
    let fee_schedule_ranges = fee_schedule.ranges(connection)?;

    Ok(HttpResponse::Ok().json(FeeScheduleWithRanges {
        id: fee_schedule.id,
        name: fee_schedule.name,
        version: fee_schedule.version,
        created_at: fee_schedule.created_at,
        ranges: fee_schedule_ranges,
    }))
}

pub fn add_fee_schedule(
    (connection, parameters, json, user): (
        Connection,
        Path<PathParameters>,
        Json<NewFeeSchedule>,
        User,
    ),
) -> Result<HttpResponse, BigNeonError> {
    user.requires_scope(Scopes::OrgAdmin)?;
    let connection = connection.get();

    let fee_schedule = json.into_inner().commit(connection)?;
    let fee_schedule_ranges = fee_schedule.ranges(connection)?;

    Organization::find(parameters.id, connection)?.add_fee_schedule(&fee_schedule, connection)?;

    Ok(HttpResponse::Created().json(FeeScheduleWithRanges {
        id: fee_schedule.id,
        name: fee_schedule.name,
        version: fee_schedule.version,
        created_at: fee_schedule.created_at,
        ranges: fee_schedule_ranges,
    }))
}

pub fn search_fans(
    (connection, path, query, user): (
        Connection,
        Path<PathParameters>,
        Query<PagingParameters>,
        User,
    ),
) -> Result<WebPayload<DisplayFan>, BigNeonError> {
    let connection = connection.get();
    let org = Organization::find(path.id, connection)?;
    user.requires_scope_for_organization(Scopes::OrgReadFans, &org, &connection)?;
    let payload = org.search_fans(
        query.get_tag("query"),
        query.page(),
        query.limit(),
        query
            .sort
            .as_ref()
            .map(|s| s.parse().unwrap_or(FanSortField::LastOrder))
            .unwrap_or(FanSortField::LastOrder),
        query.dir.unwrap_or(SortingDir::Desc),
        connection,
    )?;
    Ok(WebPayload::new(StatusCode::OK, payload))
}
