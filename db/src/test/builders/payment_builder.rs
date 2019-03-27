use diesel::prelude::*;
use models::*;

pub struct PaymentBuilder<'a> {
    user: Option<User>,
    organization: Option<Organization>,
    event: Option<Event>,
    status: PaymentStatus,
    connection: &'a PgConnection,
}

impl<'a> PaymentBuilder<'a> {
    pub fn new(connection: &'a PgConnection) -> Self {
        PaymentBuilder {
            user: None,
            organization: None,
            event: None,
            status: PaymentStatus::Completed,
            connection,
        }
    }

    pub fn with_user(mut self, user: &User) -> PaymentBuilder<'a> {
        self.user = Some(user.clone());
        self
    }
    pub fn with_event(mut self, event: &Event) -> PaymentBuilder<'a> {
        self.event = Some(event.clone());
        self
    }

    pub fn with_organization(mut self, organization: &Organization) -> PaymentBuilder<'a> {
        self.organization = Some(organization.clone());
        self
    }

    pub fn with_status(mut self, status: PaymentStatus) -> PaymentBuilder<'a> {
        self.status = status;
        self
    }

    pub fn finish(self) -> Payment {
        let mut cart =
            Order::find_or_create_cart(&self.user.clone().unwrap(), self.connection).unwrap();
        let ticket_type = &self
            .event
            .unwrap()
            .ticket_types(true, None, self.connection)
            .unwrap()[0];
        cart.update_quantities(
            self.user.clone().unwrap().id,
            &[UpdateOrderItem {
                ticket_type_id: ticket_type.id,
                quantity: 1,
                redemption_code: None,
            }],
            false,
            false,
            self.connection,
        )
        .unwrap();
        let total = cart.calculate_total(self.connection).unwrap();
        cart.add_provider_payment(
            Some("Test".to_string()),
            PaymentProviders::External,
            Some(self.user.unwrap().id),
            total,
            self.status,
            Some("nonce".to_string()),
            json!(null),
            self.connection,
        )
        .unwrap()
    }
}
