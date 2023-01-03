use std::{
    error::Error,
    fs::File,
    io::{BufReader, Write},
    path::PathBuf,
    result,
};

use xml::{
    reader::EventReader,
    writer::{EmitterConfig, EventWriter, Result},
};

// TODO Idea: Use `output: Option<PathBuf>` to handle writing to a file or stdout.
pub fn transform(input: PathBuf, output: PathBuf) -> result::Result<(), Box<dyn Error>> {
    use xml::reader::XmlEvent;

    // Create the reader object.
    let input_file = File::open(input)?;
    let input_file = BufReader::new(input_file);
    let parser = EventReader::new(input_file);

    // Create the writer object.
    let mut output_file = File::create(output)?;
    let mut writer = EmitterConfig::new()
        .perform_indent(true)
        .indent_string("\t")
        .create_writer(&mut output_file);

    {
        // Use this sub-scope to process the xml.
        for element in parser {
            match element {
                Ok(XmlEvent::Characters(line)) => {
                    write_event(&mut writer, EventType::Content, &line)?;
                }
                // TODO: Determine if handling attributes is actually necessary.
                Ok(XmlEvent::StartElement { name, .. }) => {
                    let name = name.to_string();
                    let name = simplify_name(&name);

                    write_event(&mut writer, EventType::OpeningElement, name)?;
                }
                Ok(XmlEvent::EndElement { name }) => {
                    let name = name.to_string();
                    let name = simplify_name(&name);

                    write_event(&mut writer, EventType::ClosingElement, name)?;
                }
                Err(e) => {
                    // TODO: Determine if there's a better way to handle this error.
                    eprintln!("Error: {}", e);
                    break;
                }
                _ => {}
            }
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

            let name = line[0].get(1..).unwrap();

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
        EventType::Content => XmlEvent::characters(&line).into(),
    };

    w.write(event)
}

/// Takes a string that would look something like
///
/// `{http:://www.opengis.net/kml/2.2}coordinates`
///
/// and removes the link by splitting the &str at the '}' and returning the element to the right.
///
/// `coordinates`
fn simplify_name<'a>(name: &'a str) -> &'a str {
    let is_split = match name.find('}') {
        Some(_) => 1,
        None => 0,
    };

    let split_name: Vec<&str> = name.split('}').collect();

    split_name[is_split]
}
