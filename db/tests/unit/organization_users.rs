use bigneon_db::dev::TestProject;
use bigneon_db::models::{OrganizationUser, Roles};
use bigneon_db::utils::errors::ErrorCode::ValidationError;

#[test]
fn create() {
    let project = TestProject::new();
    let user = project.create_user().finish();
    let user2 = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .finish();
    let organization_user = OrganizationUser::create(
        organization.id,
        user2.id,
        vec![Roles::OrgMember],
        Vec::new(),
    )
    .commit(project.get_connection())
    .unwrap();

    assert_eq!(organization_user.user_id, user2.id);
    assert_eq!(organization_user.organization_id, organization.id);
    assert_eq!(organization_user.role, [Roles::OrgMember]);
    assert_eq!(organization_user.id.to_string().is_empty(), false);
}

#[test]
fn create_event_limited_access_user() {
    let project = TestProject::new();
    let user = project.create_user().finish();
    let organization = project.create_organization().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .finish();
    let organization_user = OrganizationUser::create(
        organization.id,
        user.id,
        vec![Roles::Promoter],
        vec![event.id],
    )
    .commit(project.get_connection())
    .unwrap();

    assert_eq!(vec![event.id], organization_user.event_ids);
    assert_eq!(vec![Roles::Promoter], organization_user.role);
}

#[test]
fn create_with_validation_errors() {
    let project = TestProject::new();
    let user = project.create_user().finish();
    let organization = project.create_organization().finish();
    let other_organization_event = project.create_event().finish();
    let result = OrganizationUser::create(
        organization.id,
        user.id,
        vec![Roles::Promoter],
        vec![other_organization_event.id],
    )
    .commit(project.get_connection());

    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
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
fn update() {
    let project = TestProject::new();
    let user = project.create_user().finish();
    let organization = project.create_organization().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .finish();
    let organization_user =
        OrganizationUser::create(organization.id, user.id, vec![Roles::OrgMember], Vec::new())
            .commit(project.get_connection())
            .unwrap();
    let organization_user_id = organization_user.id;

    let organization_user = OrganizationUser::create(
        organization.id,
        user.id,
        vec![Roles::Promoter],
        vec![event.id],
    )
    .commit(project.get_connection())
    .unwrap();

    assert_eq!(vec![event.id], organization_user.event_ids);
    assert_eq!(vec![Roles::Promoter], organization_user.role);
    assert_eq!(organization_user_id, organization_user.id);
}

#[test]
fn update_with_validation_errors() {
    let project = TestProject::new();
    let user = project.create_user().finish();
    let organization = project.create_organization().finish();
    let other_organization_event = project.create_event().finish();
    OrganizationUser::create(organization.id, user.id, vec![Roles::OrgMember], Vec::new())
        .commit(project.get_connection())
        .unwrap();

    let result = OrganizationUser::create(
        organization.id,
        user.id,
        vec![Roles::Promoter],
        vec![other_organization_event.id],
    )
    .commit(project.get_connection());

    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
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
