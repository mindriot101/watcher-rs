#![allow(unreachable_code)]
#![recursion_limit = "1024"]

#[macro_use]
extern crate error_chain;
extern crate clap;
extern crate notify;

use std::sync::mpsc::channel;
use std::path::{Path, PathBuf};
use std::process::Command;
use notify::{Watcher, RecursiveMode, watcher, DebouncedEvent};
use std::time::Duration;

mod errors {
    // Create the Error, ErrorKind, ResultExt, and Result types
    error_chain!{}
}

use clap::{Arg, App};
use errors::*;

fn main() {
    let matches = App::new("watcher")
        .version("0.0.1")
        .author("Simon Walker")
        .about("Watches things")
        .arg(Arg::with_name("file")
             .help("File to watch")
             .long("file")
             .multiple(true)
             .short("f")
             .takes_value(true)
             .required(true))
        .arg(Arg::with_name("command")
             .help("Command to run")
             .long("command")
             .short("c")
             .takes_value(true)
             .required(true))
        .get_matches();

    let files: Vec<_> = matches.values_of("file").unwrap().map(|s| {
        let p = Path::new(s);
        p.canonicalize().unwrap()
    }).collect();
    let command = matches.value_of("command").unwrap();

    if let Err(ref e) = run(files, command) {
        use std::io::Write;
        let stderr = &mut ::std::io::stderr();
        let errmsg = "Error writing to stderr";

        writeln!(stderr, "error: {}", e).expect(errmsg);

        for e in e.iter().skip(1) {
            writeln!(stderr, "caused by: {}", e).expect(errmsg);
        }

        // The backtrace is not always generated. Try to run this example
        // with `RUST_BACKTRACE=1`.
        if let Some(backtrace) = e.backtrace() {
            writeln!(stderr, "backtrace: {:?}", backtrace).expect(errmsg);
        }

        ::std::process::exit(1);
    }
}

fn run(files: Vec<PathBuf>, command: &str) -> Result<()> {
    let (tx, rx) = channel();
    let mut watcher = watcher(tx, Duration::new(0, 250_000_000)).unwrap();
    for file in &files {
        watcher.watch(file, RecursiveMode::Recursive).unwrap();
    }

    loop {
        match rx.recv() {
            Ok(DebouncedEvent::Write(path)) => {
                let abs_path = path.canonicalize().unwrap();
                if files.contains(&abs_path) {
                    run_command(command).unwrap();
                }
            },
            _ => {},
        }
    }
    Ok(())
}

fn run_command(command: &str) -> Result<()> {
    let mut args = command.split_whitespace();
    let program = args.next().unwrap();
    let arguments: Vec<&str> = args.collect();
    let status = Command::new(program)
        .args(&arguments)
        .status()
        .expect("Could not start command");
    if status.success() {
        Ok(())
    } else {
        Err(format!("Status value: {:?}", status).into())
    }
}
