use bigneon_api::models::{DisplayTicketPricing, UserDisplayTicketType};
use bigneon_db::dev::times;
use bigneon_db::prelude::*;
use chrono::prelude::*;
use chrono::Duration;
use support::database::TestDatabase;

#[test]
fn from_ticket_type() {
    let database = TestDatabase::new();

    let admin = database.create_user().finish();
    let fee_schedule = database.create_fee_schedule().finish(admin.id);
    let organization = database
        .create_organization()
        .with_fee_schedule(&fee_schedule)
        .finish();
    let event = database
        .create_event()
        .with_organization(&organization)
        .with_ticket_pricing()
        .finish();
    let conn = database.connection.get();

    let ticket_type = event.ticket_types(true, None, conn).unwrap().remove(0);
    let box_office_pricing = ticket_type
        .add_ticket_pricing(
            "Box office".into(),
            NaiveDate::from_ymd(2016, 7, 8).and_hms(7, 8, 10),
            NaiveDate::from_ymd(9999, 7, 8).and_hms(7, 8, 10),
            5000,
            true,
            None,
            conn,
        )
        .unwrap();
    let ticket_pricing = ticket_type.current_ticket_pricing(false, conn).unwrap();
    let fee_in_cents = fee_schedule
        .get_range(ticket_pricing.price_in_cents, conn)
        .unwrap()
        .fee_in_cents;

    // Box office pricing
    let display_ticket_type =
        UserDisplayTicketType::from_ticket_type(&ticket_type, &fee_schedule, true, None, conn)
            .unwrap();
    assert_eq!(
        Some(
            DisplayTicketPricing::from_ticket_pricing(
                &box_office_pricing,
                &fee_schedule,
                None,
                true,
                conn
            )
            .unwrap()
        ),
        display_ticket_type.ticket_pricing,
    );
    let display_ticket_pricing = display_ticket_type.ticket_pricing.unwrap();
    // No fee for box office pricing
    assert_eq!(0, display_ticket_pricing.fee_in_cents);

    // New event nothing sold
    let display_ticket_type =
        UserDisplayTicketType::from_ticket_type(&ticket_type, &fee_schedule, false, None, conn)
            .unwrap();
    assert_eq!(display_ticket_type.available, 100);
    assert_eq!(display_ticket_type.status, TicketTypeStatus::Published);
    assert_eq!(
        Some(
            DisplayTicketPricing::from_ticket_pricing(
                &ticket_pricing,
                &fee_schedule,
                None,
                false,
                conn
            )
            .unwrap()
        ),
        display_ticket_type.ticket_pricing,
    );
    let display_ticket_pricing = display_ticket_type.ticket_pricing.unwrap();
    assert_eq!(fee_in_cents, display_ticket_pricing.fee_in_cents);

    // 10 tickets sold / reserved (via create_order for_event)
    let mut order = database
        .create_order()
        .for_event(&event)
        .quantity(10)
        .finish();
    let display_ticket_type =
        UserDisplayTicketType::from_ticket_type(&ticket_type, &fee_schedule, false, None, conn)
            .unwrap();
    assert_eq!(display_ticket_type.available, 90);
    assert_eq!(display_ticket_type.status, TicketTypeStatus::Published);

    // Remaining tickets sold
    order
        .update_quantities(
            admin.id,
            &[UpdateOrderItem {
                ticket_type_id: ticket_type.id,
                quantity: 100,
                redemption_code: None,
            }],
            false,
            false,
            conn,
        )
        .unwrap();
    let display_ticket_type =
        UserDisplayTicketType::from_ticket_type(&ticket_type, &fee_schedule, false, None, conn)
            .unwrap();
    assert_eq!(display_ticket_type.available, 0);
    assert_eq!(display_ticket_type.status, TicketTypeStatus::SoldOut);

    // Release some tickets
    order
        .update_quantities(
            admin.id,
            &[UpdateOrderItem {
                ticket_type_id: ticket_type.id,
                quantity: 90,
                redemption_code: None,
            }],
            false,
            false,
            conn,
        )
        .unwrap();
    let display_ticket_type =
        UserDisplayTicketType::from_ticket_type(&ticket_type, &fee_schedule, false, None, conn)
            .unwrap();
    assert_eq!(display_ticket_type.available, 10);
    assert_eq!(display_ticket_type.status, TicketTypeStatus::Published);

    // Holds code with discount
    let hold = database
        .create_hold()
        .with_hold_type(HoldTypes::Discount)
        .with_quantity(1)
        .with_ticket_type_id(ticket_type.id)
        .finish();
    assert_eq!(Some(10), hold.discount_in_cents);
    let discounted_fee_in_cents = fee_schedule
        .get_range(ticket_pricing.price_in_cents - 10, conn)
        .unwrap()
        .fee_in_cents;
    let display_ticket_type = UserDisplayTicketType::from_ticket_type(
        &ticket_type,
        &fee_schedule,
        false,
        Some(hold.redemption_code),
        conn,
    )
    .unwrap();
    let display_ticket_pricing = display_ticket_type.ticket_pricing.unwrap();
    assert_eq!(display_ticket_pricing.discount_in_cents, 10);
    assert_eq!(display_ticket_pricing.fee_in_cents, discounted_fee_in_cents);

    // Comps code
    let hold = database
        .create_hold()
        .with_hold_type(HoldTypes::Comp)
        .with_quantity(1)
        .with_ticket_type_id(ticket_type.id)
        .finish();
    let display_ticket_type = UserDisplayTicketType::from_ticket_type(
        &ticket_type,
        &fee_schedule,
        false,
        Some(hold.redemption_code),
        conn,
    )
    .unwrap();
    let display_ticket_pricing = display_ticket_type.ticket_pricing.unwrap();
    assert_eq!(
        display_ticket_pricing.discount_in_cents,
        display_ticket_pricing.price_in_cents
    );
    assert_eq!(display_ticket_pricing.fee_in_cents, 0);

    // Discount code
    let code = database
        .create_code()
        .with_event(&event)
        .with_code_type(CodeTypes::Discount)
        .for_ticket_type(&ticket_type)
        .with_discount_in_cents(Some(10))
        .finish();
    assert_eq!(Some(10), code.discount_in_cents);
    let discounted_fee_in_cents = fee_schedule
        .get_range(ticket_pricing.price_in_cents - 10, conn)
        .unwrap()
        .fee_in_cents;
    let display_ticket_type = UserDisplayTicketType::from_ticket_type(
        &ticket_type,
        &fee_schedule,
        false,
        Some(code.redemption_code),
        conn,
    )
    .unwrap();
    let display_ticket_pricing = display_ticket_type.ticket_pricing.unwrap();
    assert_eq!(display_ticket_pricing.discount_in_cents, 10);
    assert_eq!(display_ticket_pricing.fee_in_cents, discounted_fee_in_cents);

    // Code expired
    let code = database
        .create_code()
        .with_event(&event)
        .with_code_type(CodeTypes::Discount)
        .for_ticket_type(&ticket_type)
        .with_discount_in_cents(Some(10))
        .with_end_date(NaiveDateTime::from(
            Utc::now().naive_utc() - Duration::days(1),
        ))
        .finish();
    let display_ticket_type = UserDisplayTicketType::from_ticket_type(
        &ticket_type,
        &fee_schedule,
        false,
        Some(code.redemption_code),
        conn,
    )
    .unwrap();
    let display_ticket_pricing = display_ticket_type.ticket_pricing.unwrap();
    // Code is not applied
    assert_eq!(display_ticket_pricing.discount_in_cents, 0);
    assert_eq!(display_ticket_pricing.fee_in_cents, fee_in_cents);

    // No active ticket pricing
    let event = database.create_event().with_tickets().finish();
    let ticket_type = event.ticket_types(true, None, conn).unwrap().remove(0);
    let display_ticket_type =
        UserDisplayTicketType::from_ticket_type(&ticket_type, &fee_schedule, false, None, conn)
            .unwrap();
    assert_eq!(display_ticket_type.available, 100);
    assert_eq!(
        display_ticket_type.status,
        TicketTypeStatus::NoActivePricing
    );

    // Below min fee = 0

    let event = database.create_event().finish();
    event
        .add_ticket_type(
            "Free tickets".to_string(),
            None,
            10,
            times::zero(),
            times::infinity(),
            event.issuer_wallet(conn).unwrap().id,
            None,
            10,
            0,
            conn,
        )
        .unwrap();
    let ticket_type = event.ticket_types(true, None, conn).unwrap().remove(0);

    println!("{:?}", ticket_type);
    let display_ticket_type = UserDisplayTicketType::from_ticket_type(
        &ticket_type,
        &FeeSchedule::find(event.organization(conn).unwrap().fee_schedule_id, conn).unwrap(),
        false,
        None,
        conn,
    )
    .unwrap();

    println!("{:?}", display_ticket_type);
    assert_eq!(display_ticket_type.ticket_pricing.unwrap().fee_in_cents, 0);
}
