use bigneon_db::dev::TestProject;
use bigneon_db::prelude::*;
use uuid::Uuid;

#[test]
fn create() {
    let project = TestProject::new();
    let creator = project.create_user().finish();

    let connection = project.get_connection();
    let fee_schedule = FeeSchedule::create(
        Uuid::nil(),
        format!("Zero fees",).into(),
        vec![NewFeeScheduleRange {
            min_price_in_cents: 0,
            company_fee_in_cents: 0,
            client_fee_in_cents: 0,
        }],
    )
    .commit(creator.id, connection)
    .unwrap();

    let updated_fee_schedule = FeeSchedule::find(fee_schedule.id, connection).unwrap();
    assert_eq!(updated_fee_schedule.organization_id, Uuid::nil());

    let mut organization = Organization::create("Organization", fee_schedule.id);
    organization.sendgrid_api_key = Some("A_Test_Key".to_string());

    let mut organization = organization
        .commit(&"encryption_key".to_string(), creator.id, connection)
        .unwrap();
    assert_eq!(organization.id.to_string().is_empty(), false);

    assert_ne!(
        organization.sendgrid_api_key,
        Some("A_Test_Key".to_string())
    );
    organization.decrypt(&"encryption_key".to_string()).unwrap();
    assert_eq!(
        organization.sendgrid_api_key,
        Some("A_Test_Key".to_string())
    );

    let updated_fee_schedule = FeeSchedule::find(fee_schedule.id, connection).unwrap();

    assert_eq!(updated_fee_schedule.organization_id, organization.id);
}

#[test]
fn find_by_asset_id() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project
        .create_event()
        .with_ticket_pricing()
        .with_ticket_type_count(2)
        .finish();
    let event2 = project
        .create_event()
        .with_ticket_pricing()
        .with_ticket_type_count(1)
        .finish();
    let ticket_type = &event.ticket_types(true, None, &connection).unwrap()[0];
    let ticket_type2 = &event2.ticket_types(true, None, &connection).unwrap()[0];
    let asset = Asset::find_by_ticket_type(ticket_type.id, connection).unwrap();
    let asset2 = Asset::find_by_ticket_type(ticket_type2.id, connection).unwrap();
    assert_eq!(
        Organization::find_by_asset_id(asset.id, connection).unwrap(),
        event.organization(connection).unwrap()
    );
    assert_eq!(
        Organization::find_by_asset_id(asset2.id, connection).unwrap(),
        event2.organization(connection).unwrap()
    );
}

#[test]
fn find_by_ticket_type_ids() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project
        .create_event()
        .with_ticket_pricing()
        .with_ticket_type_count(2)
        .finish();
    let event2 = project
        .create_event()
        .with_ticket_pricing()
        .with_ticket_type_count(1)
        .finish();
    let ticket_types = event.ticket_types(true, None, &connection).unwrap();
    let ticket_type = &ticket_types[0];
    let ticket_type2 = &ticket_types[1];
    let ticket_types = event2.ticket_types(true, None, &connection).unwrap();
    let ticket_type3 = &ticket_types[0];

    let organizations = Organization::find_by_ticket_type_ids(
        vec![ticket_type.id, ticket_type2.id, ticket_type3.id],
        connection,
    )
    .unwrap();
    assert_eq!(2, organizations.len());
    assert!(organizations.contains(&event.organization(connection).unwrap()));
    assert!(organizations.contains(&event2.organization(connection).unwrap()));
}

