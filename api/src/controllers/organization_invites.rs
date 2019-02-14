use actix_web::{http::StatusCode, HttpResponse, Path, Query, State};
use auth::user::User as AuthUser;
use bigneon_db::models::*;
use bigneon_db::utils::errors::DatabaseError;
use bigneon_db::utils::errors::Optional;
use communications::mailers;
use db::Connection;
use errors::*;
use extractors::*;
use helpers::application;
use models::{OrganizationInvitePathParameters, PathParameters, WebPayload};
use server::AppState;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct Info {
    pub token: Uuid,
    pub user_id: Uuid,
}

#[derive(Deserialize)]
pub struct InviteResponseQuery {
    pub security_token: Uuid,
}

#[derive(Deserialize)]
pub struct NewOrgInviteRequest {
    pub user_email: String,
    pub roles: Vec<Roles>,
    pub event_ids: Option<Vec<Uuid>>,
}

pub fn create(
    (state, connection, new_org_invite, path, auth_user): (
        State<AppState>,
        Connection,
        Json<NewOrgInviteRequest>,
        Path<PathParameters>,
        AuthUser,
    ),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let organization = Organization::find(path.id, connection)?;
    for role in &new_org_invite.roles {
        match role {
            &Roles::OrgOwner => auth_user.requires_scope_for_organization(
                Scopes::OrgAdmin,
                &organization,
                connection,
            )?,
            &Roles::OrgAdmin => auth_user.requires_scope_for_organization(
                Scopes::OrgAdminUsers,
                &organization,
                connection,
            )?,
            &Roles::OrgMember => auth_user.requires_scope_for_organization(
                Scopes::OrgUsers,
                &organization,
                connection,
            )?,
            &Roles::DoorPerson => auth_user.requires_scope_for_organization(
                Scopes::OrgUsers,
                &organization,
                connection,
            )?,
            &Roles::OrgBoxOffice => auth_user.requires_scope_for_organization(
                Scopes::OrgUsers,
                &organization,
                connection,
            )?,
            &Roles::Promoter => auth_user.requires_scope_for_organization(
                Scopes::OrgUsers,
                &organization,
                connection,
            )?,
            &Roles::PromoterReadOnly => auth_user.requires_scope_for_organization(
                Scopes::OrgUsers,
                &organization,
                connection,
            )?,
            _ => DatabaseError::validation_error(
                "role",
                "Role must be either OrgOwner or OrgMember",
            )?,
        }
    }

    let mut invite: NewOrganizationInvite;
    let recipient: String;
    let user_id: Option<Uuid>;

    match User::find_by_email(&new_org_invite.user_email, connection).optional() {
        Ok(maybe_a_user) => match maybe_a_user {
            Some(user) => {
                recipient = user.full_name();
                user_id = Some(user.id);
            }
            None => {
                recipient = "New user".to_string();
                user_id = None;
            }
        },
        Err(e) => return Err(e.into()),
    };

    //If an active invite exists for this email then first expire it before issuing the new invite.
    if let Some(mut i) =
        OrganizationInvite::find_active_invite_by_email(&new_org_invite.user_email, connection)?
    {
        i.change_invite_status(0, connection)?;
    }

    invite = OrganizationInvite::create(
        path.id,
        auth_user.id(),
        new_org_invite.user_email.as_str(),
        user_id,
        new_org_invite.roles.clone(),
        new_org_invite.event_ids.clone(),
    );

    let invite = invite.commit(connection)?;
    let organization = Organization::find(invite.organization_id, connection)?;

    mailers::organization_invites::invite_user_to_organization_email(
        &state.config,
        &invite,
        &organization,
        &recipient,
        connection,
    )?;
    Ok(HttpResponse::Created().json(invite))
}

