use bigneon_db::dev::TestProject;
use bigneon_db::prelude::*;
use bigneon_db::schema::ticket_instances;

use chrono::NaiveDate;
use diesel::prelude::*;
use uuid::Uuid;

#[test]
fn update_asset_blockchain_id() {
    let project = TestProject::new();
    let conn = project.get_connection();
    let event = project.create_event().finish();
    let wallet_id = event.issuer_wallet(conn).unwrap().id;
    let sd = NaiveDate::from_ymd(2016, 7, 8).and_hms(4, 10, 11);
    let ed = NaiveDate::from_ymd(2016, 7, 9).and_hms(4, 10, 11);
    let ticket_type = event
        .add_ticket_type(
            "VIP".to_string(),
            None,
            100,
            sd,
            ed,
            wallet_id,
            None,
            0,
            100,
            SoldOutBehavior::ShowSoldOut,
            false,
            conn,
        )
        .unwrap();

    let asset = Asset::find_by_ticket_type(ticket_type.id, conn).unwrap();
    let tari_asset_id = Uuid::new_v4().to_string();
    let asset = asset
        .update_blockchain_id(tari_asset_id.clone(), conn)
        .unwrap();

    assert_eq!(asset.blockchain_asset_id, Some(tari_asset_id));
}

#[test]
fn asset_find() {
    let project = TestProject::new();
    let conn = project.get_connection();
    let event = project.create_event().finish();

    let wallet_id = event.issuer_wallet(conn).unwrap().id;
    let sd = NaiveDate::from_ymd(2016, 7, 8).and_hms(4, 10, 11);
    let ed = NaiveDate::from_ymd(2016, 7, 9).and_hms(4, 10, 11);

    event
        .add_ticket_type(
            "VIP".to_string(),
            None,
            100,
            sd,
            ed,
            wallet_id,
            None,
            0,
            100,
            SoldOutBehavior::ShowSoldOut,
            false,
            conn,
        )
        .unwrap();

    let ticket_instance = ticket_instances::table
        .filter(ticket_instances::wallet_id.eq(wallet_id))
        .first::<TicketInstance>(conn)
        .unwrap();

    let asset = Asset::find(ticket_instance.asset_id, conn).unwrap();

    assert_eq!(asset.id, ticket_instance.asset_id);
}
