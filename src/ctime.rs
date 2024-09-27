use std::fmt;
use time::OffsetDateTime;

#[derive(Debug, PartialEq, Eq)]
pub struct DateTime {
    odt: OffsetDateTime,
}

impl DateTime {
    pub fn now() -> Self {
        let odt: OffsetDateTime = match OffsetDateTime::now_local() {
            Ok(dt) => dt,
            Err(_) => OffsetDateTime::now_utc(),
        };
        DateTime { odt }
    }

    pub fn from_timestamp(ts: f64) -> Self {
        let dummy_now = Self::now();
        let offset_seconds = dummy_now.odt.offset().whole_minutes() * 60;
        let ts_nano = (ts + offset_seconds as f64) * 1000000000.0;
        let odt: OffsetDateTime = match OffsetDateTime::from_unix_timestamp_nanos(ts_nano as i128) {
            Ok(x) => x,
            Err(_) => OffsetDateTime::now_utc(),
        };
        DateTime { odt }
    }

    pub fn unix_timestamp(&self) -> f64 {
        self.odt.unix_timestamp_nanos() as f64 / 1000000000.0
    }
}

impl fmt::Display for DateTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:03}",
            self.odt.year(),
            self.odt.month() as u8,
            self.odt.day(),
            self.odt.hour(),
            self.odt.minute(),
            self.odt.second(),
            self.odt.millisecond(),
        )
    }
}
