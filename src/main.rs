use shiplift::{ContainerOptions, Docker};
use std::path::PathBuf;
use structopt::StructOpt;
use tokio::runtime::Runtime;

#[derive(Debug, StructOpt)]
#[structopt(name = "cmake-container-build")]
struct Opt {
    /// Build directory name
    #[structopt(parse(from_os_str))]
    build: Option<PathBuf>,

    /// Path of configure file
    #[structopt(parse(from_os_str))]
    config: Option<PathBuf>,
}

fn container_option(image_name: &str) -> ContainerOptions {
    ContainerOptions::builder(image_name)
        .name(&format!("cmake-container-builder-{}", image_name))
        .auto_remove(true)
        .build()
}

fn main() {
    let opt = Opt::from_args();
    println!("{:?}", opt);

    let mut rt = Runtime::new().expect("Cannot init tokio runtime");

    let docker = Docker::new();

    let res = rt.block_on(docker.containers().create(&container_option("debian")));
    println!("{:?}", res);
}
