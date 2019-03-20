use fbid::FBID;
use nodes::Category;
use nodes::CoverPhoto;

#[derive(Serialize, Debug)]
pub struct Event {
    pub category: Category,
    pub name: String,
    pub description: String,
    pub place_id: FBID,
    pub timezone: String,
    pub cover: Option<CoverPhoto>,
    pub start_time: String,
}

pub enum EventRole {}

impl Event {
    pub fn new(
        category: Category,
        name: String,
        description: String,
        place_id: FBID,
        timezone: String,
        cover: Option<CoverPhoto>,
        start_time: String,
    ) -> Event {
        Event {
            category,
            name,
            description,
            place_id,
            timezone,
            cover,
            start_time,
        }
    }
}
