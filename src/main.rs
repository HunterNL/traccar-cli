use futures::future::join_all;
use owo_colors::OwoColorize;
use owo_colors::colors::Green;

use chrono::Utc;
use geo::Bearing;
use geo::Distance;
use geo::Point;
use traccar_lib::DeviceReponse;
use traccar_lib::GeoFenceResponse;
use traccar_lib::Position;

use crate::config::AppConfig;

mod config;

#[derive(Debug)]
struct Landmark {
    name: String,
    position: Point,
}

fn main() {
    let config = config::config_get();
    run(config);
}

#[tokio::main]
async fn run(config: AppConfig) {
    let client = traccar_lib::Traccar::new(config.host(), config.token());
    let devices = client.list_devices().await;
    let geofences = client.geofences_all().await;
    let landmarks = config.landmarks();

    // Join the actual position to a device
    let devices_with_position = join_all(devices.into_iter().map(async |device| {
        let position = client.position_get(device.position_id).await;
        (device, position)
    }))
    .await;

    // devices_with_position.iter().for_each(|(device, position)| {
    for (device, position) in devices_with_position.iter() {
        println!(
            "{}",
            report_device(device, position, geofences.as_slice(), landmarks)
        )
    }
}

fn report_device(
    device: &DeviceReponse,
    position: &Position,
    geofences: &[GeoFenceResponse],
    landmarks: &[Landmark],
) -> String {
    let now = Utc::now();
    let dtime = now.signed_duration_since(position.fix_time);
    let seconds_ago = dtime.as_seconds_f32().round();

    // Turn the ids inside a position into a vec of &geofence
    let fences: Vec<&GeoFenceResponse> = position
        .geofence_ids
        .iter()
        .filter_map(|id| geofences.iter().find(|geofence| *id == geofence.id))
        .collect();

    // If we are in a known geofence
    if !fences.is_empty() {
        return format!(
            "{} was in {} {} seconds ago",
            device.name,
            fences
                .iter()
                .map(|a| a.name.as_str())
                .collect::<Vec<&str>>()
                .join(","),
            seconds_ago.fg::<Green>()
        );
    }

    // We're not in a geofence, go find the nearest landmark
    let device_location = Point::new(position.longitude, position.latitude);

    let closest = landmarks
        .iter()
        .min_by(|l1, l2| {
            let a = geo::GeodesicMeasure::wgs84().distance(l1.position, device_location);
            let b = geo::GeodesicMeasure::wgs84().distance(l2.position, device_location);

            f64::total_cmp(&a, &b)
        })
        .unwrap();

    let bearing = geo::GeodesicMeasure::wgs84().bearing(closest.position, device_location);
    let distance = geo::GeodesicMeasure::wgs84().distance(closest.position, device_location);

    format!(
        "{} was {}m {} of {} {} seconds ago",
        device.name,
        distance.round(),
        bearing_to_compass_dir(bearing),
        closest.name,
        seconds_ago.fg::<Green>()
    )
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
