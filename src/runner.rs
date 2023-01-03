use std::{error::Error, fs::File, io::BufReader, path::PathBuf};

/// The library crate to perform the actual operations
use kml_to_fgfp::{self, EmitterConfig, EventReader};

/// The config for the transformation of the .kml file into .fgfp. Taken as an argument by the
/// [`run`](run) function.
///
/// You can create an instance of this struct using [`Config::build()`](Config::build).
pub struct Config {
    input: PathBuf,
    output: PathBuf,
    departure: Option<String>,
    arrival: Option<String>,
}

impl Config {
    /// Creates a `Config` type. **Assumes** the first iteration of `args` is the program name, so
    /// it's ignored.
    ///
    /// Parameter:
    /// - `args` - An iterator, meant to iterate over the binary's arguments and flags.
    ///
    /// # Errors
    ///
    /// The functions will fail if `args` does not contain input and output files, or if a flag is
    /// not recognized.
    pub fn build(mut args: impl Iterator<Item = String>) -> Result<Config, Box<dyn Error>> {
        args.next();

        let input = match args.next() {
            Some(path) => PathBuf::from(path),
            _ => return Err("Didn't get an input file".into()),
        };

        let output = match args.next() {
            Some(path) => PathBuf::from(path),
            _ => return Err("Didn't get a destination directory".into()),
        };

        let departure = args.next();

        let arrival = args.next();

        Ok(Config { input, output, departure, arrival })
    }

    /// Prints the configuration options to stderr.
    ///
    /// # Example
    ///
    /// ```
    /// # use kml_to_fgp::*;
    /// Config::print_config()
    /// ```
    pub fn print_config() {
        eprint!(
            "\
Usage:
      \x1B[01m{} INPUT OUTPUT\x1B[00m\n
Version: {}, {} License
",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
            env!("CARGO_PKG_LICENSE")
        )
    }
}

// FIXME: Add documentation after documenting kml_to_fgfp::transform.
pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
    // Create the writer object.
    let mut output_file = File::create(config.output)?;
    let mut writer = EmitterConfig::new()
        .perform_indent(true)
        .indent_string("\t")
        .create_writer(&mut output_file);

    // 1. Write the beginning of the tree.
    kml_to_fgfp::write_start_of_tree(&mut writer)?;

    // 2. Write the destination and arrival airports.
    kml_to_fgfp::write_airports(&mut writer, config.departure, config.arrival)?;

    // Create the reader object.
    let input_file = File::open(config.input)?;
    let input_file = BufReader::new(input_file);
    let parser = EventReader::new(input_file);

    // 3. Transform the route in the .kml to .fgfp.
    kml_to_fgfp::transform_route(parser, &mut writer)?;

    // 4. Close the xml tree.
    kml_to_fgfp::close_tree(&mut writer)?;

    Ok(())
}
