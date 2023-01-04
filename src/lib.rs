use std::io::Write;

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
    departure: &Option<Airport>,
    destination: &Option<Airport>,
) -> Result<()> {
    if let Some(airport) = departure {
        write_event(writer, EventType::OpeningElement, "departure")?;
        write_airport_details(writer, &airport.ident, &airport.runway)?;
        write_event(writer, EventType::ClosingElement, "departure")?;
    }

    if let Some(airport) = destination {
        write_event(writer, EventType::OpeningElement, "destination")?;
        write_airport_details(writer, &airport.ident, &airport.runway)?;
        write_event(writer, EventType::ClosingElement, "destination")?;
    }

    Ok(())
}

/// Internal function to write the details of an airport.
fn write_airport_details<W: Write>(
    writer: &mut EventWriter<W>,
    ident: &str,
    runway: &Option<String>,
) -> Result<()> {
    write_event(writer, EventType::OpeningElement, "airport type=string")?;
    write_event(writer, EventType::Content, ident)?;
    write_event(writer, EventType::ClosingElement, "airport")?;

    if let Some(runway) = runway {
        write_event(writer, EventType::OpeningElement, "runway type=string")?;
        write_event(writer, EventType::Content, runway)?;
        write_event(writer, EventType::ClosingElement, "runway")?;
    }

    write_event(writer, EventType::OpeningElement, "airport type=string")?;
    write_event(writer, EventType::Content, ident)?;
    write_event(writer, EventType::ClosingElement, "airport")?;

    Ok(())
}

// # Step 3: The route
// ###################

// This step was moved to it's own module because of it's size.
mod route;
pub use route::{transform_route, Airport};

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
fn write_event<W: Write>(
    writer: &mut EventWriter<W>,
    event_type: EventType,
    line: &str,
) -> Result<()> {
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
