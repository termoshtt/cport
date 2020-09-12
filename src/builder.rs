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

use crate::config::Configure;

use failure::Fallible;
use futures::stream::Stream;
use log::*;
use maplit::hashmap;
use shiplift::{
    Container, ContainerFilter, ContainerListOptions, ContainerOptions, Docker,
    ExecContainerOptions,
};
use std::{collections::HashMap, path::Path};
use tokio::{prelude::Future, runtime::Runtime};

/// Container builder corresponding to the setting in TOML
///
/// - The container created by `cport` has a label `cport.source`,
///   whose value is the abstract path of the TOML file.
pub struct Builder {
    runtime: Runtime,
    docker: Docker,
    cfg: Configure,
}

impl Builder {
    pub fn new(cfg: Configure) -> Self {
        let runtime = Runtime::new().expect("Cannot init tokio runtime");
        let docker = Docker::new();
        Builder {
            runtime,
            docker,
            cfg,
        }
    }

    fn seek(&mut self) -> Fallible<Option<String>> {
        let image = self.runtime.block_on(
            self.docker.containers().list(
                &ContainerListOptions::builder()
                    .all()
                    .filter(vec![ContainerFilter::Label(
                        "cport.source".into(),
                        format!("{}", self.cfg.source.display()),
                    )])
                    .build(),
            ),
        )?;
        Ok(if !image.is_empty() {
            let id = &image[0].id;
            Some(id.into())
        } else {
            None
        })
    }

    pub fn get_container(&mut self) -> Fallible<ContainerRef> {
        let id = if let Some(id) = self.seek()? {
            info!("Container found: {}", id);
            id
        } else {
            let src = format!("{}", self.cfg.source.display());
            let id = self.runtime.block_on(
                self.docker
                    .containers()
                    .create(
                        &ContainerOptions::builder(&self.cfg.image)
                            .volumes(vec![&format!("{}:{}", src, src)])
                            .tty(true)
                            .labels(&hashmap! {
                                "cport.source" => src.as_str(),
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
            id
        };
        Ok(ContainerRef {
            runtime: &mut self.runtime,
            container: Container::new(&self.docker, id),
            cfg: &self.cfg,
        })
    }
}

pub struct ContainerRef<'a> {
    runtime: &'a mut Runtime,
    container: Container<'a, 'static>,
    cfg: &'a Configure,
}

impl<'a> ContainerRef<'a> {
    pub fn start(&mut self) -> Fallible<()> {
        info!("Start container");
        self.runtime.block_on(self.container.start())?;
        Ok(())
    }

    pub fn stop(&mut self) -> Fallible<()> {
        info!("Stop container");
        self.runtime.block_on(self.container.stop(None))?;
        Ok(())
    }

    pub fn apt(&mut self) -> Fallible<()> {
        info!("apt install");
        self.runtime.block_on(
            self.container
                .exec(
                    &ExecContainerOptions::builder()
                        .cmd(vec!["apt", "update"])
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
            self.container
                .exec(
                    &ExecContainerOptions::builder()
                        .cmd(
                            vec!["apt", "install", "-y"]
                                .into_iter()
                                .chain(self.cfg.apt.iter().map(|s| s.as_str()))
                                .collect(),
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
        Ok(())
    }

    pub fn configure(&mut self) -> Fallible<()> {
        info!("cmake configure step");
        let build_dir = self.cfg.source.join(&self.cfg.build);
        self.runtime.block_on(
            self.container
                .exec(
                    &ExecContainerOptions::builder()
                        .cmd(
                            CMakeArgBuilder::new()
                                .build_dir(&build_dir)
                                .source_dir(&self.cfg.source)
                                .option(&self.cfg.option)
                                .generator(&self.cfg.generator)
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
        Ok(())
    }

    pub fn build(&mut self) -> Fallible<()> {
        info!("cmake build step");
        let build_dir = self.cfg.source.join(&self.cfg.build);
        self.runtime.block_on(
            self.container
                .exec(
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