#[test]
fn find_by_order_item_ids() {
    let project = TestProject::new();
    let creator = project.create_user().finish();
    let connection = project.get_connection();
    let organization = project
        .create_organization()
        .with_name("Organization1".into())
        .with_fee_schedule(&project.create_fee_schedule().finish(creator.id))
        .finish();
    let organization2 = project
        .create_organization()
        .with_name("Organization2".into())
        .with_fee_schedule(&project.create_fee_schedule().finish(creator.id))
        .finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let event2 = project
        .create_event()
        .with_organization(&organization2)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
    let ticket_type2 = &event2.ticket_types(true, None, connection).unwrap()[0];
    cart.update_quantities(
        user.id,
        &[
            UpdateOrderItem {
                ticket_type_id: ticket_type.id,
                quantity: 10,
                redemption_code: None,
            },
            UpdateOrderItem {
                ticket_type_id: ticket_type2.id,
                quantity: 10,
                redemption_code: None,
            },
        ],
        false,
        false,
        connection,
    )
    .unwrap();

    let items = cart.items(&connection).unwrap();
    let order_item = items
        .iter()
        .find(|i| i.ticket_type_id == Some(ticket_type.id))
        .unwrap();
    let order_item2 = items
        .iter()
        .find(|i| i.ticket_type_id == Some(ticket_type2.id))
        .unwrap();

    // Ticket belonging to only first event / organization
    let organizations =
        Organization::find_by_order_item_ids(&vec![order_item.id], connection).unwrap();
    assert_eq!(organizations, vec![organization.clone()]);

    // Ticket belonging to only second event / organization
    let organizations =
        Organization::find_by_order_item_ids(&vec![order_item2.id], connection).unwrap();
    assert_eq!(organizations, vec![organization2.clone()]);

    // Ticket belonging to both events / organizations
    let organizations =
        Organization::find_by_order_item_ids(&vec![order_item.id, order_item2.id], connection)
            .unwrap();
    assert_eq!(organizations, vec![organization, organization2]);
}

#[test]
fn has_fan() {
    let project = TestProject::new();
    let creator = project.create_user().finish();

    let connection = project.get_connection();
    let user = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_fee_schedule(&project.create_fee_schedule().finish(creator.id))
        .finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];

    // No relationship
    assert!(!organization.has_fan(&user, connection).unwrap());

    // User adds item to cart but does not checkout so no relationship
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 10,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();
    assert!(!organization.has_fan(&user, connection).unwrap());

    // User checks out so has a paid order so relationship exists
    assert_eq!(cart.calculate_total(connection).unwrap(), 1700);
    cart.add_external_payment(Some("test".to_string()), user.id, 1700, connection)
        .unwrap();
    assert_eq!(cart.status, OrderStatus::Paid);
    assert!(organization.has_fan(&user, connection).unwrap());
}

#[test]
fn update() {
    let project = TestProject::new();
    let user = project.create_user().finish();
    //Edit Organization
    let mut edited_organization = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .finish();

    edited_organization.name = "Test Org".to_string();
    edited_organization.address = Some("Test Address".to_string());
    edited_organization.city = Some("Test Address".to_string());
    edited_organization.state = Some("Test state".to_string());
    edited_organization.country = Some("Test country".to_string());
    edited_organization.postal_code = Some("0124".to_string());
    edited_organization.phone = Some("+27123456789".to_string());
    edited_organization.sendgrid_api_key = Some("A_Test_Key".to_string());

    let mut changed_attrs: OrganizationEditableAttributes = Default::default();
    changed_attrs.name = Some("Test Org".to_string());
    changed_attrs.address = Some("Test Address".to_string());
    changed_attrs.city = Some("Test Address".to_string());
    changed_attrs.state = Some("Test state".to_string());
    changed_attrs.country = Some("Test country".to_string());
    changed_attrs.postal_code = Some("0124".to_string());
    changed_attrs.phone = Some("+27123456789".to_string());
    changed_attrs.sendgrid_api_key = Some(Some("A_Test_Key".to_string()));
    let mut updated_organization = Organization::update(
        &edited_organization,
        changed_attrs,
        &"encryption_key".to_string(),
        project.get_connection(),
    )
    .unwrap();

    updated_organization
        .decrypt(&"encryption_key".to_string())
        .unwrap();

    assert_eq!(edited_organization, updated_organization);
}

#[test]
fn find() {
    let project = TestProject::new();
    let user = project.create_user().finish();
    //Edit Organization
    let organization = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .with_address()
        .finish();
    let found_organization = Organization::find(organization.id, project.get_connection()).unwrap();
    assert_eq!(organization, found_organization);
}

#[test]
fn find_for_event() {
    let project = TestProject::new();
    let user = project.create_user().finish();
    //Edit Organization
    let organization = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .with_address()
        .finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .finish();
    let found_organization =
        Organization::find_for_event(event.id, project.get_connection()).unwrap();
    assert_eq!(organization, found_organization);
}

