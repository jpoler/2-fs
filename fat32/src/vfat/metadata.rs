use std::fmt;

use traits::{self, Metadata as MetadataTrait, Timestamp as TimestampTrait};

/// A date as represented in FAT32 on-disk structures.
#[repr(C, packed)]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Date(u16);

impl Date {
    pub fn from_raw(raw: u16) -> Date {
        Date(raw)
    }

    fn from_ymd(year: u16, month: u16, day: u16) -> Date {
        Date(Date::to_year(year) | Date::to_month(month) | Date::to_day(day))
    }

    fn to_year(year: u16) -> u16 {
        (year - 1980) << 9
    }

    fn to_month(month: u16) -> u16 {
        month << 5
    }

    fn to_day(day: u16) -> u16 {
        day
    }

    fn year(&self) -> usize {
        (self.0 >> 9) as usize + 1980usize
    }

    fn month(&self) -> u8 {
        (self.0 >> 5) as u8 & 0xF
    }

    fn day(&self) -> u8 {
        self.0 as u8 & 0x1F
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "year: {}, month: {}, day{}",
            self.year(),
            self.month(),
            self.day()
        )
    }
}

/// Time as represented in FAT32 on-disk structures.
#[repr(C, packed)]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Time(u16);

impl Time {
    pub fn from_raw(raw: u16) -> Time {
        Time(raw)
    }

    fn from_hms(hour: u16, minute: u16, second: u16) -> Time {
        Time(Time::to_hour(hour) | Time::to_minute(minute) | Time::to_second(second))
    }

    fn to_hour(hour: u16) -> u16 {
        hour << 11
    }

    fn to_minute(minute: u16) -> u16 {
        minute << 5
    }

    fn to_second(second: u16) -> u16 {
        second >> 1
    }

    fn hour(&self) -> u8 {
        (self.0 >> 11) as u8
    }

    fn minute(&self) -> u8 {
        ((self.0 >> 5) as u8) & 0x1F
    }

    fn second(&self) -> u8 {
        ((self.0 & 0x1F) as u8) << 1
    }
}

impl fmt::Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "hour: {}, minute: {}, second: {}",
            self.hour(),
            self.minute(),
            self.second()
        )
    }
}

/// File attributes as represented in FAT32 on-disk structures.
#[repr(C, packed)]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Attributes(u8);

impl Attributes {
    pub fn from_raw(raw: u8) -> Attributes {
        Attributes(raw)
    }

    fn read_only(&self) -> bool {
        self.0 & 0x01 != 0
    }

    fn hidden(&self) -> bool {
        self.0 & 0x02 != 0
    }

    fn system(&self) -> bool {
        self.0 & 0x04 != 0
    }

    fn volume_id(&self) -> bool {
        self.0 & 0x08 != 0
    }

    pub fn directory(&self) -> bool {
        self.0 & 0x10 != 0
    }

    fn archive(&self) -> bool {
        self.0 & 0x20 != 0
    }

    pub fn lfn(&self) -> bool {
        self.read_only() && self.hidden() && self.system() && self.volume_id()
    }
}

/// A structure containing a date and time.
#[derive(Default, Copy, Clone, Debug, PartialEq, Eq)]
pub struct Timestamp {
    pub date: Date,
    pub time: Time,
}

impl Timestamp {
    pub fn new(date: Date, time: Time) -> Timestamp {
        Timestamp { date, time }
    }
}

impl traits::Timestamp for Timestamp {
    fn year(&self) -> usize {
        self.date.year()
    }

    fn month(&self) -> u8 {
        self.date.month()
    }

    fn day(&self) -> u8 {
        self.date.day()
    }

    fn hour(&self) -> u8 {
        self.time.hour()
    }

    fn minute(&self) -> u8 {
        self.time.minute()
    }

    fn second(&self) -> u8 {
        self.time.second()
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Timestamp")
            .field("date", &self.date)
            .field("time", &self.time)
            .finish()
    }
}

/// Metadata for a directory entry.
#[derive(Default, Debug, Clone)]
pub struct Metadata {
    pub attributes: Attributes,
    pub created: Timestamp,
    pub accessed: Timestamp,
    pub modified: Timestamp,
    pub size: u64,
}

impl traits::Metadata for Metadata {
    type Timestamp = Timestamp;

    fn read_only(&self) -> bool {
        self.attributes.read_only()
    }

    fn hidden(&self) -> bool {
        self.attributes.hidden()
    }

    fn created(&self) -> Self::Timestamp {
        self.created
    }

    fn accessed(&self) -> Self::Timestamp {
        self.accessed
    }

    fn modified(&self) -> Self::Timestamp {
        self.modified
    }
}

impl fmt::Display for Metadata {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Metadata")
            .field("read_only", &self.read_only())
            .field("hidden", &self.hidden())
            .field("created", &self.created)
            .field("accessed", &self.accessed)
            .field("modified", &self.modified)
            .finish()
    }
}
