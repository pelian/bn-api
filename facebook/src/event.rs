use category::Category;
use chrono::prelude::*;
use cover_photo::CoverPhoto;
use fbid::FBID;

#[derive(Serialize, Debug)]
pub struct Event {
    pub category: Category,
    pub name: String,
    pub description: String,
    pub place_id: FBID,
    pub timezone: String,
    pub cover: CoverPhoto,
    pub start_time: String,
}

pub enum EventRole {}