#[test]
fn users() {
    let project = TestProject::new();
    let user = project.create_user().with_last_name("User1").finish();
    let user2 = project.create_user().with_last_name("User2").finish();
    let user3 = project.create_user().with_last_name("User3").finish();
    let organization = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .finish();
    let organization2 = project
        .create_organization()
        .with_member(&user3, Roles::OrgOwner)
        .finish();
    OrganizationUser::create(
        organization2.id,
        user2.id,
        vec![Roles::OrgMember],
        Vec::new(),
    )
    .commit(project.get_connection())
    .unwrap();

    // Owner is included in the user results for organization2 but not organization2
    let user_results = organization.users(None, project.get_connection()).unwrap();
    assert_eq!(user_results.len(), 1);
    assert_eq!(user.id, user_results[0].1.id);

    let user_results = organization2.users(None, project.get_connection()).unwrap();
    assert_eq!(
        vec![user2.id, user3.id],
        user_results.iter().map(|u| u.1.id).collect::<Vec<Uuid>>()
    );

    // Explicitly make the organization user an org user
    OrganizationUser::create(organization.id, user.id, vec![Roles::OrgMember], Vec::new())
        .commit(project.get_connection())
        .unwrap();
    let user_results = organization.users(None, project.get_connection()).unwrap();
    assert_eq!(user_results.len(), 1);
    assert_eq!(user.id, user_results[0].1.id);
    let user_results2 = organization2.users(None, project.get_connection()).unwrap();
    assert_eq!(user_results2.len(), 2);
    assert_eq!(user2.id, user_results2[0].1.id);
    assert_eq!(user3.id, user_results2[1].1.id);

    // Add a new user to the organization
    OrganizationUser::create(
        organization.id,
        user2.id,
        vec![Roles::OrgMember],
        Vec::new(),
    )
    .commit(project.get_connection())
    .unwrap();
    let user_results = organization.users(None, project.get_connection()).unwrap();
    assert_eq!(user_results.len(), 2);
    assert_eq!(user.id, user_results[0].1.id);
    assert_eq!(user2.id, user_results[1].1.id);
    let user_results2 = organization2.users(None, project.get_connection()).unwrap();
    assert_eq!(user_results2.len(), 2);
    assert_eq!(user2.id, user_results2[0].1.id);
    assert_eq!(user3.id, user_results2[1].1.id);
}

#[test]
fn is_member() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let user2 = project.create_user().finish();
    let user3 = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .with_member(&user2, Roles::OrgMember)
        .finish();
    // Reload for owner role
    let user = User::find(user.id.clone(), connection).unwrap();

    assert!(organization.is_member(&user, connection).unwrap());
    assert!(organization.is_member(&user2, connection).unwrap());
    assert!(!organization.is_member(&user3, connection).unwrap());
}

#[test]
fn all_linked_to_user() {
    let project = TestProject::new();
    let user = project.create_user().finish();
    let user2 = project.create_user().finish();
    let org1 = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .finish();
    let org2 = project
        .create_organization()
        .with_member(&user2, Roles::OrgOwner)
        .with_member(&user, Roles::OrgMember)
        .finish();
    let _org3 = project
        .create_organization()
        .with_member(&user2, Roles::OrgOwner)
        .finish();

    let orgs = Organization::all_linked_to_user(user.id, project.get_connection()).unwrap();
    let mut test_vec = vec![org1, org2];
    test_vec.sort_by_key(|org| org.name.clone());
    assert_eq!(orgs, test_vec);
}

#[test]
fn all_org_names_linked_to_user() {
    let project = TestProject::new();
    let user1 = project.create_user().finish(); //Member and owner link
    let user2 = project.create_user().finish(); //Only owner link
    let user3 = project.create_user().finish(); //Only membership link
    let user4 = project.create_user().finish(); //No links
    let org1 = project
        .create_organization()
        .with_name(String::from("Test Org1"))
        .with_member(&user1, Roles::OrgOwner)
        .with_member(&user3, Roles::OrgMember)
        .finish();
    let org2 = project
        .create_organization()
        .with_name(String::from("Test Org2"))
        .with_member(&user2, Roles::OrgOwner)
        .with_member(&user1, Roles::OrgMember)
        .finish();
    let user1_links =
        Organization::all_org_names_linked_to_user(user1.id, project.get_connection()).unwrap();
    let user2_links =
        Organization::all_org_names_linked_to_user(user2.id, project.get_connection()).unwrap();
    let user3_links =
        Organization::all_org_names_linked_to_user(user3.id, project.get_connection()).unwrap();
    let user4_links =
        Organization::all_org_names_linked_to_user(user4.id, project.get_connection()).unwrap();

    //User1 has 2 links, owner of Org1 and member of Org2
    assert_eq!(user1_links.len(), 2);
    assert_eq!(user1_links[0].id, org1.id);
    assert_eq!(user1_links[1].id, org2.id);

    assert_eq!(user1_links[0].role, vec![Roles::OrgOwner]);
    assert_eq!(user1_links[1].role, vec![Roles::OrgMember]);

    //User2 has only 1 owner link with Org2
    assert_eq!(user2_links.len(), 1);
    assert_eq!(user2_links[0].id, org2.id);
    assert_eq!(user2_links[0].role, vec![Roles::OrgOwner]);
    //User3 has only 1 member link with Org1
    assert_eq!(user3_links.len(), 1);
    assert_eq!(user3_links[0].id, org1.id);
    assert_eq!(user3_links[0].role, vec![Roles::OrgMember]);
    //User4 has no links
    assert_eq!(user4_links.len(), 0);
}

