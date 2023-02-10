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

mod handlers;
use handlers::{LookingFor, Waypoint};

/// Represents an airport by it's ICAO code and runway.
pub struct Airport {
    pub ident: String,
    pub runway: Option<String>,
}

// TODO Idea: Use `output: Option<PathBuf>` to handle writing to a file or stdout.
/// This function will use the Placemarks in the .kml file to write a route using waypoints for the
/// .fgfp file.
///
/// # Errors
/// This function can fail if trying to write invalid xml or other io errors.
pub fn transform_route<W: Write, R: Read>(
    parser: EventReader<R>,
    writer: &mut EventWriter<W>,
    departure: &Option<Airport>,
    destination: &Option<Airport>,
) -> result::Result<(), Box<dyn Error>> {
    use xml::reader::XmlEvent;

    let mut current_search = LookingFor::OpeningPlacemark;

    // The waypoint information
    let mut wp = 0;
    let mut drop = false;
    let mut waypoint = Waypoint {
        number: wp,
        ident: String::new(),
        lon: 0f64,
        lat: 0f64,
        altitude: 0,
    };

    super::write_event(writer, EventType::OpeningElement, "route")?;

    if let Some(ap) = departure {
        write_ap_waypoint(writer, ap, true, wp)?;
        wp += 1;
    }

    for element in parser {
        match element {
            Ok(XmlEvent::StartElement { name, .. }) => {
                let name = name.to_string();
                let name = simplify_name(&name);

                (waypoint, current_search, drop) =
                    handlers::handle_start_event(waypoint, current_search, drop, wp, name);
            }
            Ok(XmlEvent::Characters(line)) => {
                (waypoint, current_search, drop) = handlers::handle_characters_event(
                    waypoint,
                    current_search,
                    drop,
                    line,
                    departure,
                    destination,
                );
            }
            Ok(XmlEvent::EndElement { name }) => {
                let name = name.to_string();
                let name = simplify_name(&name);

                (waypoint, current_search, wp) =
                    handlers::handle_end_event(writer, waypoint, current_search, drop, wp, name)?;
            }
            Err(e) => {
                // Split the error message to simplify it.
                let msg = e.to_string();
                let msg: Vec<&str> = msg
                    .split_whitespace()
                    .map(simplify_name)
                    .collect();

                // Make a string from the vector.
                let mut message = String::new();
                for m in msg {
                    message.push_str(format!("{m} ").as_str());
                }

                // Print error and exit loop.
                eprintln!("\x1B[01;33mError on line\x1B[00m: {message}");
                break;
            }
            _ => {}
        }
    }

    if let Some(ap) = destination {
        write_ap_waypoint(writer, ap, false, wp)?;
    }

    super::write_event(writer, EventType::ClosingElement, "route")?;

    Ok(())
}

/// Function that takes a waypoint and writes it to the .fgfp file
#[rustfmt::skip]
fn write_waypoint<W: Write>(writer: &mut EventWriter<W>, wp: &Waypoint) -> xml::writer::Result<()> {
    let number = if wp.number > 0 {
        format!(" n={}", wp.number)
    } else {
        String::new()
    };
    let opening = format!("wp{number}");

    super::write_event(writer, EventType::OpeningElement, &opening)?;

    super::write_event(writer, EventType::OpeningElement, "type type=string")?;
    super::write_event(writer, EventType::Content, "basic")?;
    super::write_event(writer, EventType::ClosingElement, "type")?;

    super::write_event(writer, EventType::OpeningElement, "alt-restrict type=string")?;
    super::write_event(writer, EventType::Content, "at")?;
    super::write_event(writer, EventType::ClosingElement, "alt-restrict")?;

    super::write_event(writer, EventType::OpeningElement, "altitude-ft type=double")?;
    super::write_event(writer, EventType::Content, format!("{:.0}", &wp.altitude).as_str())?;
    super::write_event(writer, EventType::ClosingElement, "altitude-ft")?;

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

/// Function that takes an airport waypoint and writes it to the .fgfp file
fn write_ap_waypoint<W: Write>(
    writer: &mut EventWriter<W>,
    airport: &Airport,
    is_departure: bool,
    wp_counter: usize,
) -> xml::writer::Result<()> {
    let number = if wp_counter > 0 {
        format!(" n={wp_counter}")
    } else {
        String::new()
    };
    let opening = format!("wp{number}");

    super::write_event(writer, EventType::OpeningElement, &opening)?;

    super::write_event(writer, EventType::OpeningElement, "type type=string")?;
    super::write_event(writer, EventType::Content, "runway")?;
    super::write_event(writer, EventType::ClosingElement, "type")?;

    if is_departure {
        super::write_event(writer, EventType::OpeningElement, "departure type=bool")?;
        super::write_event(writer, EventType::Content, "true")?;
        super::write_event(writer, EventType::ClosingElement, "departure")?;
    } else {
        // It's destination
        super::write_event(writer, EventType::OpeningElement, "approach type=bool")?;
        super::write_event(writer, EventType::Content, "true")?;
        super::write_event(writer, EventType::ClosingElement, "approach")?;
    }

    if let Some(runway) = &airport.runway {
        super::write_event(writer, EventType::OpeningElement, "ident type=string")?;
        super::write_event(writer, EventType::Content, runway)?;
        super::write_event(writer, EventType::ClosingElement, "ident")?;
    }

    super::write_event(writer, EventType::OpeningElement, "icao type=string")?;
    super::write_event(writer, EventType::Content, &airport.ident)?;
    super::write_event(writer, EventType::ClosingElement, "icao")?;

    super::write_event(writer, EventType::ClosingElement, "wp")?;
    Ok(())
}

/// Internal function that takes a [`&str`](str) that would look something like
/// `{http:://www.opengis.net/kml/2.2}coordinates` and removes the link by splitting the &str at the
/// '}' and returning the element to the right: `coordinates`.
fn simplify_name(name: &str) -> &str {
    let is_split = match name.find('}') {
        Some(_) => 1,
        None => 0,
    };

    let split_name: Vec<&str> = name.split('}').collect();

    split_name[is_split]
}
