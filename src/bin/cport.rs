use failure::Fallible;
use std::{env, path::PathBuf};
use structopt::{clap::AppSettings::ColoredHelp, StructOpt};

#[derive(Debug, StructOpt)]
#[structopt(
    name = "cport",
    about = "cmake container builder",
    raw(setting = "ColoredHelp")
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

    #[structopt(subcommand)]
    command: Command,
}

#[derive(Debug, StructOpt)]
enum Command {
    /// Configure and build
    #[structopt(name = "build", raw(setting = "ColoredHelp"))]
    Build {},

    /// Install apt/yum packages
    #[structopt(name = "install", raw(setting = "ColoredHelp"))]
    Install {},
}

fn main() -> Fallible<()> {
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

    let toml = opt.config_toml.unwrap_or("cport.toml".into());
    let cfg = cport::read_toml(&toml)?;
    let mut builder = cport::Builder::new(cfg);
    let mut container = builder.get_container()?;

    match opt.command {
        Command::Build {} => {
            container.start()?;
            container.configure()?;
            container.build()?;
            container.stop()?;
        }
        Command::Install {} => {
            container.start()?;
            container.apt()?;
            container.stop()?;
        }
    }
    Ok(())
}