pub fn index(
    (connection, path, query_parameters, auth_user): (
        Connection,
        Path<PathParameters>,
        Query<PagingParameters>,
        AuthUser,
    ),
) -> Result<WebPayload<DisplayInvite>, BigNeonError> {
    let connection = connection.get();
    let organization = Organization::find(path.id, connection)?;
    auth_user.requires_scope_for_organization(Scopes::OrgUsers, &organization, connection)?;

    let payload = OrganizationInvite::find_pending_by_organization_paged(
        path.id,
        query_parameters.page(),
        query_parameters.limit(),
        connection,
    )?;

    Ok(WebPayload::new(StatusCode::OK, payload))
}

pub fn destroy(
    (connection, path, auth_user): (Connection, Path<OrganizationInvitePathParameters>, AuthUser),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let invite = OrganizationInvite::find(path.invite_id, connection)?;
    let organization = invite.organization(connection)?;

    // Level of access dependent on scope of the invited member
    for role in &invite.roles {
        match role {
            &Roles::OrgOwner => auth_user.requires_scope_for_organization(
                Scopes::OrgAdmin,
                &organization,
                connection,
            )?,
            &Roles::OrgAdmin => auth_user.requires_scope_for_organization(
                Scopes::OrgAdminUsers,
                &organization,
                connection,
            )?,
            &Roles::OrgMember => auth_user.requires_scope_for_organization(
                Scopes::OrgUsers,
                &organization,
                connection,
            )?,
            &Roles::DoorPerson => auth_user.requires_scope_for_organization(
                Scopes::OrgUsers,
                &organization,
                connection,
            )?,
            &Roles::OrgBoxOffice => auth_user.requires_scope_for_organization(
                Scopes::OrgUsers,
                &organization,
                connection,
            )?,
            &Roles::Promoter => auth_user.requires_scope_for_organization(
                Scopes::OrgUsers,
                &organization,
                connection,
            )?,
            &Roles::PromoterReadOnly => auth_user.requires_scope_for_organization(
                Scopes::OrgUsers,
                &organization,
                connection,
            )?,
            // Should not happen but if it ever did allow admin to destroy record
            _ => auth_user.requires_scope_for_organization(
                Scopes::OrgAdmin,
                &organization,
                connection,
            )?,
        }
    }

    invite.destroy(connection)?;
    Ok(HttpResponse::Ok().json(json!({})))
}

pub fn view(
    (connection, path): (Connection, Path<PathParameters>),
) -> Result<HttpResponse, BigNeonError> {
    // TODO: Change /{id} to /?token={} in routing and client apps.
    // Until then, just remember that the id passed in is actually the token
    let connection = connection.get();
    let invite_details = OrganizationInvite::get_invite_display(&path.id, connection)?;
    Ok(HttpResponse::Ok().json(json!(invite_details)))
}

pub fn accept_request(
    (connection, query, user): (Connection, Query<InviteResponseQuery>, OptionalUser),
) -> Result<HttpResponse, BigNeonError> {
    let query_struct = query.into_inner();
    let connection = connection.get();
    let mut invite_details =
        OrganizationInvite::find_by_token(query_struct.security_token, connection)?;
    //Check that the user is logged in, that if the invite has a user_id associated with it that it is the currently logged in user
    match user.into_inner() {
        Some(u) => {
            let valid_for_acceptance = match invite_details.user_id {
                // If the user_id was provided confirm that the current user is the accepting user
                Some(user_id) => user_id == u.id(),
                None => {
                    // If not confirm that the current user has an email set and that it matches the invite
                    if let Some(email) = u.email() {
                        invite_details.user_email == email
                    } else {
                        false
                    }
                }
            };

            if valid_for_acceptance {
                invite_details.change_invite_status(1, connection)?;
                let org = Organization::find(invite_details.organization_id, connection)?;
                org.add_user(
                    u.id(),
                    invite_details.roles,
                    invite_details.event_ids,
                    connection,
                )?;
            } else {
                return application::unauthorized(Some(u), None);
            }
        }
        None => return application::unauthorized(None, None),
    }
    Ok(HttpResponse::Ok().finish())
}
