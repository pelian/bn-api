use bigneon_db::prelude::*;
use db::Connection;
use domain_events::executor_future::ExecutorFuture;
use domain_events::routing::DomainActionExecutor;
use errors::*;
use futures::future;
use itertools::Itertools;
use log::Level::Error;
use utils::communication::*;

pub struct BroadcastPushNotificationExecutor {}

impl DomainActionExecutor for BroadcastPushNotificationExecutor {
    fn execute(&self, action: DomainAction, conn: Connection) -> ExecutorFuture {
        match self.perform_job(&action, &conn) {
            Ok(_) => ExecutorFuture::new(action, conn, Box::new(future::ok(()))),
            Err(e) => {
                jlog!(Error, "Send tickets mail action failed", {"action_id": action.id, "main_table_id":action.main_table_id,  "error": e.to_string()});
                ExecutorFuture::new(action, conn, Box::new(future::err(e)))
            }
        }
    }
}

impl BroadcastPushNotificationExecutor {
    pub fn new() -> BroadcastPushNotificationExecutor {
        BroadcastPushNotificationExecutor {}
    }

    fn perform_job(&self, action: &DomainAction, conn: &Connection) -> Result<(), BigNeonError> {
        let action_data: BroadcastPushNotificationAction =
            serde_json::from_value(action.payload.clone())?;
        let broadcast_id = action.main_table_id.ok_or(ApplicationError::new(
            "No broadcast id attached to domain action".to_string(),
        ))?;
        let broadcast = Broadcast::find(broadcast_id, conn.get())?;
        if broadcast.status == BroadcastStatus::Cancelled {
            return Ok(());
        }

        let broadcast = broadcast.set_in_progress(conn.get())?;
        let (audience_type, message) = match broadcast.notification_type {
            BroadcastType::LastCall => {
                (BroadcastAudience::PeopleAtTheEvent, "🗣LAST CALL! 🍻The bar is closing soon, grab something now before it's too late!")
            }
        };

        let audience = match audience_type {
            BroadcastAudience::PeopleAtTheEvent => {
                Event::checked_in_users(broadcast.event_id, conn.get())?
            }
        };

        for user in audience {
            let tokens = user
                .push_notification_tokens(conn.get())?
                .into_iter()
                .map(|pt| pt.token)
                .collect_vec();

            if tokens.len() > 0 {
                DomainAction::create(
                    None,
                    DomainActionTypes::Communication,
                    Some(CommunicationChannelType::Push),
                    serde_json::to_value(Communication::new(
                        CommunicationType::Push,
                        message.to_string(),
                        None,
                        None,
                        CommAddress::from_vec(tokens),
                        None,
                        None,
                    ))?,
                    Some(Tables::Events.to_string()),
                    Some(action_data.event_id),
                )
                .commit(conn.get())?;
            }
        }

        Ok(())
    }
}
