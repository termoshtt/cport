use shiplift::{ContainerListOptions, ContainerOptions, Docker};
use std::env;
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
    #[structopt(short = "-B")]
    build_dir: Option<String>,

    /// Path of configure file
    #[structopt(parse(from_os_str), short = "-H")]
    source: Option<PathBuf>,

    /// Nickname of build container (for avoiding duplicate container name)
    #[structopt(short = "-x", long = "nickname")]
    nickname: Option<String>,
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
        let build_dir = opt.build_dir.unwrap_or("_cbuild".into());
        let source = opt.source.unwrap_or(cur_dir);

        let name = if let Some(nickname) = &opt.nickname {
            format!("cmake-cb-{}{}-{}", image, source.display(), nickname)
        } else {
            format!("cmake-cb-{}{}", image, source.display(),)
        }
        .replace("/", "_");

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
            Some(image[0].id.to_string())
        } else {
            None
        })
    }

    fn create_container(&mut self) -> Result<String, shiplift::errors::Error> {
        if let Some(id) = self.seek_container()? {
            return Ok(id);
        }
        eprintln!("No build container found. Create a new container...");
        self.runtime.block_on(
            self.docker
                .containers()
                .create(
                    &ContainerOptions::builder(&self.image)
                        .name(&self.name)
                        .volumes(vec![&format!("{}:/src", self.source.display(),)])
                        .auto_remove(false)
                        .entrypoint("cmake")
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

    let mut builder = Builder::new(opt);

    let res = builder.create_container();
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
