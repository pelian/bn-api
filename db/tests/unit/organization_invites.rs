use bigneon_db::dev::TestProject;
use bigneon_db::prelude::*;
use bigneon_db::utils::errors::ErrorCode::ValidationError;
use chrono::prelude::*;
use chrono::Duration;
use diesel;
use diesel::prelude::*;

#[test]
fn create() {
    let project = TestProject::new();
    let user = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .finish();
    let org_invite = project
        .create_organization_invite()
        .with_org(&organization)
        .with_invitee(&user)
        .finish();

    assert_eq!(org_invite.organization_id, organization.id);
    assert_eq!(org_invite.inviter_id, user.id);
}

#[test]
fn find_pending_by_org() {
    let project = TestProject::new();
    let user = project.create_user().finish();
    let connection = project.get_connection();
    let organization = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .finish();
    let _org_invite = project
        .create_organization_invite()
        .with_org(&organization)
        .with_invitee(&user)
        .finish();
    let pending =
        OrganizationInvite::find_pending_by_organization(organization.id, None, connection)
            .unwrap();
    assert_eq!(pending.len(), 1);
    let email = "test@test.com".to_string();
    let active_find = OrganizationInvite::find_active_invite_by_email(&email, connection).unwrap();
    assert_eq!(active_find.is_some(), true);
}

#[test]
fn create_event_limited_access_user() {
    let project = TestProject::new();
    let user = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .finish();
    let organization_invite = OrganizationInvite::create(
        organization.id,
        user.id,
        &"valid@test.com".to_string(),
        Some(user.id),
        vec![Roles::Promoter],
        Some(vec![event.id]),
    )
    .commit(project.get_connection())
    .unwrap();

    assert_eq!(vec![event.id], organization_invite.event_ids);
    assert_eq!(vec![Roles::Promoter], organization_invite.roles);
}

#[test]
fn create_with_validation_errors() {
    let project = TestProject::new();
    let user = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .finish();
    let other_organization_event = project.create_event().finish();
    let result = OrganizationInvite::create(
        organization.id,
        user.id,
        &"invalid-email".to_string(),
        Some(user.id),
        vec![Roles::Promoter],
        Some(vec![other_organization_event.id]),
    )
    .commit(project.get_connection());

    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("user_email"));
                assert_eq!(errors["user_email"].len(), 1);
                assert_eq!(errors["user_email"][0].code, "email");
                assert_eq!(
                    &errors["user_email"][0]
                        .message
                        .clone()
                        .unwrap()
                        .into_owned(),
                    "User email is invalid"
                );

                assert!(errors.contains_key("event_ids"));
                assert_eq!(errors["event_ids"].len(), 1);
                assert_eq!(
                    errors["event_ids"][0].code,
                    "event_ids_do_not_belong_to_organization"
                );
                assert_eq!(
                    &errors["event_ids"][0].message.clone().unwrap().into_owned(),
                    "Event ids invalid for organization user"
                );
            }
            _ => panic!("Expected validation error"),
        },
    }
}

#[test]
fn change_invite_status_of_invite() {
    let project = TestProject::new();
    let user = project.create_user().finish();
    let user2 = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .finish();
    let mut org_invite = project
        .create_organization_invite()
        .with_org(&organization)
        .with_invitee(&user)
        .link_to_user(&user2)
        .finish();
    let mut org_invite2 = project
        .create_organization_invite()
        .with_org(&organization)
        .with_invitee(&user)
        .link_to_user(&user2)
        .finish();
    /*making the assumption that it wont take more than 60 seconds to update the status
    we cant test for an exact date, as this will depend on the database write delay
    we will test for a period of 30 seconds
    */
    assert!(org_invite.accept_invite(&project.get_connection()).is_ok());
    assert!(org_invite2
        .decline_invite(&project.get_connection())
        .is_ok());

    assert_eq!(org_invite.accepted, Some(1));
    assert_eq!(org_invite.security_token, None);

    assert_eq!(org_invite2.accepted, Some(0));
    assert_eq!(org_invite2.security_token, None);
}

#[test]
fn view_invitation() {
    let project = TestProject::new();
    let user = project.create_user().finish();
    let inviter = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .finish();
    let org_invite = project
        .create_organization_invite()
        .with_org(&organization)
        .with_invitee(&user)
        .with_inviter(&inviter)
        .finish();
    let display_invite = OrganizationInvite::get_invite_display(
        &org_invite.security_token.unwrap(),
        project.get_connection(),
    )
    .unwrap();

    assert_eq!(display_invite.organization_name, organization.name);
    assert_eq!(
        display_invite.inviter_name,
        format!(
            "{} {}",
            inviter.first_name.unwrap_or("".to_string()),
            inviter.last_name.unwrap_or("".to_string())
        )
    );
}
#[test]
fn test_token_validity() {
    let project = TestProject::new();
    let user = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .finish();
    let mut org_invite = project
        .create_organization_invite()
        .with_org(&organization)
        .with_invitee(&user)
        .finish();
    let recovered_invite = OrganizationInvite::find_by_token(
        org_invite.security_token.unwrap(),
        project.get_connection(),
    )
    .unwrap();
    assert_eq!(org_invite, recovered_invite);
    org_invite.created_at = NaiveDate::from_ymd(2016, 7, 8).and_hms(9, 10, 11);
    org_invite = update(&org_invite, &project.get_connection()).unwrap();
    let recovered_invite2 = OrganizationInvite::find_by_token(
        org_invite.security_token.unwrap(),
        &project.get_connection(),
    );
    let error_value = DatabaseError {
        code: 2000,
        message: "No results".into(),
        cause: Some("No valid token found, NotFound".into()),
        error_code: ErrorCode::NoResults,
    };
    match recovered_invite2 {
        Ok(_val) => assert_eq!(true, false), //this should not happen, so this should fail
        Err(val) => assert_eq!(error_value, val),
    }
}

