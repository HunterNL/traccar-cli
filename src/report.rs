use core::fmt;
use std::fmt::Display;

use chrono::{DateTime, Utc};
use geo::Point;

use crate::format_distance;

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
                f.write_fmt(format_args!("at {},{}", point.x(), point.y()))
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
    match in_timeout {
        // Don't format if the option is not provided
        None => f.write_fmt(format_args!(" {} seconds ago", seconds_ago)),

        // Color green or red depending on timeout setting
        Some(true) => f.write_fmt(format_args!(" {} seconds ago", seconds_ago.fg::<Green>())),
        Some(false) => f.write_fmt(format_args!(" {} seconds ago", seconds_ago.fg::<Red>())),
    }
}

impl Display for Report {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name)?;
        f.write_str(" was in ")?;
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
