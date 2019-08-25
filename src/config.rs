use failure::Fallible;
use std::{
    collections::HashMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
};

/// Configure about container management
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct CPort {
    pub image: String,
    pub apt: Option<Vec<String>>,
}

/// Configure about cmake execution
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct CMake {
    pub builder: Option<String>,
    pub build: Option<String>,
    pub option: Option<HashMap<String, String>>,
}

/// Root type for reading configure TOML
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct Configure {
    /// Directory of root CMakeLists.txt exists
    ///
    /// - The directory where the TOML file exists if not specified
    ///
    pub source: Option<PathBuf>,
    pub cport: CPort,
    pub cmake: CMake,
}

impl Configure {
    pub fn load<P: AsRef<Path>>(filename: P) -> Fallible<Self> {
        let filename = filename.as_ref();
        let mut f = fs::File::open(filename)?;
        let mut buf = String::new();
        f.read_to_string(&mut buf)?;
        let mut cfg: Configure = toml::from_str(&buf)?;
        if cfg.source.is_none() {
            cfg.source = filename.parent().map(|p| p.into());
        }
        Ok(cfg)
    }
}

#[cfg(test)]
mod tests {
    use maplit::hashmap;

    #[test]
    fn read_toml() -> failure::Fallible<()> {
        let cfg = super::Configure::load("cport.toml")?;
        dbg!(&cfg);

        assert_eq!(cfg.cport.image, "debian");
        assert_eq!(cfg.cport.apt, Some(vec!["libboost-dev".to_string()]));

        assert_eq!(cfg.cmake.builder, Some("Ninja".into()));
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