#[test]
fn find() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user1 = project.create_user().finish();
    let user2 = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_member(&user1, Roles::OrgOwner)
        .finish();
    let invite = project
        .create_organization_invite()
        .with_org(&organization)
        .with_inviter(&user1)
        .with_invitee(&user2)
        .finish();
    let result = OrganizationInvite::find(invite.id, connection);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), invite);
}

#[test]
fn destroy() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user1 = project.create_user().finish();
    let user2 = project.create_user().finish();
    let user3 = project.create_user().finish();
    let user4 = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_member(&user1, Roles::OrgOwner)
        .finish();

    let mut org_invite1 = project
        .create_organization_invite()
        .with_org(&organization)
        .with_inviter(&user1)
        .with_invitee(&user2)
        .finish();

    let mut org_invite2 = project
        .create_organization_invite()
        .with_org(&organization)
        .with_inviter(&user1)
        .with_invitee(&user3)
        .finish();

    let org_invite3 = project
        .create_organization_invite()
        .with_org(&organization)
        .with_inviter(&user1)
        .with_invitee(&user4)
        .finish();

    // Decline first invite
    org_invite1.change_invite_status(0, connection).unwrap();

    // Accept second invite
    org_invite2.change_invite_status(1, connection).unwrap();

    let result = org_invite1.destroy(connection);
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().cause,
        Some("Cannot destroy invite it has already been declined.".into())
    );

    let result = org_invite2.destroy(connection);
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().cause,
        Some("Cannot destroy invite it has already been accepted.".into())
    );

    let result = org_invite3.destroy(connection);
    assert!(result.is_ok());
}

#[test]
fn organization() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user1 = project.create_user().finish();
    let user2 = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_member(&user1, Roles::OrgOwner)
        .finish();

    let org_invite = project
        .create_organization_invite()
        .with_org(&organization)
        .with_inviter(&user1)
        .with_invitee(&user2)
        .finish();

    assert_eq!(organization, org_invite.organization(connection).unwrap());
}

#[test]
fn find_pending_by_organization_paged() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user1 = project.create_user().finish();
    let user2 = project.create_user().finish();
    let user3 = project.create_user().finish();
    let user4 = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_member(&user1, Roles::OrgOwner)
        .finish();

    let mut org_invite1 = project
        .create_organization_invite()
        .with_org(&organization)
        .with_inviter(&user1)
        .with_invitee(&user2)
        .finish();

    let mut org_invite2 = project
        .create_organization_invite()
        .with_org(&organization)
        .with_inviter(&user1)
        .with_invitee(&user3)
        .finish();

    let org_invite3 = project
        .create_organization_invite()
        .with_org(&organization)
        .with_inviter(&user1)
        .with_invitee(&user4)
        .finish();

    // Decline first invite
    org_invite1.change_invite_status(0, connection).unwrap();

    // Accept second invite
    org_invite2.change_invite_status(1, connection).unwrap();

    let paged_invites =
        OrganizationInvite::find_pending_by_organization_paged(organization.id, 0, 100, connection)
            .unwrap();
    assert_eq!(
        vec![DisplayInvite {
            id: org_invite3.id,
            organization_name: organization.name,
            inviter_name: format!("{} {}", user4.first_name.unwrap(), user4.last_name.unwrap())
                .into(),
            expires_at: org_invite3.created_at + Duration::days(INVITE_EXPIRATION_PERIOD_IN_DAYS),
        }],
        paged_invites.data
    );
    assert_eq!(1, paged_invites.paging.total);
}

#[test]
fn test_sending_status() {
    let project = TestProject::new();
    let user = project.create_user().finish();
    let user2 = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .finish();
    let org_invite = project
        .create_organization_invite()
        .with_org(&organization)
        .with_invitee(&user)
        .link_to_user(&user2)
        .finish();
    /*making the assumption that it wont take more than 60 seconds to update the status
    we cant test for an exact date, as this will depend on the database write delay
    we will test for a period of 30 seconds
    */
    let pre_send_invite = org_invite.clone();
    let post_send_invite = org_invite
        .change_sent_status(true, &project.get_connection())
        .unwrap();

    assert_eq!(pre_send_invite.sent_invite, false);
    assert_eq!(post_send_invite.sent_invite, true);
}

// dont want to update the details in the main function, so keeping this in the unit test section
fn update(
    org_invite: &OrganizationInvite,
    conn: &PgConnection,
) -> Result<OrganizationInvite, DatabaseError> {
    DatabaseError::wrap(
        ErrorCode::UpdateError,
        "Could not update organization_invite",
        diesel::update(org_invite).set(org_invite).get_result(conn),
    )
}
