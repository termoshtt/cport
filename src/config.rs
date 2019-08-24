use std::path::{Path, PathBuf};

pub struct Configure {
    pub build: Option<String>,
    pub image: Option<String>,
    pub source: Option<PathBuf>,
}

impl Configure {
    pub fn load<P: AsRef<Path>>(_filename: P) -> Self {
        unimplemented!()
    }
}
