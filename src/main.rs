use shiplift::{ContainerListOptions, ContainerOptions, Docker};
use std::env;
use std::path::{Path, PathBuf};
use structopt::StructOpt;
use tokio::{prelude::Future, runtime::Runtime};

#[derive(Debug, StructOpt)]
#[structopt(name = "cmake-container-build")]
struct Opt {
    /// Build directory name
    #[structopt(help = "Image name")]
    image: String,

    /// Build directory name
    #[structopt(parse(from_os_str), short = "-B")]
    build: Option<PathBuf>,

    /// Path of configure file
    #[structopt(parse(from_os_str))]
    config: Option<PathBuf>,
}

struct Builder {
    runtime: Runtime,
    docker: Docker,
}

impl Builder {
    fn new() -> Self {
        let runtime = Runtime::new().expect("Cannot init tokio runtime");
        let docker = Docker::new();
        Builder { runtime, docker }
    }

    fn create_build_container(
        &mut self,
        image_name: &str,
        src: &Path,
    ) -> Result<String, shiplift::errors::Error> {
        let src = src.canonicalize().expect("Cannot canonicalize source path");
        let name = format!("cmake-cb-{}{}", image_name, src.display()).replace("/", "_");

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
                    if &n[1..] == &name {
                        return true;
                    }
                }
                return false;
            })
            .collect();
        if !image.is_empty() {
            return Ok(image[0].id.to_string());
        }

        eprintln!("No build container found. Create a new container...");
        self.runtime.block_on(
            self.docker
                .containers()
                .create(
                    &ContainerOptions::builder(image_name)
                        .name(&name)
                        .volumes(vec![&format!("{}:/src", src.display())])
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
    println!("{:?}", opt);

    let mut builder = Builder::new();

    let cur_dir = env::current_dir().unwrap();
    let res = builder.create_build_container(&opt.image, &cur_dir);
    match res {
        Ok(status) => {
            println!("Create succeeded. ID = {}", status);
        }
        Err(e) => {
            match e {
                shiplift::errors::Error::Fault { code, message } => {
                    eprintln!("Failed to create a container: reason = {}", code);
                    eprintln!("{}", message);
                }
                _ => {
                    eprintln!("{:?}", e);
                }
            };
            std::process::exit(1)
        }
    }
}
