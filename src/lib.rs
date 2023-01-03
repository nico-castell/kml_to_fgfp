extern crate xml;

use std::{
    error::Error,
    fs::File,
    io::{self, BufReader, Write},
    path::PathBuf,
    result,
};

use xml::writer::{EmitterConfig, EventWriter, Result, XmlEvent};

pub fn write_event<W: Write>(w: &mut EventWriter<W>, line: String) -> Result<()> {
    let line = line.trim();

    let event: XmlEvent = if line.starts_with("+") && line.len() > 1 {
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
    } else if line.starts_with("-") {
        XmlEvent::end_element().into()
    } else {
        XmlEvent::characters(&line).into()
    };

    w.write(event)
}

pub fn transform(input: PathBuf, output: PathBuf) -> result::Result<(), Box<dyn Error>> {
    // Open and create files
    let input_file = File::open(input)?;
    let mut output_file = File::create(output)?;

    // Create the io objects
    let input = io::stdin();
    let mut output = io::stdout();
    let mut writer = EmitterConfig::new()
        .perform_indent(true)
        .create_writer(&mut output_file);

    // Supposed to process the .kml file's data to make a useful .fgfp, for now it just processes
    // user input.
    loop {
        print!("> ");
        output.flush().unwrap();
        let mut line = String::new();
        match input.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => match write_event(&mut writer, line) {
                Ok(_) => {}
                Err(e) => panic!("Write error: {}", e),
            },
            Err(e) => panic!("Input error: {}", e),
        }
    }

    Ok(())
}
