use shiplift::{ContainerListOptions, ContainerOptions, Docker};
use std::path::PathBuf;
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
    ) -> Result<String, shiplift::errors::Error> {
        let name = format!("cmake-container-build-{}", image_name);
        let images = self.runtime.block_on(
            self.docker
                .containers()
                .list(&ContainerListOptions::builder().all().build()),
        )?;
        let image: Vec<_> = images
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
                        .auto_remove(true)
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

    let res = builder.create_build_container(&opt.image);
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
