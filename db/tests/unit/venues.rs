use bigneon_db::dev::TestProject;
use bigneon_db::prelude::*;
use bigneon_db::utils::errors::ErrorCode::ValidationError;

#[test]
fn commit() {
    let project = TestProject::new();
    let name = "Name";
    let venue = Venue::create(name.clone(), None, None, "America/Los_Angeles".into())
        .commit(project.get_connection())
        .unwrap();

    assert_eq!(venue.name, name);
    assert_eq!(venue.id.to_string().is_empty(), false);
}

#[test]
fn new_venue_with_validation_errors() {
    let project = TestProject::new();
    let name = "Name";
    let venue = Venue::create(name.clone(), None, None, "".into());
    let result = venue.commit(project.get_connection());
    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("timezone"));
                assert_eq!(errors["timezone"].len(), 1);
                assert_eq!(errors["timezone"][0].code, "length");
            }
            _ => panic!("Expected validation error"),
        },
    }
}

#[test]
fn update_with_validation_errors() {
    let project = TestProject::new();
    let venue = project.create_venue().finish();

    let parameters = VenueEditableAttributes {
        timezone: Some("".to_string()),
        ..Default::default()
    };

    let result = venue.update(parameters, project.get_connection());
    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("timezone"));
                assert_eq!(errors["timezone"].len(), 1);
                assert_eq!(errors["timezone"][0].code, "length");
            }
            _ => panic!("Expected validation error"),
        },
    }
}

#[test]
fn update() {
    let project = TestProject::new();
    let venue = project.create_venue().finish();

    let new_name = "New Venue Name";
    let new_address = "Test Address";

    let parameters = VenueEditableAttributes {
        name: Some(new_name.to_string()),
        address: Some(new_address.to_string()),
        ..Default::default()
    };

    let updated_venue = venue.update(parameters, project.get_connection()).unwrap();
    assert_eq!(updated_venue.name, new_name);
    assert_eq!(updated_venue.address, new_address);
}

#[test]
fn set_privacy() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let mut venue = project.create_venue().finish();
    assert!(!venue.is_private);

    venue = venue.set_privacy(true, connection).unwrap();
    assert!(venue.is_private);

    venue = venue.set_privacy(false, connection).unwrap();
    assert!(!venue.is_private);
}

#[test]
fn find() {
    let project = TestProject::new();
    let venue = project.create_venue().finish();

    let found_venue = Venue::find(venue.id, project.get_connection()).unwrap();
    assert_eq!(venue, found_venue);
}

#[test]
fn find_by_ids() {
    let project = TestProject::new();
    let venue = project
        .create_venue()
        .with_name("Venue1".to_string())
        .finish();
    let _ = project
        .create_venue()
        .with_name("Venue2".to_string())
        .finish();
    let venue3 = project
        .create_venue()
        .with_name("Venue3".to_string())
        .finish();

    let mut expected_venues = vec![venue.clone(), venue3.clone()];
    expected_venues.sort_by_key(|v| v.id);

    let found_venues =
        Venue::find_by_ids(vec![venue.id, venue3.id], &project.get_connection()).unwrap();
    assert_eq!(expected_venues, found_venues);
}

#[test]
fn all() {
    let project = TestProject::new();
    let venue = project
        .create_venue()
        .with_name("Venue1".to_string())
        .finish();
    let venue2 = project
        .create_venue()
        .with_name("Venue2".to_string())
        .finish();
    let organization = project.create_organization().finish();
    let conn = &project.get_connection();

    let all_found_venues = Venue::all(None, conn).unwrap();
    let mut all_venues = vec![venue, venue2];
    assert_eq!(all_venues, all_found_venues);

    let venue3 = project
        .create_venue()
        .with_name("Venue3".to_string())
        .make_private()
        .finish();
    let venue3 = venue3.add_to_organization(&organization.id, conn);
    let user = project.create_user().finish();
    let _org_user = organization
        .add_user(user.id, vec![Roles::OrgMember], Vec::new(), conn)
        .unwrap();
    all_venues.push(venue3.unwrap());
    let all_found_venues = Venue::all(Some(&user), conn).unwrap();
    assert_eq!(all_venues, all_found_venues);
    let all_found_venues = Venue::all(Some(&User::find(user.id, conn).unwrap()), conn).unwrap();
    assert_eq!(all_venues, all_found_venues);
}

#[test]
fn find_for_organization() {
    let project = TestProject::new();
    let owner = project.create_user().finish();
    let member = project.create_user().finish();
    let user = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_member(&owner, Roles::OrgOwner)
        .with_member(&member, Roles::OrgMember)
        .finish();
    let venue1 = project
        .create_venue()
        .with_name("Venue1".to_string())
        .with_organization(&organization)
        .finish();

    let venue2 = project
        .create_venue()
        .with_name("Venue2".to_string())
        .with_organization(&organization)
        .finish();

    let venue3 = project
        .create_venue()
        .with_name("Venue3".to_string())
        .with_organization(&organization)
        .make_private()
        .finish();

    // Add another venue for another org to make sure it isn't included
    let organization2 = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .finish();
    let venue4 = project
        .create_venue()
        .with_name("Venue4".to_string())
        .with_organization(&organization2)
        .finish();

    let user = project.create_user().finish();

    let mut all_venues = vec![venue1, venue2];

    let found_venues =
        Venue::find_for_organization(None, organization.id, project.get_connection()).unwrap();
    assert_eq!(found_venues, all_venues);

    let found_venues =
        Venue::find_for_organization(None, organization.id, project.get_connection()).unwrap();
    assert_eq!(found_venues, all_venues);
    assert!(!found_venues.contains(&venue3));
    assert!(!found_venues.contains(&venue4));

    // Private venue is not shown for users
    let found_venues =
        Venue::find_for_organization(Some(user.id), organization.id, project.get_connection())
            .unwrap();
    assert_eq!(found_venues, all_venues);

    // Private venue is shown for owners and members
    all_venues.push(venue3);
    let found_venues =
        Venue::find_for_organization(Some(owner.id), organization.id, project.get_connection())
            .unwrap();
    assert_eq!(found_venues, all_venues);

    let found_venues =
        Venue::find_for_organization(Some(member.id), organization.id, project.get_connection())
            .unwrap();
    assert_eq!(found_venues, all_venues);
}

#[test]
fn organization() {
    let project = TestProject::new();
    let organization = project.create_organization().finish();
    let venue = project
        .create_venue()
        .with_organization(&organization)
        .finish();
    let venue2 = project.create_venue().finish();

    assert_eq!(
        Ok(Some(organization)),
        venue.organization(project.get_connection())
    );
    assert_eq!(Ok(None), venue2.organization(project.get_connection()));
}
