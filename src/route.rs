use std::{
    error::Error,
    io::{Read, Write},
    result,
};

pub use xml::{
    reader::EventReader,
    writer::{EmitterConfig, EventWriter},
};

use super::EventType;

/// Represents an airport by it's ICAO code and runway.
pub struct Airport{
    pub ident: String,
    pub runway: Option<String>
}

// TODO Idea: Use `output: Option<PathBuf>` to handle writing to a file or stdout.
pub fn transform_route<W: Write, R: Read>(
    parser: EventReader<R>,
    writer: &mut EventWriter<W>,
) -> result::Result<(), Box<dyn Error>> {
    use xml::reader::XmlEvent;

    let mut current_search = LookingFor::OpeningPlacemark;

    // The waypoint information
    let mut wp = 0;
    let mut drop = false;
    let mut waypoint = Waypoint {
        number: wp,
        ident: String::from(""),
        lon: 0f64,
        lat: 0f64,
        altitude: 0,
    };

    super::write_event(writer, EventType::OpeningElement, "route")?;

    for element in parser {
        match element {
            Ok(XmlEvent::StartElement { name, .. }) => {
                let name = name.to_string();
                let name = simplify_name(&name);

                (waypoint, current_search, drop) =
                    handle_start_event(waypoint, current_search, drop, wp, name);
            }
            Ok(XmlEvent::Characters(line)) => {
                (waypoint, current_search, drop) =
                    handle_characters_event(waypoint, current_search, drop, line);
            }
            Ok(XmlEvent::EndElement { name }) => {
                let name = name.to_string();
                let name = simplify_name(&name);

                (waypoint, current_search, wp) =
                    handle_end_event(writer, waypoint, current_search, drop, wp, name)?;
            }
            Err(e) => {
                // TODO: Determine if there's a better way to handle this error.
                eprintln!("Error: {}", e);
                break;
            }
            _ => {}
        }
    }

    super::write_event(writer, EventType::ClosingElement, "route")?;

    Ok(())
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
enum LookingFor {
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
fn handle_start_event(
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
fn handle_characters_event(
    mut waypoint: Waypoint,
    mut current_search: LookingFor,
    mut drop: bool,
    line: String,
) -> (Waypoint, LookingFor, bool) {
    // 3. Find contents of `name`
    if matches!(current_search, LookingFor::ContentName) {
        waypoint.ident = String::from(&line);
        current_search = LookingFor::ClosingName;
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

        let mut message = String::from("");

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
        waypoint.altitude = match data[2].parse() {
            Ok(d) => d,
            Err(e) => {
                message = e.to_string();
                0usize
            }
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
fn handle_end_event<W: Write>(
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
            write_waypoint(writer, &waypoint)?;
            wp += 1;
        }
        current_search = LookingFor::OpeningPlacemark;
    }

    Ok((waypoint, current_search, wp))
}

/// Used to write a waypoint to the .fgfp file.
struct Waypoint {
    number: usize,
    ident: String,
    lon: f64,
    lat: f64,
    altitude: usize,
}

/// Function that takes a waypoint and writes it to the .fgfp file
#[rustfmt::skip]
fn write_waypoint<W: Write>(writer: &mut EventWriter<W>, wp: &Waypoint) -> xml::writer::Result<()> {
    let number = if wp.number > 0 {
        format!(" n={}", wp.number)
    } else {
        format!("")
    };
    let opening = format!("wp{}", number);

    super::write_event(writer, EventType::OpeningElement, &opening)?;

    super::write_event(writer, EventType::OpeningElement, "type type=string")?;
    super::write_event(writer, EventType::Content, "basic")?;
    super::write_event(writer, EventType::ClosingElement, "type")?;

    super::write_event(writer, EventType::OpeningElement, "ident type=string")?;
    super::write_event(writer, EventType::Content, &wp.ident)?;
    super::write_event(writer, EventType::ClosingElement, "ident")?;

    super::write_event(writer, EventType::OpeningElement, "lon type=double")?;
    super::write_event(writer, EventType::Content, format!("{:.6}", wp.lon).as_str())?;
    super::write_event(writer, EventType::ClosingElement, "lon")?;

    super::write_event(writer, EventType::OpeningElement, "lat type=double")?;
    super::write_event(writer, EventType::Content, format!("{:.6}", wp.lat).as_str())?;
    super::write_event(writer, EventType::ClosingElement, "lat")?;

    super::write_event(writer, EventType::ClosingElement, "wp")?;

    Ok(())
}

/// Internal function that takes a [`&str`](str) that would look something like
/// `{http:://www.opengis.net/kml/2.2}coordinates` and removes the link by splitting the &str at the
/// '}' and returning the element to the right: `coordinates`
fn simplify_name(name: &str) -> &str {
    let is_split = match name.find('}') {
        Some(_) => 1,
        None => 0,
    };

    let split_name: Vec<&str> = name.split('}').collect();

    split_name[is_split]
}
