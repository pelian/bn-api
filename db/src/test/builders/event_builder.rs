use chrono::prelude::*;
use chrono::NaiveDate;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use prelude::*;
use rand::prelude::*;
use test::builders::*;
use time::Duration;
use uuid::Uuid;

pub struct EventBuilder<'a> {
    name: String,
    status: EventStatus,
    organization_id: Option<Uuid>,
    venue_id: Option<Uuid>,
    event_start: Option<NaiveDateTime>,
    event_end: Option<NaiveDateTime>,
    connection: &'a PgConnection,
    with_tickets: bool,
    with_ticket_pricing: bool,
    ticket_quantity: u32,
    ticket_type_count: i64,
    is_external: bool,
    publish_date: Option<NaiveDateTime>,
    private_access_code: Option<String>,
}

impl<'a> EventBuilder<'a> {
    pub fn new(connection: &PgConnection) -> EventBuilder {
        let x: u32 = random();
        EventBuilder {
            name: format!("Event {}", x).into(),
            status: EventStatus::Published,
            organization_id: None,
            venue_id: None,
            event_start: None,
            event_end: None,
            connection,
            with_tickets: false,
            with_ticket_pricing: false,
            ticket_quantity: 100,
            ticket_type_count: 1,
            is_external: false,
            publish_date: Some(NaiveDate::from_ymd(2018, 7, 8).and_hms(9, 10, 11)),
            private_access_code: None,
        }
    }

    pub fn with_ticket_type_count(mut self, ticket_type_count: i64) -> Self {
        self.ticket_type_count = ticket_type_count;
        self
    }

    pub fn external(mut self) -> Self {
        self.is_external = true;
        self
    }

    pub fn as_private(mut self, private_access_code: String) -> Self {
        self.private_access_code = Some(private_access_code);
        self
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.name = name;
        self
    }

    pub fn with_status(mut self, status: EventStatus) -> Self {
        if status != EventStatus::Published {
            self.publish_date = None;
        }
        self.status = status;
        self
    }

    pub fn with_organization(mut self, organization: &Organization) -> Self {
        self.organization_id = Some(organization.id.clone());
        self
    }

    pub fn with_venue(mut self, venue: &Venue) -> Self {
        self.venue_id = Some(venue.id.clone());
        self
    }

    pub fn with_event_start(mut self, date: NaiveDateTime) -> Self {
        self.event_start = Some(date);
        self
    }

    pub fn with_event_end(mut self, date: NaiveDateTime) -> Self {
        self.event_end = Some(date);
        self
    }

    pub fn with_publish_date(mut self, date: NaiveDateTime) -> Self {
        self.publish_date = Some(date);
        self
    }

    pub fn with_tickets(mut self) -> Self {
        self.with_tickets = true;
        self
    }

    pub fn with_a_specific_number_of_tickets(mut self, num: u32) -> Self {
        self.with_tickets = true;
        self.ticket_quantity = num;
        self
    }

    pub fn with_ticket_pricing(mut self) -> Self {
        self.with_tickets = true;
        self.with_ticket_pricing = true;
        self
    }

    pub fn finish(&mut self) -> Event {
        let organization_id = self
            .organization_id
            .or_else(|| Some(OrganizationBuilder::new(self.connection).finish().id))
            .unwrap();

        let event = Event::create(
            &self.name,
            self.status,
            organization_id,
            self.venue_id,
            Some(
                self.event_start
                    .unwrap_or(NaiveDate::from_ymd(2016, 7, 8).and_hms(9, 10, 11)),
            ),
            Some(NaiveDate::from_ymd(2016, 7, 8).and_hms(7, 8, 10)),
            self.publish_date,
            self.event_end,
        )
        .commit(self.connection)
        .unwrap();

        let mut attributes = EventEditableAttributes {
            promo_image_url: Some(Some("http://localhost".to_string())),
            ..Default::default()
        };
        if self.private_access_code.is_some() {
            attributes.private_access_code = Some(self.private_access_code.clone());
        }

        if self.is_external {
            attributes.is_external = Some(true);
        }

        let event = event.update(attributes, self.connection).unwrap();

        if self.with_tickets {
            let early_bird_start = NaiveDateTime::from(Utc::now().naive_utc() - Duration::days(2));
            let early_bird_end = NaiveDateTime::from(Utc::now().naive_utc() - Duration::days(1));
            let standard_start = NaiveDateTime::from(Utc::now().naive_utc() - Duration::days(1));
            let standard_end = NaiveDateTime::from(Utc::now().naive_utc() + Duration::days(2));

            let event_start = NaiveDateTime::from(Utc::now().naive_utc() + Duration::days(2));
            let event_end = NaiveDateTime::from(Utc::now().naive_utc() + Duration::days(4));

            let wallet_id = event.issuer_wallet(self.connection).unwrap().id;

            for x in 0..self.ticket_type_count {
                let ticket_type = event
                    .add_ticket_type(
                        format!("Ticket Type {}", x).into(),
                        None,
                        self.ticket_quantity,
                        event_start,
                        event_end,
                        wallet_id,
                        None,
                        0,
                        100,
                        SoldOutBehavior::ShowSoldOut,
                        self.connection,
                    )
                    .unwrap();

                if self.with_ticket_pricing {
                    ticket_type
                        .add_ticket_pricing(
                            "Early bird".into(),
                            early_bird_start,
                            early_bird_end,
                            100,
                            false,
                            None,
                            self.connection,
                        )
                        .unwrap();

                    ticket_type
                        .add_ticket_pricing(
                            "Standard".into(),
                            standard_start,
                            standard_end,
                            150,
                            false,
                            None,
                            self.connection,
                        )
                        .unwrap();
                }

                Asset::find_by_ticket_type(ticket_type.id, self.connection)
                    .unwrap()
                    .update_blockchain_id(
                        format!("{}.{}", event.name, ticket_type.name).to_string(),
                        self.connection,
                    )
                    .unwrap();
            }
        }

        event
    }
}
