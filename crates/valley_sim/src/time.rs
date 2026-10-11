use serde::{Deserialize, Serialize};

/// Real milliseconds per 10 in-game minutes (documented as 7 seconds).
pub const TEN_MINUTES_MS: u32 = 7000;
pub const DAY_START: i32 = 600;
/// 2:00 AM, when the farmer passes out.
pub const DAY_END: i32 = 2600;
pub const DAYS_PER_SEASON: i32 = 28;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default, PartialOrd, Ord)]
pub enum Season {
    #[default]
    Spring,
    Summer,
    Fall,
    Winter,
}

impl Season {
    pub const ALL: [Season; 4] = [Season::Spring, Season::Summer, Season::Fall, Season::Winter];

    pub fn index(self) -> i32 {
        self as i32
    }
    pub fn key(self) -> &'static str {
        match self {
            Season::Spring => "spring",
            Season::Summer => "summer",
            Season::Fall => "fall",
            Season::Winter => "winter",
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Season::Spring => "Spring",
            Season::Summer => "Summer",
            Season::Fall => "Fall",
            Season::Winter => "Winter",
        }
    }
    pub fn parse(s: &str) -> Option<Season> {
        match s.trim().to_ascii_lowercase().as_str() {
            "spring" => Some(Season::Spring),
            "summer" => Some(Season::Summer),
            "fall" => Some(Season::Fall),
            "winter" => Some(Season::Winter),
            _ => None,
        }
    }
    pub fn next(self) -> Season {
        Season::ALL[((self.index() + 1) % 4) as usize]
    }
}

pub const WEEKDAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Weather {
    #[default]
    Sun,
    Rain,
    Storm,
    Snow,
    Wind,
    /// Once a year in summer: a mossy green downpour.
    GreenRain,
}

impl Weather {
    pub fn is_raining(self) -> bool {
        matches!(self, Weather::Rain | Weather::Storm | Weather::GreenRain)
    }
}

/// Summer days that can have the year's green rain (public list).
pub const GREEN_RAIN_DAYS: [i32; 8] = [5, 6, 7, 14, 15, 16, 18, 23];

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Clock {
    pub year: i32,
    pub season: Season,
    /// Day of month, 1..=28.
    pub day: i32,
    /// Military-style time, 600..=2600 (minutes always a multiple of 10).
    pub time: i32,
    /// Milliseconds accumulated toward the next 10-minute tick.
    pub ms: u32,
}

impl Default for Clock {
    fn default() -> Self {
        Self { year: 1, season: Season::Spring, day: 1, time: DAY_START, ms: 0 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClockEvent {
    TenMinutes,
    PassOut,
}

impl Clock {
    /// Days since the start of the game, starting at 1 (used for seeds).
    pub fn days_played(&self) -> i32 {
        (self.year - 1) * DAYS_PER_SEASON * 4 + self.season.index() * DAYS_PER_SEASON + self.day
    }

    pub fn weekday(&self) -> &'static str {
        WEEKDAYS[((self.day - 1) % 7) as usize]
    }

    pub fn weekday_index(&self) -> i32 {
        (self.day - 1) % 7
    }

    /// Add `minutes` to a military time value, carrying into hours.
    pub fn add_minutes(time: i32, minutes: i32) -> i32 {
        let total = (time / 100) * 60 + time % 100 + minutes;
        (total / 60) * 100 + total % 60
    }

    /// Minutes between two military times.
    pub fn minutes_between(a: i32, b: i32) -> i32 {
        ((b / 100) * 60 + b % 100) - ((a / 100) * 60 + a % 100)
    }

    /// Advance by real milliseconds; returns the events that fired.
    pub fn advance(&mut self, ms: u32) -> Vec<ClockEvent> {
        let mut ev = Vec::new();
        if self.time >= DAY_END {
            return ev;
        }
        self.ms += ms;
        while self.ms >= TEN_MINUTES_MS {
            self.ms -= TEN_MINUTES_MS;
            self.time = Self::add_minutes(self.time, 10);
            ev.push(ClockEvent::TenMinutes);
            if self.time >= DAY_END {
                self.time = DAY_END;
                self.ms = 0;
                ev.push(ClockEvent::PassOut);
                break;
            }
        }
        ev
    }

    pub fn next_day(&mut self) {
        self.day += 1;
        if self.day > DAYS_PER_SEASON {
            self.day = 1;
            self.season = self.season.next();
            if self.season == Season::Spring {
                self.year += 1;
            }
        }
        self.time = DAY_START;
        self.ms = 0;
    }

    /// "6:00 am" style display.
    pub fn time_string(time: i32) -> String {
        let h = (time / 100) % 24;
        let m = time % 100;
        let (h12, ampm) = match h {
            0 => (12, "am"),
            1..=11 => (h, "am"),
            12 => (12, "pm"),
            _ => (h - 12, "pm"),
        };
        format!("{h12}:{m:02} {ampm}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ticks_and_rollover() {
        let mut c = Clock::default();
        assert_eq!(c.advance(6999), vec![]);
        assert_eq!(c.advance(1), vec![ClockEvent::TenMinutes]);
        assert_eq!(c.time, 610);
        c.time = 650;
        c.advance(TEN_MINUTES_MS);
        assert_eq!(c.time, 700);
        c.time = 2550;
        let ev = c.advance(TEN_MINUTES_MS * 3);
        assert_eq!(ev, vec![ClockEvent::TenMinutes, ClockEvent::PassOut]);
        assert_eq!(c.time, DAY_END);
        c.day = 28;
        c.season = Season::Winter;
        c.next_day();
        assert_eq!((c.year, c.season, c.day, c.time), (2, Season::Spring, 1, 600));
        assert_eq!(Clock::time_string(1330), "1:30 pm");
        assert_eq!(Clock::time_string(2400), "12:00 am");
        assert_eq!(Clock::minutes_between(650, 720), 30);
    }
}
