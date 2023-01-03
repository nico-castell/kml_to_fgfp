use std::{
    error::Error,
    io::{Read, Write},
    result,
};

// Export these structs so callers needn't have to declare xml-rs as a dependency.
pub use xml::{reader::EventReader, writer::EmitterConfig};

use xml::writer::{EventWriter, Result};

// TODO Idea: Use `output: Option<PathBuf>` to handle writing to a file or stdout.
pub fn transform_route<W: Write, R: Read>(
    parser: EventReader<R>,
    writer: &mut EventWriter<W>,
) -> result::Result<(), Box<dyn Error>> {
    use xml::reader::XmlEvent;

    for element in parser {
        match element {
            Ok(XmlEvent::Characters(line)) => {
                write_event(writer, EventType::Content, &line)?;
            }
            // TODO: Determine if handling attributes is actually necessary.
            Ok(XmlEvent::StartElement { name, .. }) => {
                let name = name.to_string();
                let name = simplify_name(&name);

                write_event(writer, EventType::OpeningElement, name)?;
            }
            Ok(XmlEvent::EndElement { name }) => {
                let name = name.to_string();
                let name = simplify_name(&name);

                write_event(writer, EventType::ClosingElement, name)?;
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

/// Internal enum to help [`transform`](transform) communicate instructions to
/// [`write_event`](write_event).
enum EventType {
    OpeningElement,
    ClosingElement,
    Content,
}

fn write_event<W: Write>(w: &mut EventWriter<W>, event_type: EventType, line: &str) -> Result<()> {
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

    w.write(event)
}

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

/// Write the destination and arrival airports to the .fgfp's xml tree. The airport codes should
/// have [ICAO codes](https://en.wikipedia.org/wiki/List_of_airports_by_ICAO_code:_A).
///
/// Codes being given as YSSY/34L, for example, will be interpreted as:
/// - Airport code: YSSY
/// - Runway: 34L
pub fn write_airports<W: Write>(
    writer: &mut EventWriter<W>,
    dep: Option<String>,
    arr: Option<String>,
) -> Result<()> {
    Ok(())
}

/// Write the end of the .fgfp's xml tree.
#[rustfmt::skip]
pub fn close_tree<W: Write>(writer: &mut EventWriter<W>) -> Result<()> {
    write_event(writer, EventType::ClosingElement, "PropertyList")?;

    Ok(())
}

/// Takes a [`&str`](str) that would look something like
///
/// `{http:://www.opengis.net/kml/2.2}coordinates`
///
/// and removes the link by splitting the &str at the '}' and returning the element to the right.
///
/// `coordinates`
fn simplify_name(name: &str) -> &str {
    let is_split = match name.find('}') {
        Some(_) => 1,
        None => 0,
    };

    let split_name: Vec<&str> = name.split('}').collect();

    split_name[is_split]
}
