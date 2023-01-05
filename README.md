# Make FlightGear flight plans with Google Earth's `.kml`s
[![Commits since last release](https://img.shields.io/github/commits-since/nico-castell/kml_to_fgfp/latest?label=Commits%20since%20last%20release&color=informational&logo=Git&logoColor=white&style=flat-square)](https://github.com/nico-castell/kml_to_fgfp/commits)
[![Crates version](https://img.shields.io/crates/v/kml_to_fgfp?color=informational&label=Crate%20version&logo=Rust&logoColor=white&style=flat-square)](https://crates.io/crates/kml_to_fgfp/versions)
[![License](https://img.shields.io/github/license/nico-castell/kml_to_fgfp?label=License&color=informational&logo=Open%20Source%20Initiative&logoColor=white&style=flat-square)](LICENSE)

The popular website [SimBrief](https://www.simbrief.com/) can provide you with comprehensive flight
documentation for simulations. It gives you the option to **download your flight plan as a Google
Earth .kml file**, but no option to download a FlightGear flight plan.

This tool allows you transform this .kml file into a .fgfp, a FlightGear flight plan.

## Installation
To install this application you will need to have **cargo** from the Rust language. If you don't
have it, you can refer to the installation instructions
[here](https://www.rust-lang.org/learn/get-started).

Then you simply run the following command in your terminal:
```
$ cargo install kml_to_fgfp
```

## Usage of the executable binary
The binary will, for now, need at least two arguments:

- The **first argument** refers to the **source file**, expected to be a .kml file.
- The **second argument** refers to the destination file, meaning the generated .fgfp file.

  Keep in mind that if the .fgfp file already exists, it will be overwritten.

Here's an example:
```
$ kml_to_fgfp YSSYSAEZ.kml YSSYSAEZ.fgfp
```

Don't worry if you don't see output in your terminal, that's the expected behavior.

You can also specify the departure and destination airports, which will complete the flight plan
with the airport waypoints:

```
$ kml_to_fgfp YSSYSAEZ.kml YSSYSAEZ.fgfp YSSY/34L SAEZ/11
```

---

The program can output a warning when it detects invalid data in the .kml file (maybe it was
manually edited and there's a mistake).

```
$ kml_to_fgfp YSSYSAEZ.kml YSSYSAEZ.fgfp
Dropping ARSOT waypoint: invalid float literal
```

In this example, the program will not generate a waypoint for the ARSOT navaid because it found an
error in the data.

---

There's also a help menu that can be accessed with the `--help` and `-h` arguments.
```
$ kml_to_fgfp --help
Usage:
      kml_to_fgfp INPUT OUTPUT [DEPARTURE_AIRPORT] [DESTINATION AIRPORT]

INPUT is the Google Earth (.kml) file.

OUTPUT is the name of the generated FlightGear flight plan (.fgfp) file.

[DEPARTURE_AIRPORT] is an optional argument detailing the departure airport's
ICAO designation. It would look something like `YSSY`. You can also type a `/`
to add a specific runway, so it would look like `YSSY/34L`.

[DESTINATION_AIRPORT] is an optional argument detailing the destination
airport's ICAO designation. It would look something like `SAEZ`. You can also
type a `/` to add a specific runway, so it would look like `SAEZ/11`.

Version: 0.1.0, MIT License
```

## Usage of the library API
1. Create an `EventWriter`, it will be used to write to the output file.
   ```rust
   let mut output_file = File::create(output_filepath)?;
   let mut writer = EmitterConfig::new()
       .perform_indent(true)
       .create_writer(&mut output_file);
   ```

2. Write the beginning of the .fgfp xml tree using the `write_start_of_tree` function.
   ```rust
   kml_to_fgfp::write_start_of_tree(&mut writer)?;
   ```

3. Create 2 `Option<kml_to_fgfp::Airport>` setting the value to `None` if no departure or
   no destination airports are in the flight plan.

   Then call the `write_airports` function, passing the previous options as arguments.

   ```rust
   let departure = kml_to_fgfp::Airport {
       ident: String::from("YSSY"),
       runway: Some(String::from("34L")),
   };

   let destination = kml_to_fgfp::Airport {
       ident: String::from("SAEZ"),
       runway: Some(String::from("11")),
   };

   kml_to_fgfp::write_airports(&mut writer, &departure, &destination)?;
   ```

4. Create an `EventReader`, it will be used to read the .kml file.

   ```rust
   let input_file = File::open(input_filepath)?;
   let input_file = BufReader::new(input_file);
   let parser = EventReader::new(input_file);
   ```

5. Call the `transform_route` function, which will need the xml `EventReader`, `EventWriter`, and 2
   airport options. This function creates the .fgfp's route using waypoints with the information in
   the .kml file.

   ```rust
   kml_to_fgfp::transform_route(
       parser,
       &mut writer,
       &departure,
       &destination,
   )?;
   ```

6. Close the xml tree by calling the `close_tree` function.

   ```rust
   kml_to_fgfp::close_tree(&mut writer)?;
   ```

If you want an example you can refer to the `run` function in the [`runner`](src/runner.rs) module.

## About
This program and this repository are available under an [MIT License](LICENSE).
