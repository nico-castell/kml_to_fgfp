use std::{
    error::Error,
    io::{Read, Write},
    result,
};

// Export these structs so callers needn't have to declare xml-rs as a dependency.
pub use xml::{reader::EventReader, writer::EmitterConfig};

use xml::writer::{EventWriter, Result};

// # Step 1: Start of tree
// #######################

/// Write the start of the .fgfp's xml tree. AKA the version, flight-rules, flight-type and
/// estimated duration.
#[rustfmt::skip]
pub fn write_start_of_tree<W: Write>(writer: &mut EventWriter<W>) -> Result<()> {
    write_event(writer, EventType::OpeningElement, "PropertyList")?;

    write_event(writer, EventType::OpeningElement, "version type=int")?;
    write_event(writer, EventType::Content, "2")?;
    write_event(writer, EventType::ClosingElement, "version")?;

    write_event(writer, EventType::OpeningElement, "flight-rules type=string")?;
    write_event(writer, EventType::Content, "V")?;
    write_event(writer, EventType::ClosingElement, "flight-rules")?;

    write_event(writer, EventType::OpeningElement, "flight-type type=string")?;
    write_event(writer, EventType::Content, "X")?;
    write_event(writer, EventType::ClosingElement, "flight-type")?;

    write_event(writer, EventType::OpeningElement, "estimated-duration-minutes type=int")?;
    write_event(writer, EventType::Content, "0")?;
    write_event(writer, EventType::ClosingElement, "estimated-duration-minutes")?;

    Ok(())
}

// # Step: Airports
// ################

/// Write the destination and arrival airports to the .fgfp's xml tree. The airport codes should
/// have [ICAO codes](https://en.wikipedia.org/wiki/List_of_airports_by_ICAO_code:_A).
///
/// Codes being given as YSSY/34L, for example, will be interpreted as:
/// - Airport code: YSSY
/// - Runway: 34L
pub fn write_airports<W: Write>(
    writer: &mut EventWriter<W>,
    departure: Option<String>,
    arrival: Option<String>,
) -> Result<()> {
    if let Some(code) = departure {
        write_event(writer, EventType::OpeningElement, "departure")?;
        write_airport_details(writer, code)?;
        write_event(writer, EventType::ClosingElement, "departure")?;
    }

    if let Some(code) = arrival {
        write_event(writer, EventType::OpeningElement, "destination")?;
        write_airport_details(writer, code)?;
        write_event(writer, EventType::ClosingElement, "destination")?;
    }

    Ok(())
}

/// Internal function to write the details of an airport.
fn write_airport_details<W: Write>(writer: &mut EventWriter<W>, code: String) -> Result<()> {
    if code.contains('/') {
        let elements: Vec<&str> = code.split('/').collect();

        write_event(writer, EventType::OpeningElement, "airport type=string")?;
        write_event(writer, EventType::Content, elements[0])?;
        write_event(writer, EventType::ClosingElement, "airport")?;

        write_event(writer, EventType::OpeningElement, "runway type=string")?;
        write_event(writer, EventType::Content, elements[1])?;
        write_event(writer, EventType::ClosingElement, "runway")?;
    } else {
        write_event(writer, EventType::OpeningElement, "airport type=string")?;
        write_event(writer, EventType::Content, &code)?;
        write_event(writer, EventType::ClosingElement, "airport")?;
    }

    Ok(())
}