#[test]
fn all() {
    let project = TestProject::new();
    let user = project.create_user().finish();
    let user2 = project.create_user().finish();
    let org1 = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .finish();
    let org2 = project
        .create_organization()
        .with_member(&user2, Roles::OrgOwner)
        .with_member(&user, Roles::OrgMember)
        .finish();
    let org3 = project
        .create_organization()
        .with_member(&user2, Roles::OrgOwner)
        .finish();

    let orgs = Organization::all(project.get_connection()).unwrap();
    let mut test_vec = vec![org1, org2, org3];
    test_vec.sort_by_key(|org| org.name.clone());
    assert_eq!(orgs, test_vec);
}

#[test]
fn remove_users() {
    let project = TestProject::new();
    let user = project.create_user().with_last_name("user1").finish();
    let user2 = project.create_user().with_last_name("user2").finish();
    let user3 = project.create_user().with_last_name("user3").finish();
    let organization = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .finish();
    OrganizationUser::create(
        organization.id,
        user2.id,
        vec![Roles::OrgMember],
        Vec::new(),
    )
    .commit(project.get_connection())
    .unwrap();
    OrganizationUser::create(
        organization.id,
        user3.id,
        vec![Roles::OrgMember],
        Vec::new(),
    )
    .commit(project.get_connection())
    .unwrap();
    let user2_id = user2.id;

    let mut user_results = organization.users(None, project.get_connection()).unwrap();

    user_results.sort_by_key(|k| k.1.last_name.clone());
    let mut users_before_delete = vec![user.clone(), user2, user3.clone()];
    users_before_delete.sort_by_key(|k| k.last_name.clone());

    assert_eq!(user_results[0].1, users_before_delete[0]);
    assert_eq!(user_results[1].1, users_before_delete[1]);
    assert_eq!(user_results[2].1, users_before_delete[2]);
    assert_eq!(user_results.len(), 3);

    //remove user
    let result = organization
        .remove_user(user2_id, project.get_connection())
        .unwrap();
    assert_eq!(result, 1);
    let user_results2: Vec<User> = organization
        .users(None, project.get_connection())
        .unwrap()
        .into_iter()
        .map(|u| u.1)
        .collect();
    let users_post_delete = vec![user, user3];

    assert_eq!(user_results2, users_post_delete);
}

#[test]
pub fn get_roles_for_user() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let user2 = project.create_user().finish();
    let mut user3 = project.create_user().finish();
    let user4 = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .with_member(&user2, Roles::OrgMember)
        .finish();
    user3 = user3.add_role(Roles::Admin, connection).unwrap();

    assert_eq!(
        organization.get_roles_for_user(&user, connection).unwrap(),
        vec![Roles::OrgOwner]
    );
    assert_eq!(
        organization.get_roles_for_user(&user2, connection).unwrap(),
        vec![Roles::OrgMember]
    );
    assert_eq!(
        organization.get_roles_for_user(&user3, connection).unwrap(),
        vec![Roles::OrgOwner]
    );
    assert!(organization
        .get_roles_for_user(&user4, connection)
        .unwrap()
        .is_empty());
}

