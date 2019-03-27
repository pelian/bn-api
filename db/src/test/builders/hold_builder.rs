use chrono::prelude::*;
use diesel::prelude::*;
use models::*;
use rand::prelude::*;
use test::builders::*;
use uuid::Uuid;

pub struct HoldBuilder<'a> {
    name: String,
    redemption_code: String,
    event_id: Option<Uuid>,
    end_at: Option<NaiveDateTime>,
    ticket_type_id: Option<Uuid>,
    hold_type: HoldTypes,
    quantity: u32,
    max_per_user: Option<u32>,
    connection: &'a PgConnection,
}

impl<'a> HoldBuilder<'a> {
    pub fn new(connection: &'a PgConnection) -> Self {
        let x: u32 = random();
        HoldBuilder {
            name: format!("Hold {}", x).into(),
            redemption_code: format!("REDEEM{}", x).into(),
            connection,
            hold_type: HoldTypes::Discount,
            event_id: None,
            ticket_type_id: None,
            quantity: 10,
            end_at: None,
            max_per_user: None,
        }
    }

    pub fn with_hold_type(mut self, hold_type: HoldTypes) -> Self {
        self.hold_type = hold_type;
        self
    }

    pub fn with_quantity(mut self, quantity: u32) -> Self {
        self.quantity = quantity;
        self
    }

    pub fn with_end_at(mut self, end_at: NaiveDateTime) -> Self {
        self.end_at = Some(end_at);
        self
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.name = name;
        self
    }

    pub fn with_event(mut self, event: &Event) -> Self {
        self.event_id = Some(event.id);
        self
    }

    pub fn with_max_per_user(mut self, max_per_user: u32) -> Self {
        self.max_per_user = Some(max_per_user);
        self
    }

    pub fn with_ticket_type_id(mut self, ticket_type_id: Uuid) -> Self {
        self.ticket_type_id = Some(ticket_type_id);
        self
    }

    pub fn finish(mut self) -> Hold {
        // Use passed in ticket type id and its event if present otherwise fetch from event id
        let ticket_type_id = match self.ticket_type_id {
            Some(id) => {
                let ticket_type = TicketType::find(id, self.connection).unwrap();
                self.event_id = Some(
                    Event::find(ticket_type.event_id, self.connection)
                        .unwrap()
                        .id,
                );

                id
            }
            None => {
                if self.event_id.is_none() {
                    self.event_id = Some(
                        EventBuilder::new(self.connection)
                            .with_ticket_pricing()
                            .with_tickets()
                            .finish()
                            .id,
                    );
                }

                let event = Event::find(self.event_id.unwrap(), self.connection).unwrap();
                event.ticket_types(true, None, self.connection).unwrap()[0].id
            }
        };

        let hold = Hold::create_hold(
            self.name,
            self.event_id.unwrap(),
            Some(self.redemption_code),
            if self.hold_type == HoldTypes::Discount {
                Some(10)
            } else {
                None
            },
            self.end_at,
            self.max_per_user,
            self.hold_type,
            ticket_type_id,
        )
        .commit(None, self.connection)
        .unwrap();

        hold.set_quantity(None, self.quantity, self.connection)
            .unwrap();
        hold
    }
}
