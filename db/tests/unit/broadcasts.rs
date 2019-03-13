use bigneon_db::dev::TestProject;
use bigneon_db::prelude::*;
use chrono::Utc;

#[test]
fn new_broadcast_commit() {
    let project = TestProject::new();
    let conn = project.get_connection();
    let event = project.create_event().finish();

    let send_at = Utc::now().naive_utc();

    let broadcast = Broadcast::create(
        event.id,
        BroadcastType::LastCall,
        BroadcastChannel::PushNotification,
        "myname".to_string(),
        None,
        Some(send_at),
        None,
    );

    assert_eq!(
        BroadcastStatus::Pending,
        broadcast.status,
        "Invalid status for NewBroadcast"
    );

    let broadcast = broadcast.commit(conn).unwrap();
    assert!(!broadcast.id.is_nil());
    assert_eq!("myname".to_string(), broadcast.name);

    assert_eq!(broadcast.channel, BroadcastChannel::PushNotification);

    let domain_actions =
        DomainAction::find_pending(Some(DomainActionTypes::BroadcastPushNotification), conn)
            .unwrap();
    assert_eq!(domain_actions.len(), 1, "DomainAction was not created");
    assert_eq!(domain_actions[0].main_table_id.unwrap(), broadcast.id);
    assert_eq!(domain_actions[0].scheduled_at.date(), send_at.date());
}

#[test]
fn broadcast_find() {
    let project = TestProject::new();
    let conn = project.get_connection();

    let broadcast = project.create_broadcast().finish();

    let found = Broadcast::find(broadcast.id, conn).unwrap();
    assert_eq!(broadcast.id, found.id);
}

#[test]
fn broadcast_find_by_id() {
    let project = TestProject::new();
    let conn = project.get_connection();

    let event = project.create_event().finish();
    let broadcast = project.create_broadcast().with_event_id(event.id).finish();

    let found = Broadcast::find_by_event_id(event.id, 0, 1, conn).unwrap();
    assert_eq!(1, found.data.len());
    assert_eq!(broadcast.id, found.data[0].id);
    assert_eq!(0, found.paging.page);
}

#[test]
fn broadcast_cancel() {
    let project = TestProject::new();
    let conn = project.get_connection();

    let broadcast = project.create_broadcast().finish();
    let broadcast = broadcast.cancel(conn).unwrap();

    assert_eq!(broadcast.status, BroadcastStatus::Cancelled);
}

#[test]
fn broadcast_update() {
    let project = TestProject::new();
    let conn = project.get_connection();

    let broadcast = project
        .create_broadcast()
        .with_name("old name".to_string())
        .with_channel(BroadcastChannel::PushNotification)
        .with_send_at(Utc::now().naive_utc())
        .with_status(BroadcastStatus::Pending)
        .finish();

    let attributes = BroadcastEditableAttributes {
        notification_type: None,
        channel: None,
        name: Some("new name".to_string()),
        send_at: Some(None),
        status: Some(BroadcastStatus::InProgress),
    };

    let broadcast = broadcast.update(attributes, conn).unwrap();

    assert_eq!(broadcast.status, BroadcastStatus::InProgress);
    assert_eq!(broadcast.channel, BroadcastChannel::PushNotification);
    assert_eq!(broadcast.name, "new name");
    assert!(broadcast.send_at.is_none());
}

#[test]
fn broadcast_update_if_cancelled() {
    let project = TestProject::new();
    let conn = project.get_connection();

    let broadcast = project
        .create_broadcast()
        .with_status(BroadcastStatus::Cancelled)
        .finish();

    let attributes = BroadcastEditableAttributes {
        notification_type: None,
        channel: None,
        name: Some("new name".to_string()),
        send_at: Some(None),
        status: Some(BroadcastStatus::InProgress),
    };

    let error = broadcast.update(attributes, conn).err();
    assert!(
        error.is_some(),
        "broadcast.update did not return expected error"
    );
    let error = error.unwrap();
    assert_eq!(error.error_code, ErrorCode::UpdateError);
    assert_eq!(
        "This broadcast has been cancelled, it cannot be modified.",
        error.cause.unwrap()
    );
}

#[test]
fn broadcast_set_in_progress() {
    let project = TestProject::new();
    let conn = project.get_connection();

    let broadcast = project
        .create_broadcast()
        .with_status(BroadcastStatus::Pending)
        .finish();

    let broadcast = broadcast.set_in_progress(conn).unwrap();
    assert_eq!(BroadcastStatus::InProgress, broadcast.status);
}
