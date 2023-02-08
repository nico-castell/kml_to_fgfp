use std::{error::Error, io::Write};

use xml::writer::EventWriter;

use super::Airport;

/// Used to write a waypoint to the .fgfp file.
pub struct Waypoint {
    pub number: usize,
    pub ident: String,
    pub lon: f64,
    pub lat: f64,
    pub altitude: usize,
}

/// An example of the sequence to look for in the .kml's xml is:
///
/// ```text
/// <Placemark>
///    <name>EZE11</name>
///    <styleUrl>#FixMark</styleUrl>
///    <coordinates>-58.594239,-34.811897,823</coordinates>
/// </Placemark>
/// ```
pub enum LookingFor {
    OpeningPlacemark,
    OpeningName,
    ContentName,
    ClosingName,
    OpeningStyleUrl,
    ContentStyleUrl,
    ClosingStyleUrl,
    OpeningCoordinates,
    ContentCoordinates,
    ClosingCoordinates,
    ClosingPlacemark,
}

/// Internal function to handle start events from the `transform_route` function.
pub fn handle_start_event(
    mut waypoint: Waypoint,
    mut current_search: LookingFor,
    mut drop: bool,
    wp: usize,
    name: &str,
) -> (Waypoint, LookingFor, bool) {
    // 1. Find opening of `Placemark`
    if matches!(current_search, LookingFor::OpeningPlacemark) && name == "Placemark" {
        waypoint.number = wp;
        current_search = LookingFor::OpeningName;
        drop = false;
    }

    // 2. Find opening of `name`
    if matches!(current_search, LookingFor::OpeningName) && name == "name" {
        current_search = LookingFor::ContentName;
    }

    // 5. Find opening of `styleUrl`
    if matches!(current_search, LookingFor::OpeningStyleUrl) && name == "styleUrl" {
        current_search = LookingFor::ContentStyleUrl;
    }

    // 8. Find opening of `coordinates`
    if matches!(current_search, LookingFor::OpeningCoordinates) && name == "coordinates" {
        current_search = LookingFor::ContentCoordinates;
    }

    (waypoint, current_search, drop)
}

/// Internal function to handle characters events from the `transform_route` function.
pub fn handle_characters_event(
    mut waypoint: Waypoint,
    mut current_search: LookingFor,
    mut drop: bool,
    line: String,
    departure_airport: &Option<Airport>,
    destination_airport: &Option<Airport>,
) -> (Waypoint, LookingFor, bool) {
    // 3. Find contents of `name`
    if matches!(current_search, LookingFor::ContentName) {
        waypoint.ident = String::from(&line);
        current_search = LookingFor::ClosingName;

        // Handle the waypoints that reference airports by dropping them.
        if let Some(airport) = departure_airport {
            if line == airport.ident {
                current_search = LookingFor::ClosingPlacemark;
                drop = true;
            }
        }

        if let Some(airport) = destination_airport {
            if line == airport.ident {
                current_search = LookingFor::ClosingPlacemark;
                drop = true;
            }
        }
    }

    // 6. Find contents of `styleUrl`
    if matches!(current_search, LookingFor::ContentStyleUrl) {
        if line != "#FixMark" {
            drop = true;

            // We found that this Placemark is not part of the route, so we avoid
            // further processing of the waypoint.
            current_search = LookingFor::ClosingPlacemark;

            return (waypoint, current_search, drop);
        }
        current_search = LookingFor::ClosingStyleUrl;
    }

    // 9. Find contents of `coordinates`
    if matches!(current_search, LookingFor::ContentCoordinates) {
        let data: Vec<&str> = line.split(',').map(|l| l.trim()).collect();

        let mut message = String::new();

        waypoint.lon = match data[0].parse() {
            Ok(d) => d,
            Err(e) => {
                message = e.to_string();
                0f64
            }
        };
        waypoint.lat = match data[1].parse() {
            Ok(d) => d,
            Err(e) => {
                message = e.to_string();
                0f64
            }
        };
        waypoint.altitude = {
            let meters: f64 = match data[2].parse() {
                Ok(d) => d,
                Err(e) => {
                    message = e.to_string();
                    0f64
                }
            };

            // We don't want exact precision, we need to be precise up to one hundred feet. Example:
            // If the real altitude is 12478.64 feet, we interpret that as 12500 feet. We divide by
            // one hundred and multiply by one hundred to let the round function do this for us.
            let feet = (meters * 3.280839895 / 100.0).round() * 100.0;
            feet as usize
        };

        if !message.is_empty() {
            eprintln!(
                "\x1B[01;33mDropping\x1B[00;01m {}\x1B[00m waypoint: {}",
                waypoint.ident, message
            );
            drop = true;
        }

        current_search = LookingFor::ClosingCoordinates;
    }

    (waypoint, current_search, drop)
}

/// Internal function to handle end events from the `transform_route` function.
pub fn handle_end_event<W: Write>(
    writer: &mut EventWriter<W>,
    waypoint: Waypoint,
    mut current_search: LookingFor,
    drop: bool,
    mut wp: usize,
    name: &str,
) -> Result<(Waypoint, LookingFor, usize), Box<dyn Error>> {
    // 4. Find closing of `name`
    if matches!(current_search, LookingFor::ClosingName) && name == "name" {
        current_search = LookingFor::OpeningStyleUrl;
    }

    // 7. Find closing of `styleUrl`
    if matches!(current_search, LookingFor::ClosingStyleUrl) && name == "styleUrl" {
        current_search = LookingFor::OpeningCoordinates;
    }

    // 10. Find closing of `coordinates`
    if matches!(current_search, LookingFor::ClosingCoordinates) && name == "coordinates" {
        current_search = LookingFor::ClosingPlacemark;
    }

    // 11. Find closing of `Placemark`
    if matches!(current_search, LookingFor::ClosingPlacemark) && name == "Placemark" {
        if !drop {
            super::write_waypoint(writer, &waypoint)?;
            wp += 1;
        }
        current_search = LookingFor::OpeningPlacemark;
    }

    Ok((waypoint, current_search, wp))
}
