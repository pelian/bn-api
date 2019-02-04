use diesel::prelude::*;
use models::*;
use rand::prelude::*;
use std::collections::HashMap;
use test::builders::user_builder::UserBuilder;
use uuid::Uuid;

pub struct OrganizationBuilder<'a> {
    name: String,
    members: HashMap<Uuid, Roles>,
    connection: &'a PgConnection,
    fee_schedule: Option<FeeSchedule>,
    event_fee_in_cents: Option<i64>,
    company_fee_in_cents: Option<i64>,
    client_fee_in_cents: Option<i64>,
    use_address: bool,
}

impl<'a> OrganizationBuilder<'a> {
    pub fn new(connection: &'a PgConnection) -> OrganizationBuilder {
        let x: u16 = random();
        OrganizationBuilder {
            name: format!("test org{}", x).into(),
            members: HashMap::new(),
            fee_schedule: None,
            connection,
            use_address: false,
            event_fee_in_cents: None,
            company_fee_in_cents: None,
            client_fee_in_cents: None,
        }
    }

    pub fn with_member(mut self, user: &User, role: Roles) -> OrganizationBuilder<'a> {
        self.members.insert(user.id.clone(), role);
        self
    }

    pub fn with_address(mut self) -> OrganizationBuilder<'a> {
        self.use_address = true;
        self
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.name = name;
        self
    }

    pub fn with_fee_schedule(mut self, fee_schedule: &FeeSchedule) -> OrganizationBuilder<'a> {
        self.fee_schedule = Some(fee_schedule.clone());
        self
    }

    pub fn with_event_fee(mut self) -> Self {
        self.event_fee_in_cents = Some(250);
        self.company_fee_in_cents = Some(100);
        self.client_fee_in_cents = Some(150);
        self
    }

    pub fn finish(mut self) -> Organization {
        let members = self.members.clone();
        let current_user_id = match members.iter().find(|(_, v)| *v.clone() == Roles::OrgOwner) {
            Some((owner_id, _)) => owner_id.clone(),
            None => UserBuilder::new(self.connection).finish().id,
        };

        if self.fee_schedule.is_none() {
            let x: u16 = random();
            let fee_schedule = FeeSchedule::create(
                format!("{} fees.{}", self.name, x).into(),
                vec![NewFeeScheduleRange {
                    min_price_in_cents: 1,
                    company_fee_in_cents: 20,
                    client_fee_in_cents: 30,
                }],
            )
            .commit(current_user_id, self.connection);
            self.fee_schedule = Some(fee_schedule.unwrap());
        }

        let mut organization = Organization::create(&self.name, self.fee_schedule.unwrap().id)
            .commit("encryption_key", current_user_id, self.connection)
            .unwrap();

        let event_fee_update = OrganizationEditableAttributes {
            company_event_fee_in_cents: self.company_fee_in_cents,
            client_event_fee_in_cents: self.client_fee_in_cents,
            ..Default::default()
        };

        let _ = organization
            .update(
                event_fee_update,
                &"encryption_key".to_string(),
                self.connection,
            )
            .unwrap();

        for (user_id, role) in self.members {
            OrganizationUser::create(organization.id, user_id, vec![role])
                .commit(self.connection)
                .unwrap();
        }

        Wallet::create_for_organization(
            organization.id,
            String::from("Default wallet"),
            self.connection,
        )
        .unwrap();

        if self.use_address {
            let mut attrs: OrganizationEditableAttributes = Default::default();

            attrs.address = Some(<String>::from("Test Address"));
            attrs.city = Some(<String>::from("Test Address"));
            attrs.state = Some(<String>::from("Test state"));
            attrs.country = Some(<String>::from("Test country"));
            attrs.postal_code = Some(<String>::from("0124"));
            attrs.phone = Some(<String>::from("+27123456789"));

            organization = organization
                .update(attrs, &"encryption_key".to_string(), self.connection)
                .unwrap();
        }
        organization
    }
}
