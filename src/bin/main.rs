use log::*;
use std::env;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "cport")]
struct Opt {
    /// Path of configure file
    #[structopt(parse(from_os_str), short = "-f", long = "config")]
    config: Option<PathBuf>,

    /// debug output (equal to RUST_LOG=debug)
    #[structopt(long = "--debug")]
    debug: bool,

    /// verbose (equal to RUST_LOG=info)
    #[structopt(short = "-v")]
    verbose: bool,

    /// less verbose (equal to RUST_LOG=error)
    #[structopt(short = "-q")]
    quiet: bool,
}

fn main() {
    let opt = Opt::from_args();
    if opt.debug {
        env::set_var("RUST_LOG", "debug");
    } else if opt.verbose {
        env::set_var("RUST_LOG", "info");
    } else if opt.quiet {
        env::set_var("RUST_LOG", "error");
    } else {
        env::set_var("RUST_LOG", "warn");
    }
    env_logger::init();

    // load config
    let config = cport::Configure::load(opt.config.unwrap());
    let mut builder = cport::Builder::new(config);
    match builder.create() {
        Ok(id) => {
            builder.exec(&id).unwrap();
        }
        Err(e) => {
            match e {
                shiplift::errors::Error::Fault { code, message } => {
                    warn!("Failed to create a container: reason = {}", code);
                    warn!("{}", message);
                }
                _ => {
                    error!("{:?}", e);
                }
            };
            std::process::exit(1)
        }
    }
}
