#[derive(Default, Serialize, Debug)]
pub struct CoverPhoto {
    pub source: String,
    pub offset_x: i32,
    pub offset_y: i32,
}

impl CoverPhoto {
    pub fn new(source: String) -> CoverPhoto {
        CoverPhoto {
            source,
            offset_x: 0,
            offset_y: 0,
        }
    }
}
