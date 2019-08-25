use failure::Fallible;
use log::*;
use std::env;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "cport",
    about = "cmake container builder",
    raw(setting = "structopt::clap::AppSettings::ColoredHelp")
)]
struct Opt {
    /// Path of configure TOML file
    #[structopt(parse(from_os_str), short = "-f", long = "config-toml")]
    config_toml: Option<PathBuf>,

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

fn build(opt: Opt) -> Fallible<()> {
    let toml = opt.config_toml.unwrap_or("cport.toml".into());
    let cfg = cport::read_toml(&toml)?;
    let mut builder = cport::Builder::new(cfg);
    let mut container = builder.get_container()?;
    container.start()?;
    container.apt()?;
    container.configure()?;
    container.build()?;
    container.stop()?;
    Ok(())
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

    if let Err(e) = build(opt) {
        if let Some(err) = e.downcast_ref() {
            match err {
                shiplift::errors::Error::Fault { code, message } => {
                    warn!("Failed to create a container: reason = {}", code);
                    warn!("{}", message);
                }
                _ => {
                    error!("Unknown error around container manipulation");
                }
            };
        }
        error!("{:?}", e);
        std::process::exit(1)
    };
}
