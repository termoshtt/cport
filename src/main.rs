use log::{error, info, warn};
use shiplift::{ContainerListOptions, ContainerOptions, Docker};
use std::env;
use std::path::PathBuf;
use structopt::StructOpt;
use tokio::{prelude::Future, runtime::Runtime};

#[derive(Debug, StructOpt)]
#[structopt(name = "cport")]
struct Opt {
    /// Build directory name
    #[structopt(help = "Image name")]
    image: String,

    /// Build directory name
    #[structopt(short = "-B")]
    build_dir: Option<String>,

    /// Path of configure file
    #[structopt(parse(from_os_str), short = "-H")]
    source: Option<PathBuf>,

    /// Nickname of build container (for avoiding duplicate container name)
    #[structopt(short = "-x", long = "nickname")]
    nickname: Option<String>,

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

struct Builder {
    runtime: Runtime,
    docker: Docker,
    image: String,
    source: PathBuf,
    build_dir: String,
    name: String,
}

impl Builder {
    fn new(opt: Opt) -> Self {
        let runtime = Runtime::new().expect("Cannot init tokio runtime");
        let docker = Docker::new();

        let cur_dir = env::current_dir().expect("Cannot get current dir");

        let image = opt.image;
        info!("Docker image = {}", image);
        let build_dir = opt.build_dir.unwrap_or("_cbuild".into());
        info!("Build directory name = {}", build_dir);
        let source = opt.source.unwrap_or(cur_dir);
        info!("Source directory = {}", source.display());

        let name = if let Some(nickname) = &opt.nickname {
            format!("{}{}-{}", image, source.display(), nickname)
        } else {
            format!("{}{}", image, source.display(),)
        }
        .replace("/", "_");
        info!("Container name = {}", name);

        Builder {
            runtime,
            docker,
            name,
            build_dir,
            image,
            source,
        }
    }

    fn seek_container(&mut self) -> Result<Option<String>, shiplift::errors::Error> {
        // XXX Is there no API to seek named container??
        let image: Vec<_> = self
            .runtime
            .block_on(
                self.docker
                    .containers()
                    .list(&ContainerListOptions::builder().all().build()),
            )?
            .into_iter()
            .filter(|c| {
                for n in &c.names {
                    // XXX ignore top '/'
                    if &n[1..] == &self.name {
                        return true;
                    }
                }
                return false;
            })
            .collect();
        Ok(if !image.is_empty() {
            info!("Container found");
            Some(image[0].id.to_string())
        } else {
            info!("No coutainer found");
            None
        })
    }

    fn create_container(&mut self) -> Result<String, shiplift::errors::Error> {
        if let Some(id) = self.seek_container()? {
            return Ok(id);
        }
        info!("Create new container: {}", self.name);
        self.runtime.block_on(
            self.docker
                .containers()
                .create(
                    &ContainerOptions::builder(&self.image)
                        .name(&self.name)
                        .volumes(vec![&format!("{}:/src", self.source.display(),)])
                        .tty(true)
                        .auto_remove(false)
                        .build(),
                )
                .map(|status| {
                    if let Some(warn) = status.warnings {
                        for w in warn {
                            eprintln!("{}", w);
                        }
                    }
                    status.id
                }),
        )
    }
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

    let mut builder = Builder::new(opt);
    let res = builder.create_container();
    match res {
        Ok(status) => {
            println!("Create succeeded. ID = {}", status);
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
