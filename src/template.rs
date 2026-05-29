use regex::{Regex, RegexBuilder};
use std::io;
use std::path::{Path, PathBuf};

fn get_regex(key: impl AsRef<str>) -> Regex {
    let key = key.as_ref();
    let pattern = format!("^({key})=(\"(?:.|[\\s\\t\\r\\n])*?\"|.+?$)");

    RegexBuilder::new(&pattern)
        .multi_line(true)
        .case_insensitive(true)
        .build()
        .expect("Invalid Regex")
}

fn process_value(value: impl AsRef<str>) -> Vec<String> {
    let value = value.as_ref().trim().trim_matches('"').trim();

    if value.contains('\n') {
        value.split('\n').map(|x| x.trim().to_owned()).collect()
    } else {
        vec![value.to_owned()]
    }
}

pub struct Template {
    contents: String,
    path: PathBuf,
}

impl Template {
    pub fn from_file(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let contents = std::fs::read_to_string(&path)?;

        Ok(Self { contents, path })
    }

    pub fn get(&self, key: impl AsRef<str>) -> Option<Vec<String>> {
        let key = key.as_ref();
        let key = regex::escape(key);
        let regex = get_regex(key);
        let caps = regex.captures(&self.contents)?;

        caps.get(2).map(|m| m.as_str().trim()).map(process_value)
    }

    pub fn get_single(&self, key: impl AsRef<str>) -> Option<String> {
        let values = self.get(key)?;

        if values.len() == 1 {
            Some(values[0].to_owned())
        } else {
            None
        }
    }

    pub fn get_asset_name(&self) -> Option<String> {
        let name = self.get_single("pkgname")?;
        let version = self.get_single("version")?;
        let revision = self.get_single("revision")?;

        Some(format!("{name}-{version}_{revision}.x86_64.xbps"))
    }

    pub fn set(&mut self, key: impl AsRef<str>, value: impl ToString) {
        let key = key.as_ref();
        let key = regex::escape(key);
        let regex = get_regex(&key);

        if regex.is_match(&self.contents) {
            let value = value.to_string();
            let replacement = format!("$1={}", shell_words::quote(&value));
            self.contents = regex
                .replace_all(&self.contents, replacement.as_str())
                .to_string();
        } else {
            // prepend
            self.contents = format!(
                "{}={}\n{}",
                key,
                shell_words::quote(&value.to_string()),
                self.contents
            );
        }
    }

    pub fn save(&self) -> io::Result<()> {
        std::fs::write(&self.path, &self.contents)
    }
}
