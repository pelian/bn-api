extern crate chrono;
extern crate facebook;

use chrono::prelude::*;
use facebook::prelude::*;

fn main() {
    let url = FacebookClient::get_login_url(
        "<app_id>",
        Some("<url>"),
        "test",
        &["email", "manage_pages"],
    );
    println!("{}", url);

    let accounts = fb.me.accounts.list().unwrap();

    let event = Event {
        name: "Hello world".to_string(),
        category: Category::MusicEvent,
        description: "This is a test event".to_string(),
        start_time: Utc::now().naive_utc().to_string(),
        timezone: "Africa/Harare".to_string(),
        cover: CoverPhoto {
            source: "http://noimg.com".to_string(),
            offset_x: 0,
            offset_y: 0,
        },
        place_id: FBID("http://www.facebook.com/pages/<page_id>".to_string()),
    };
    fb.official_events.create(event).unwrap();
}
