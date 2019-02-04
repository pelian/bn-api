use chrono::prelude::*;

pub fn zero() -> NaiveDateTime {
    NaiveDate::from_ymd(1900, 1, 1).and_hms(1, 2, 3)
}

pub fn infinity() -> NaiveDateTime {
    NaiveDate::from_ymd(4900, 12, 31).and_hms(1, 2, 3)
}
