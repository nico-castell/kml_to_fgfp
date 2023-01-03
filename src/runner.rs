use std::{error::Error, path::PathBuf};

/// The library crate to perform the actual operations
use kml_to_fgfp;

/// The config for the transformation of the .kml file into .fgfp. Taken as an argument by the
/// [`run`](run) function.
///
/// You can create an instance of this struct using [`Config::build()`](Config::build).
pub struct Config {
    input: PathBuf,
    output: PathBuf,
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
            None => return Err("Didn't get an input file".into()),
        };

        let output = match args.next() {
            Some(path) => PathBuf::from(path),
            None => return Err("Didn't get a destination directory".into()),
        };

        Ok(Config { input, output })
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
    kml_to_fgfp::transform(config.input, config.output)?;

    Ok(())
}
