use crate::config::Configure;

use futures::stream::Stream;
use log::*;
use maplit::hashmap;
use shiplift::{
    Container, ContainerFilter, ContainerListOptions, ContainerOptions, Docker,
    ExecContainerOptions,
};
use std::env;
use std::path::PathBuf;
use tokio::{prelude::Future, runtime::Runtime};

pub struct Builder {
    runtime: Runtime,
    docker: Docker,
    image: String,
    source: PathBuf,
    build: String,
}

type Result<T> = ::std::result::Result<T, shiplift::errors::Error>;

impl Builder {
    pub fn new(opt: Configure) -> Self {
        let runtime = Runtime::new().expect("Cannot init tokio runtime");
        let docker = Docker::new();

        let cur_dir = env::current_dir().expect("Cannot get current dir");

        Builder {
            runtime,
            docker,
            build: opt.build.unwrap_or("_cport".into()),
            image: opt
                .image
                .unwrap_or("registry.gitlab.com/termoshtt/cport/debian".into()),
            source: opt.source.unwrap_or(cur_dir),
        }
    }

    pub fn seek(&mut self) -> Result<Option<String>> {
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

    pub fn create(&mut self) -> Result<String> {
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

    pub fn exec(&mut self, id: &str) -> Result<()> {
        let c = Container::new(&self.docker, id);
        info!("Start container: {}", id);
        self.runtime.block_on(c.start())?;

        let build_dir = self.source.join(&self.build);
        info!("Start build");
        self.runtime.block_on(
            c.exec(
                &ExecContainerOptions::builder()
                    .cmd(vec![
                        "cmake",
                        &format!("-H{}", self.source.display()),
                        &format!("-B{}", build_dir.display()),
                        // TODO cmake flags
                    ])
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
                    .cmd(vec![
                        "cmake",
                        "--build",
                        &format!("{}", build_dir.display()),
                    ])
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
