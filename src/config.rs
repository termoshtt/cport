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

use failure::{format_err, Fallible};
use log::*;
use std::{
    collections::HashMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
};

#[derive(serde::Deserialize)]
struct CPort {
    image: String,
    apt: Option<Vec<String>>,
}

#[derive(serde::Deserialize)]
struct CMake {
    generator: Option<String>,
    build: Option<String>,
    option: Option<HashMap<String, String>>,
}

#[derive(serde::Deserialize)]
struct ParsedConfigure {
    source: Option<PathBuf>,
    cport: CPort,
    cmake: CMake,
}

/// Normalized, flattened configure
#[derive(Debug, Clone, PartialEq)]
pub struct Configure {
    /// Directory of root CMakeLists.txt exists.
    /// It will be the directory where the TOML file exists if not specified
    pub source: PathBuf,

    /// cport.image; container image
    pub image: String,
    /// cport.apt
    pub apt: Vec<String>,

    /// cmake.generator; used for `-G` option in cmake
    pub generator: String,
    /// cmake.build; used for `-B` option in cmake
    pub build: String,
    /// cmake.option; used for `-D{key}={value}` in cmake
    pub option: HashMap<String, String>,
}

impl ParsedConfigure {
    fn load<P: AsRef<Path>>(filename: P) -> Fallible<Self> {
        let filename = filename.as_ref();
        let mut f = fs::File::open(filename)?;
        let mut buf = String::new();
        f.read_to_string(&mut buf)?;
        let mut cfg: ParsedConfigure = toml::from_str(&buf)?;
        if cfg.source.is_none() {
            let abspath = filename.canonicalize().map_err(|_| {
                format_err!("Cannot canonicalize TOML path: {}", filename.display())
            })?;
            cfg.source = abspath.parent().map(|p| p.into());
        }
        Ok(cfg)
    }

    fn normalize(self) -> Configure {
        Configure {
            source: self.source.unwrap(),
            // cport
            image: self.cport.image,
            apt: self.cport.apt.unwrap_or(Vec::new()),
            // cmake
            generator: self.cmake.generator.unwrap_or("Ninja".into()),
            build: self.cmake.build.unwrap_or("_cport".into()),
            option: self.cmake.option.unwrap_or(HashMap::new()),
        }
    }
}

/// Read and normalize configure TOML
pub fn read_toml<P: AsRef<Path>>(filename: P) -> Fallible<Configure> {
    let filename = filename.as_ref();
    let cfg = ParsedConfigure::load(&filename)
        .map_err(|_| format_err!("Cannot read TOML file: {}", filename.display()))?;
    let cfg = cfg.normalize();
    info!("Load {}: {:?}", filename.display(), cfg);
    Ok(cfg)
}

#[cfg(test)]
mod tests {
    use maplit::hashmap;

    #[test]
    fn read_toml() -> failure::Fallible<()> {
        let cfg = super::ParsedConfigure::load("cport.toml")?;

        assert_eq!(cfg.cport.image, "debian");
        assert_eq!(cfg.cport.apt, Some(vec!["libboost-dev".to_string()]));

        assert_eq!(cfg.cmake.generator, Some("Ninja".into()));
        assert_eq!(cfg.cmake.build, Some("_cport".into()));
        assert_eq!(
            cfg.cmake.option,
            Some(hashmap! {
                "CMAKE_EXPORT_COMPILE_COMMANDS".into() => "ON".into(),
            })
        );
        Ok(())
    }
}
