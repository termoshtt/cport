/*
Copyright 2019-2020 Toshiki Teramura <toshiki.teramura@gmail.com>

This file is part of cport.

cport is free software: you can redistribute it and/or modify
it under the terms of the GNU Affero General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

cport is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License
along with cport.  If not, see <http://www.gnu.org/licenses/>.
*/

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
