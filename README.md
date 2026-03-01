# Traccar-cli
A cli to read data from a [Tracarr](https://www.traccar.org/) instance

## Setup
1) Clone or download this repository
2) Inside the local copy, run `cargo install --path .` 
3) The cli now availabe as `traccar-cli`

## Usage
- `traccar-cli` to list all devices
- `traccar-cli serve` to serve data on devices over dbus

## Config
Config is done via a file at `~/.config/traccar/config.json`:

Example config:
```json
{
  "host": "https://yourinstance.example",
  "token": "your-token",
  "devices": {
      "1": {
      "display_name": "Optional local rename",
      "report_timeout_seconds":180 //When to show the device as late
    }
  }
}
```

Locations are typically displayed by their coordinates or instead by the name of any geofences a device is inside.

To instead of raw coordinates, display locations relative to landmarks, create a file at `~/.config/traccar/landmarks.json` formatted as follows:
```json
[
  {
    "name": "My secret lair",
    "location": {
      "lat": 49.7735021,
      "lng": 4.7209262
    }
  }, {
    ...
  }
]
```
