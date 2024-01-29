use std::{
    io::{BufReader, BufWriter},
    path::Path,
};

use serde::{de::DeserializeOwned, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum AssetError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    MessagePackDecodingError(#[from] rmp_serde::decode::Error),
    #[error(transparent)]
    MessagePackEncodingError(#[from] rmp_serde::encode::Error),
    #[error(transparent)]
    YamlError(#[from] serde_yaml::Error),
}

#[derive(Debug)]
pub enum Backend {
    MessagePack,
    Yaml,
}

pub trait Asset: DeserializeOwned + Serialize + Sized {
    const BACKEND: Backend;

    fn load(path: impl AsRef<Path>) -> Result<Self, AssetError> {
        let file = std::fs::File::open(path)?;
        let reader = BufReader::new(file);

        Ok(match Self::BACKEND {
            Backend::MessagePack => rmp_serde::from_read(reader)?,
            Backend::Yaml => serde_yaml::from_reader(reader)?,
        })
    }

    fn save(&self, path: impl AsRef<Path>) -> Result<(), AssetError> {
        let file = std::fs::File::create(path)?;
        let mut writer = BufWriter::new(file);

        match Self::BACKEND {
            Backend::MessagePack => rmp_serde::encode::write(&mut writer, self)?,
            Backend::Yaml => serde_yaml::to_writer(&mut writer, self)?,
        }

        Ok(())
    }
}