// # Step 3: The route
// ###################

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

    for element in parser {
        match element {
            Ok(XmlEvent::Characters(line)) => {
                // 3. Find contents of `name`
                if matches!(current_search, LookingFor::ContentName) {
                write_event(writer, EventType::Content, &line)?;
                    current_search = LookingFor::ClosingName;
            }

                // 6. Find contents of `styleUrl`
                if matches!(current_search, LookingFor::ContentStyleUrl) {
                    write_event(writer, EventType::Content, &line)?;
                    current_search = LookingFor::ClosingStyleUrl;
                }

                // 9. Find contents of `coordinates`
                if matches!(current_search, LookingFor::ContentCoordinates) {
                    write_event(writer, EventType::Content, &line)?;
                    current_search = LookingFor::ClosingCoordinates;
                }
            }
            Ok(XmlEvent::StartElement { name, .. }) => {
                let name = name.to_string();
                let name = simplify_name(&name);

                // 1. Find opening of `Placemark`
                if matches!(current_search, LookingFor::OpeningPlacemark) && name == "Placemark" {
                    write_event(writer, EventType::OpeningElement, "Placemark")?;
                    current_search = LookingFor::OpeningName;
                }

                // 2. Find opening of `name`
                if matches!(current_search, LookingFor::OpeningName) && name == "name" {
                    write_event(writer, EventType::OpeningElement, "name")?;
                    current_search = LookingFor::ContentName;
                }

                // 5. Find opening of `styleUrl`
                if matches!(current_search, LookingFor::OpeningStyleUrl) && name == "styleUrl" {
                    write_event(writer, EventType::OpeningElement, "styleUrl")?;
                    current_search = LookingFor::ContentStyleUrl;
                }

                // 8. Find opening of `coordinates`
                if matches!(current_search, LookingFor::OpeningCoordinates) && name == "coordinates" {
                    write_event(writer, EventType::OpeningElement, "coordinates")?;
                    current_search = LookingFor::ContentCoordinates;
                }
            }
            Ok(XmlEvent::EndElement { name }) => {
                let name = name.to_string();
                let name = simplify_name(&name);

                // 4. Find closing of `name`
                if matches!(current_search, LookingFor::ClosingName) && name == "name" {
                    write_event(writer, EventType::ClosingElement, "Placemark")?;
                    current_search = LookingFor::OpeningStyleUrl;
                }

                // 7. Find closing of `styleUrl`
                if matches!(current_search, LookingFor::ClosingStyleUrl) && name == "styleUrl" {
                    write_event(writer, EventType::ClosingElement, "styleUrl")?;
                    current_search = LookingFor::OpeningCoordinates;
                }

                // 10. Find closing of `coordinates`
                if matches!(current_search, LookingFor::ClosingCoordinates) && name == "coordinates" {
                    write_event(writer, EventType::ClosingElement, "coordinates")?;
                    current_search = LookingFor::ClosingPlacemark;
                }

                // 11. Find closing of `Placemark`
                if matches!(current_search, LookingFor::ClosingPlacemark) && name == "Placemark" {
                    write_event(writer, EventType::ClosingElement, "Placemark")?;
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

// # Step 4: CLosing tree
// ######################

/// Write the end of the .fgfp's xml tree.
#[rustfmt::skip]
pub fn close_tree<W: Write>(writer: &mut EventWriter<W>) -> Result<()> {
    write_event(writer, EventType::ClosingElement, "PropertyList")?;

    Ok(())
}

// # Global: Writing the .fgfp
// ###########################

/// Internal enum to help [`transform`](transform) communicate instructions to
/// [`write_event`](write_event).
enum EventType {
    OpeningElement,
    ClosingElement,
    Content,
}

/// Internal function that writes an xml event to the output file.
///
/// Args:
/// - `writer` - An xml-rs [`EventWriter<>`](xml::writer::EventWriter).
/// - `event_type` - An [`EventType`](EventType) enum specifying the type of xml element to write
///   (opening & closing tags, or content)
/// - `line` - A [`String`](String), containing the text to write.
///
/// # Errors
/// This function can return an error when trying to write invalid xml, or other io errors.
fn write_event<W: Write>(writer: &mut EventWriter<W>, event_type: EventType, line: &str) -> Result<()> {
    use xml::writer::XmlEvent;

    let line = line.trim();

    let event: XmlEvent = match event_type {
        EventType::OpeningElement => {
            let line: Vec<&str> = line.split(' ').collect();

            let name = line[0];

            let mut event = XmlEvent::start_element(name);

            for code in line {
                let elements = code
                    .split('=')
                    .zip(code.split('=').skip(1))
                    .collect::<Vec<_>>();

                for attributes in elements {
                    let (name, value) = attributes;
                    event = event.attr(name, value);
                }
            }

            event.into()
        }
        EventType::ClosingElement => XmlEvent::end_element().into(),
        EventType::Content => XmlEvent::characters(line),
    };

    writer.write(event)
}
