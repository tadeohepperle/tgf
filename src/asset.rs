use std::path::PathBuf;

use image::RgbaImage;

/// An Asset that can be fetched from bytes. The bytes could come from anywhere, e.g. the network, the disk, embedded in the binary, don't care.
pub trait AssetT: Sized {
    fn from_bytes(bytes: &[u8]) -> Result<Self, anyhow::Error>;

    fn load(path: &str) -> Result<Self, anyhow::Error> {
        let bytes: Vec<u8> = std::fs::read(path)?;
        Self::from_bytes(&bytes)
    }
}

impl AssetT for RgbaImage {
    fn from_bytes(bytes: &[u8]) -> Result<Self, anyhow::Error> {
        let image = image::load_from_memory(bytes)?;
        let rgba = image.to_rgba8();
        Ok(rgba)
    }
}

impl AssetT for String {
    // Note: expects bytes to be utf8 encoded
    fn from_bytes(bytes: &[u8]) -> Result<Self, anyhow::Error> {
        let text = String::from_utf8(bytes.to_vec())?;
        Ok(text)
    }
}
