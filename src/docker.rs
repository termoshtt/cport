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

use serde::Deserialize;
use std::{io::BufRead, process::Command};

/// Raw entry of `docker ps --format {{json .}}`
#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
struct RawPsEntry {
    Command: String,
    Mounts: String,
    Names: String,
    ID: String,
    Image: String,
    Labels: String,
}

#[derive(Debug, Clone)]
pub struct PsEntry {
    command: Vec<String>,
    mounts: Vec<String>,
    name: String,
    id: String,
    image: String,
    labels: Vec<String>,
}

impl PsEntry {
    fn from_raw(raw: RawPsEntry) -> Self {
        fn trim_split(input: &str, sep: &str) -> Vec<String> {
            input
                .trim_matches('"')
                .trim()
                .split(sep)
                .flat_map(|x| {
                    if x.is_empty() {
                        None
                    } else {
                        Some(x.to_string())
                    }
                })
                .collect()
        }
        PsEntry {
            name: raw.Names,
            id: raw.ID,
            image: raw.Image,
            mounts: trim_split(&raw.Mounts, ","),
            labels: trim_split(&raw.Labels, ","),
            command: trim_split(&raw.Command, " "),
        }
    }
}

pub fn ps() -> Vec<PsEntry> {
    let output = Command::new("docker")
        .arg("ps")
        .arg("--no-trunc")
        .args(&["--format", r#"{{json .}}"#])
        .output()
        .unwrap();
    let output = std::io::BufReader::new(output.stdout.as_slice());
    output
        .lines()
        .map(|line| {
            let line = line.unwrap();
            let raw = serde_json::from_str(&line).unwrap();
            PsEntry::from_raw(raw)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn ps() {
        let container_list = super::ps();
        dbg!(container_list);
    }
}
