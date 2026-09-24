use core::fmt;
use std::{fmt::Display, time::Duration};

use chrono::{DateTime, Utc};
use duration_human::DurationHuman;
use geo::Point;

#[derive(Clone, Debug)]
pub enum ReportPosition {
    RelativeTo {
        distance: f64,
        bearing: f64,
        name: String,
    },
    InGeofences(Vec<String>),
    BarePosition(Point),
}

impl ReportPosition {
    pub fn geofences(&self) -> &[String] {
        match self {
            ReportPosition::InGeofences(fences) => fences,
            _ => &[],
        }
    }
}

impl Display for ReportPosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            ReportPosition::RelativeTo {
                distance,
                bearing,
                name,
            } => f.write_fmt(format_args!(
                "{} {} of {}",
                format_distance(distance).unwrap_or("err".to_string()),
                bearing_to_compass_dir(*bearing),
                name
            )),

            ReportPosition::InGeofences(items) => {
                let fences = items
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<&str>>()
                    .join(",");

                f.write_str(&fences)
            }

            ReportPosition::BarePosition(point) => {
                f.write_fmt(format_args!("{},{}", point.x(), point.y()))
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct Report {
    pub name: String,
    pub position: ReportPosition,
    pub in_timeout: Option<bool>,
    pub seconds_ago: u32,
    pub next_update_expected: Option<DateTime<Utc>>,
}

fn append_age(
    f: &mut std::fmt::Formatter<'_>,
    seconds_ago: u32,
    in_timeout: Option<bool>,
) -> Result<(), fmt::Error> {
    use owo_colors::{
        OwoColorize,
        colors::{Green, Red},
    };
    let duration = Duration::from_secs(u64::from(seconds_ago));
    let duration = DurationHuman::from(duration);

    match in_timeout {
        // Don't format if the option is not provided
        None => f.write_fmt(format_args!(" {duration:#} ago")),

        // Color green or red depending on timeout setting
        Some(true) => f.write_fmt(format_args!(" {:#} ago", duration.fg::<Green>())),
        Some(false) => f.write_fmt(format_args!(" {:#} ago", duration.fg::<Red>())),
    }
}

impl Display for Report {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name)?;
        match &self.position {
            ReportPosition::RelativeTo {
                distance: _,
                bearing: _,
                name: _,
            } => f.write_str(" was ")?,
            ReportPosition::InGeofences(_) => f.write_str(" was in ")?,
            ReportPosition::BarePosition(_) => f.write_str(" was at ")?,
        }
        // f.write_str(" was in ")?;
        f.write_fmt(format_args!("{}", self.position))?;

        append_age(f, self.seconds_ago, self.in_timeout)
    }
}

impl Report {
    pub fn new(
        name: String,
        position: ReportPosition,
        in_timeout: Option<bool>,
        seconds_ago: u32,
        predicted_update: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            next_update_expected: predicted_update,
            name,
            position,
            in_timeout,
            seconds_ago,
        }
    }
}
fn bearing_to_compass_dir(bearing: f64) -> &'static str {
    let b = bearing.rem_euclid(360.0);

    match b {
        348.75..=360.0 | 0.0..11.25 => "N",
        11.25..33.75 => "NNE",
        33.75..56.25 => "NE",
        56.25..78.75 => "ENE",
        78.75..101.25 => "E",
        101.25..123.75 => "ESE",
        123.75..146.25 => "SE",
        146.25..168.75 => "SSE",
        168.75..191.25 => "S",
        191.25..213.75 => "SSW",
        213.75..236.25 => "SW",
        236.25..258.75 => "WSW",
        258.75..281.25 => "W",
        281.25..303.75 => "WNW",
        303.75..326.25 => "NW",
        326.25..348.75 => "NNW",
        _ => unreachable!(),
    }
}

fn format_distance(distance: &f64) -> Option<String> {
    match distance {
        ..0.0 => None,
        0.0..1000.0 => Some(format!("{distance:.0}m")), // 0-999 meters
        1000f64..10_000f64 => Some(format!("{:.2}km", distance / 1000.0)), //1km-9.99km,
        10_000f64..100_000f64 => Some(format!("{:.1}km", distance / 1000.0)), //10.0km-99.9km
        100_000f64.. => Some(format!("{:.0}km", distance / 1000.0)), //100 km
        _ => None,                                      // _ => Some("Very far away".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compass_dir_doesnt_panic() {
        bearing_to_compass_dir(0.0);
        bearing_to_compass_dir(-0.0);
        bearing_to_compass_dir(-1.0);
        bearing_to_compass_dir(360.0);
        bearing_to_compass_dir(361.0);
    }
}
