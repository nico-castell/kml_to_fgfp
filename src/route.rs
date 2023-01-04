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

    for element in parser {
        match element {
            Ok(XmlEvent::StartElement { name, .. }) => {
                let name = name.to_string();
                let name = simplify_name(&name);

                // 1. Find opening of `Placemark`
                if matches!(current_search, LookingFor::OpeningPlacemark) && name == "Placemark" {
                    super::write_event(writer, EventType::OpeningElement, "Placemark")?;
                    current_search = LookingFor::OpeningName;
                }

                // 2. Find opening of `name`
                if matches!(current_search, LookingFor::OpeningName) && name == "name" {
                    super::write_event(writer, EventType::OpeningElement, "name")?;
                    current_search = LookingFor::ContentName;
                }

                // 5. Find opening of `styleUrl`
                if matches!(current_search, LookingFor::OpeningStyleUrl) && name == "styleUrl" {
                    super::write_event(writer, EventType::OpeningElement, "styleUrl")?;
                    current_search = LookingFor::ContentStyleUrl;
                }

                // 8. Find opening of `coordinates`
                if matches!(current_search, LookingFor::OpeningCoordinates) && name == "coordinates"
                {
                    super::write_event(writer, EventType::OpeningElement, "coordinates")?;
                    current_search = LookingFor::ContentCoordinates;
                }
            }
            Ok(XmlEvent::Characters(line)) => {
                // 3. Find contents of `name`
                if matches!(current_search, LookingFor::ContentName) {
                    super::write_event(writer, EventType::Content, &line)?;
                    current_search = LookingFor::ClosingName;
                }

                // 6. Find contents of `styleUrl`
                if matches!(current_search, LookingFor::ContentStyleUrl) {
                    super::write_event(writer, EventType::Content, &line)?;
                    current_search = LookingFor::ClosingStyleUrl;
                }

                // 9. Find contents of `coordinates`
                if matches!(current_search, LookingFor::ContentCoordinates) {
                    super::write_event(writer, EventType::Content, &line)?;
                    current_search = LookingFor::ClosingCoordinates;
                }
            }
            Ok(XmlEvent::EndElement { name }) => {
                let name = name.to_string();
                let name = simplify_name(&name);

                // 4. Find closing of `name`
                if matches!(current_search, LookingFor::ClosingName) && name == "name" {
                    super::write_event(writer, EventType::ClosingElement, "Placemark")?;
                    current_search = LookingFor::OpeningStyleUrl;
                }

                // 7. Find closing of `styleUrl`
                if matches!(current_search, LookingFor::ClosingStyleUrl) && name == "styleUrl" {
                    super::write_event(writer, EventType::ClosingElement, "styleUrl")?;
                    current_search = LookingFor::OpeningCoordinates;
                }

                // 10. Find closing of `coordinates`
                if matches!(current_search, LookingFor::ClosingCoordinates) && name == "coordinates"
                {
                    super::write_event(writer, EventType::ClosingElement, "coordinates")?;
                    current_search = LookingFor::ClosingPlacemark;
                }

                // 11. Find closing of `Placemark`
                if matches!(current_search, LookingFor::ClosingPlacemark) && name == "Placemark" {
                    super::write_event(writer, EventType::ClosingElement, "Placemark")?;
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
