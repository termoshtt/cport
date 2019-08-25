use crate::config::Configure;

use failure::Fallible;
use futures::stream::Stream;
use log::*;
use maplit::hashmap;
use shiplift::{
    Container, ContainerFilter, ContainerListOptions, ContainerOptions, Docker,
    ExecContainerOptions,
};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use tokio::{prelude::Future, runtime::Runtime};

pub struct Builder {
    runtime: Runtime,
    docker: Docker,
    image: String,
    source: PathBuf,
    build: String,
    generator: String,
    option: HashMap<String, String>,
}

impl Builder {
    pub fn new(opt: Configure) -> Self {
        let runtime = Runtime::new().expect("Cannot init tokio runtime");
        let docker = Docker::new();
        Builder {
            runtime,
            docker,
            build: opt.cmake.build.unwrap_or("_cport".into()),
            image: opt.cport.image,
            source: opt.source.unwrap(),
            generator: opt.cmake.generator.unwrap_or("Ninja".into()),
            option: opt.cmake.option.unwrap_or(HashMap::new()),
        }
    }

    pub fn seek(&mut self) -> Fallible<Option<String>> {
        let image = self.runtime.block_on(
            self.docker.containers().list(
                &ContainerListOptions::builder()
                    .all()
                    .filter(vec![
                        ContainerFilter::Label("cport.image".into(), self.image.clone()),
                        ContainerFilter::Label(
                            "cport.source".into(),
                            format!("{}", self.source.display()),
                        ),
                        ContainerFilter::Label("cport.build".into(), self.build.clone()),
                    ])
                    .build(),
            ),
        )?;
        Ok(if !image.is_empty() {
            let id = &image[0].id;
            info!("Container found: {}", id);
            Some(id.into())
        } else {
            info!("No coutainer found");
            None
        })
    }

    pub fn create(&mut self) -> Fallible<String> {
        if let Some(id) = self.seek()? {
            return Ok(id);
        }
        let src = format!("{}", self.source.display());
        let id = self.runtime.block_on(
            self.docker
                .containers()
                .create(
                    &ContainerOptions::builder(&self.image)
                        .volumes(vec![&format!("{}:{}", src, src)])
                        .tty(true)
                        .labels(&hashmap! {
                            "cport.image" => self.image.as_str(),
                            "cport.source" => src.as_str(),
                            "cport.build" => &self.build,
                        })
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
        )?;
        info!("New container created: {}", id);
        Ok(id)
    }

    pub fn exec(&mut self, id: &str) -> Fallible<()> {
        let c = Container::new(&self.docker, id);
        info!("Start container: {}", id);
        self.runtime.block_on(c.start())?;

        let build_dir = self.source.join(&self.build);
        info!("Start build");
        self.runtime.block_on(
            c.exec(
                &ExecContainerOptions::builder()
                    .cmd(
                        CMakeArgBuilder::new()
                            .build_dir(&build_dir)
                            .source_dir(&self.source)
                            .option(&self.option)
                            .generator(&self.generator)
                            .get_args(),
                    )
                    .attach_stdout(true)
                    .attach_stderr(true)
                    .build(),
            )
            .for_each(|chunk| {
                print!("{}", chunk.as_string_lossy());
                Ok(())
            }),
        )?;
        self.runtime.block_on(
            c.exec(
                &ExecContainerOptions::builder()
                    .cmd(CMakeArgBuilder::new().build_mode(&build_dir).get_args())
                    .attach_stdout(true)
                    .attach_stderr(true)
                    .build(),
            )
            .for_each(|chunk| {
                print!("{}", chunk.as_string_lossy());
                Ok(())
            }),
        )?;
        info!("Stop container: {}", &id);
        self.runtime.block_on(c.stop(None))?;
        Ok(())
    }
}

struct CMakeArgBuilder {
    params: Vec<String>,
}

impl CMakeArgBuilder {
    fn new() -> Self {
        CMakeArgBuilder {
            params: vec!["cmake".into()],
        }
    }

    fn get_args(&self) -> Vec<&str> {
        info!("Generate command: {}", self.params.join(" "));
        self.params.iter().map(|s| s.as_str()).collect()
    }

    fn build_dir<P: AsRef<Path>>(&mut self, dir: P) -> &mut Self {
        let dir = dir.as_ref();
        self.params.push(format!("-B{}", dir.display()));
        self
    }

    fn source_dir<P: AsRef<Path>>(&mut self, dir: P) -> &mut Self {
        let dir = dir.as_ref();
        self.params.push(format!("-H{}", dir.display()));
        self
    }

    fn generator(&mut self, gen: &str) -> &mut Self {
        self.params.push(format!("-G{}", gen));
        self
    }

    fn option(&mut self, opt: &HashMap<String, String>) -> &mut Self {
        for (key, value) in opt {
            self.params.push(format!("-D{}={}", key, value));
        }
        self
    }

    fn build_mode<P: AsRef<Path>>(&mut self, build_dir: P) -> &mut Self {
        self.params.push("--build".into());
        self.params
            .push(format!("{}", build_dir.as_ref().display()));
        self
    }
}