#[test]
pub fn get_scopes_for_user() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let owner = project.create_user().finish();
    let org_admin = project.create_user().finish();
    let box_office = project.create_user().finish();
    let door_person = project.create_user().finish();
    let member = project.create_user().finish();
    let no_access_user = project.create_user().finish();
    let promoter = project.create_user().finish();
    let promoter_read_only = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_member(&owner, Roles::OrgOwner)
        .with_member(&member, Roles::OrgMember)
        .with_member(&org_admin, Roles::OrgAdmin)
        .with_member(&box_office, Roles::OrgBoxOffice)
        .with_member(&door_person, Roles::DoorPerson)
        .with_member(&promoter, Roles::Promoter)
        .with_member(&promoter_read_only, Roles::PromoterReadOnly)
        .finish();
    let mut admin = project.create_user().finish();
    admin = admin.add_role(Roles::Admin, connection).unwrap();

    assert_equiv!(
        organization
            .get_scopes_for_user(&owner, connection)
            .unwrap()
            .iter()
            .map(|i| i.to_string())
            .collect::<Vec<String>>(),
        vec![
            "artist:write",
            "box-office-ticket:read",
            "box-office-ticket:write",
            "code:read",
            "code:write",
            "comp:read",
            "comp:write",
            "dashboard:read",
            "event:cancel",
            "event:financial-reports",
            "event:interest",
            "event:reports",
            "event:scan",
            "event:view-guests",
            "event:write",
            "hold:read",
            "hold:write",
            "order:make-external-payment",
            "order:read",
            "order:read-own",
            "order:refund",
            "org:admin-users",
            "org:fans",
            "org:read",
            "org:read-events",
            "org:reports",
            "org:users",
            "org:write",
            "redeem:ticket",
            "ticket:admin",
            "ticket:read",
            "ticket:transfer",
            "ticket-type:read",
            "ticket-type:write",
            "user:read",
            "venue:write"
        ]
    );

    assert_equiv!(
        organization
            .get_scopes_for_user(&org_admin, connection)
            .unwrap()
            .iter()
            .map(|i| i.to_string())
            .collect::<Vec<String>>(),
        vec![
            "artist:write",
            "box-office-ticket:read",
            "box-office-ticket:write",
            "code:read",
            "code:write",
            "comp:read",
            "comp:write",
            "dashboard:read",
            "event:cancel",
            "event:financial-reports",
            "event:interest",
            "event:reports",
            "event:scan",
            "event:view-guests",
            "event:write",
            "hold:read",
            "hold:write",
            "order:make-external-payment",
            "order:read",
            "order:read-own",
            "order:refund",
            "org:fans",
            "org:read",
            "org:read-events",
            "org:reports",
            "org:users",
            "org:write",
            "redeem:ticket",
            "ticket:admin",
            "ticket:read",
            "ticket:transfer",
            "ticket-type:read",
            "ticket-type:write",
            "user:read",
            "venue:write"
        ]
    );

    assert_equiv!(
        organization
            .get_scopes_for_user(&box_office, connection)
            .unwrap()
            .iter()
            .map(|i| i.to_string())
            .collect::<Vec<String>>(),
        vec![
            "box-office-ticket:read",
            "dashboard:read",
            "event:scan",
            "event:view-guests",
            "hold:read",
            "order:make-external-payment",
            "org:read-events",
            "redeem:ticket",
            "ticket:read",
        ]
    );

    assert_equiv!(
        organization
            .get_scopes_for_user(&door_person, connection)
            .unwrap()
            .iter()
            .map(|i| i.to_string())
            .collect::<Vec<String>>(),
        vec![
            "event:scan",
            "hold:read",
            "org:read-events",
            "redeem:ticket",
            "ticket:read",
        ]
    );

    assert_equiv!(
        organization
            .get_scopes_for_user(&promoter, connection)
            .unwrap()
            .iter()
            .map(|i| i.to_string())
            .collect::<Vec<String>>(),
        vec![
            "code:read",
            "code:write",
            "comp:read",
            "comp:write",
            "dashboard:read",
            "event:interest",
            "event:view-guests",
            "event:write",
            "hold:read",
            "hold:write",
            "order:read",
            "org:read-events",
            "ticket:read",
            "ticket-type:read",
            "ticket-type:write"
        ]
    );

    assert_equiv!(
        organization
            .get_scopes_for_user(&promoter_read_only, connection)
            .unwrap()
            .iter()
            .map(|i| i.to_string())
            .collect::<Vec<String>>(),
        vec![
            "code:read",
            "comp:read",
            "dashboard:read",
            "event:interest",
            "event:view-guests",
            "hold:read",
            "order:read",
            "org:read-events",
            "ticket:read",
            "ticket-type:read"
        ]
    );

    assert_equiv!(
        organization
            .get_scopes_for_user(&member, connection)
            .unwrap()
            .iter()
            .map(|i| i.to_string())
            .collect::<Vec<String>>(),
        vec![
            "artist:write",
            "box-office-ticket:read",
            "box-office-ticket:write",
            "code:read",
            "code:write",
            "comp:read",
            "comp:write",
            "dashboard:read",
            "event:cancel",
            "event:interest",
            "event:scan",
            "event:view-guests",
            "event:write",
            "hold:read",
            "hold:write",
            "order:read",
            "order:read-own",
            "order:refund",
            "org:fans",
            "org:read",
            "org:read-events",
            "redeem:ticket",
            "ticket:admin",
            "ticket:read",
            "ticket:transfer",
            "ticket-type:read",
            "ticket-type:write",
            "venue:write",
        ]
    );

    assert_equiv!(
        organization
            .get_scopes_for_user(&admin, connection)
            .unwrap()
            .iter()
            .map(|i| i.to_string())
            .collect::<Vec<String>>(),
        vec![
            "artist:write",
            "box-office-ticket:read",
            "box-office-ticket:write",
            "code:read",
            "code:write",
            "comp:read",
            "comp:write",
            "dashboard:read",
            "event:cancel",
            "event:financial-reports",
            "event:interest",
            "event:reports",
            "event:scan",
            "event:view-guests",
            "event:write",
            "hold:read",
            "hold:write",
            "order:make-external-payment",
            "order:read",
            "order:read-own",
            "order:refund",
            "org:admin-users",
            "org:fans",
            "org:read",
            "org:read-events",
            "org:reports",
            "org:users",
            "org:write",
            "redeem:ticket",
            "ticket:admin",
            "ticket:read",
            "ticket:transfer",
            "ticket-type:read",
            "ticket-type:write",
            "user:read",
            "venue:write"
        ]
    );

    assert!(organization
        .get_scopes_for_user(&no_access_user, connection)
        .unwrap()
        .is_empty());
}

