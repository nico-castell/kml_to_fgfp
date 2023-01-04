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

// TODO Idea: Use `output: Option<PathBuf>` to handle writing to a file or stdout.
pub fn transform_route<W: Write, R: Read>(
    parser: EventReader<R>,
    writer: &mut EventWriter<W>,
) -> result::Result<(), Box<dyn Error>> {
    use xml::reader::XmlEvent;

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

    let mut current_search = LookingFor::OpeningPlacemark;

    // The waypoint information
    let mut wp = 0;
    let mut action = Action::Push;
    let mut waypoint = Waypoint {
        number: wp,
        ident: String::from(""),
        lon: 0f64,
        lat: 0f64,
        altitude: 0,
    };

    for element in parser {
        match element {
            Ok(XmlEvent::StartElement { name, .. }) => {
                let name = name.to_string();
                let name = simplify_name(&name);

                // 1. Find opening of `Placemark`
                if matches!(current_search, LookingFor::OpeningPlacemark) && name == "Placemark" {
                    waypoint.number = wp;
                    wp += 1;
                    current_search = LookingFor::OpeningName;
                    action = Action::Push;
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
                if matches!(current_search, LookingFor::OpeningCoordinates) && name == "coordinates"
                {
                    current_search = LookingFor::ContentCoordinates;
                }
            }
            Ok(XmlEvent::Characters(line)) => {
                // 3. Find contents of `name`
                if matches!(current_search, LookingFor::ContentName) {
                    waypoint.ident = String::from(&line);
                    current_search = LookingFor::ClosingName;
                }

                // 6. Find contents of `styleUrl`
                if matches!(current_search, LookingFor::ContentStyleUrl) {
                    if line != "#FixMark" {
                        action = Action::Drop;

                        // We found that this Placemark is not part of the route, so we avoid
                        // further processing of the waypoint.
                        current_search = LookingFor::ClosingPlacemark;

                        continue;
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
                        action = Action::Drop;
                    }

                    current_search = LookingFor::ClosingCoordinates;
                }
            }
            Ok(XmlEvent::EndElement { name }) => {
                let name = name.to_string();
                let name = simplify_name(&name);

                // 4. Find closing of `name`
                if matches!(current_search, LookingFor::ClosingName) && name == "name" {
                    current_search = LookingFor::OpeningStyleUrl;
                }

                // 7. Find closing of `styleUrl`
                if matches!(current_search, LookingFor::ClosingStyleUrl) && name == "styleUrl" {
                    current_search = LookingFor::OpeningCoordinates;
                }

                // 10. Find closing of `coordinates`
                if matches!(current_search, LookingFor::ClosingCoordinates) && name == "coordinates"
                {
                    current_search = LookingFor::ClosingPlacemark;
                }

                // 11. Find closing of `Placemark`
                if matches!(current_search, LookingFor::ClosingPlacemark) && name == "Placemark" {
                    eprintln!("{:?}, {:#?}", action, waypoint);
                    current_search = LookingFor::OpeningPlacemark;
                }
            }
            Err(e) => {
                // TODO: Determine if there's a better way to handle this error.
                eprintln!("Error: {}", e);
                break;
            }
            _ => {}
        }
    }

    Ok(())
}

/// Used to write a waypoint to the .fgfp file.
#[derive(Debug)]
struct Waypoint {
    number: usize,
    ident: String,
    lon: f64,
    lat: f64,
    altitude: usize,
}

/// If the styleUrl matches `#FixMark`, it should be pushed. If it matches `#RouteMark` it should be
/// dropped.
#[derive(Debug)]
enum Action {
    Push,
    Drop,
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