#[test]
fn add_user() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let user2 = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .finish();
    let organization_user = organization
        .add_user(user2.id, vec![Roles::OrgMember], Vec::new(), connection)
        .unwrap();

    assert_eq!(organization_user.user_id, user2.id);
    assert_eq!(organization_user.organization_id, organization.id);
    assert_eq!(organization_user.id.to_string().is_empty(), false);
    let user2 = User::find(user2.id, connection).unwrap();
    assert!(organization
        .get_roles_for_user(&user2, connection)
        .unwrap()
        .contains(&Roles::OrgMember));
}

#[test]
fn add_fee_schedule() {
    let project = TestProject::new();
    let creator = project.create_user().finish();

    let organization = project.create_organization().finish();
    let fee_structure = project.create_fee_schedule().finish(creator.id);

    let updated_fee_schedule =
        FeeSchedule::find(fee_structure.id, project.get_connection()).unwrap();
    assert_eq!(updated_fee_schedule.organization_id, Uuid::nil());

    organization
        .add_fee_schedule(&fee_structure, project.get_connection())
        .unwrap();
    let organization = Organization::find(organization.id, project.get_connection()).unwrap();
    assert_eq!(organization.fee_schedule_id, fee_structure.id);

    let updated_fee_schedule =
        FeeSchedule::find(fee_structure.id, project.get_connection()).unwrap();
    assert_eq!(updated_fee_schedule.organization_id, organization.id);
}

#[test]
fn search_fans() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let organization = project.create_organization().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    project
        .create_order()
        .for_event(&event)
        .for_user(&user)
        .finish();

    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 5,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();
    let total = cart.calculate_total(connection).unwrap();

    cart.add_external_payment(Some("test".to_string()), user.id, total, connection)
        .unwrap();

    let search_results = organization
        .search_fans(
            None,
            0,
            100,
            FanSortField::FirstName,
            SortingDir::Asc,
            &project.connection,
        )
        .unwrap();
    assert_eq!(search_results.data[0].user_id, user.id);
    let search_results = organization
        .search_fans(
            Some("NOT A REAL NAME".to_string()),
            0,
            100,
            FanSortField::FirstName,
            SortingDir::Asc,
            &project.connection,
        )
        .unwrap();
    assert_eq!(search_results.data.len(), 0);
}
