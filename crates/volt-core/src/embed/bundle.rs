use std::collections::{HashMap, HashSet};
use std::path::Path;

use super::fs::collect_files;

/// A bundle of embedded web assets keyed by relative path (e.g., "index.html", "assets/main.js").
#[derive(Debug, Clone)]
pub struct AssetBundle {
    assets: HashMap<String, Vec<u8>>,
}

impl AssetBundle {
    /// Create a new empty asset bundle.
    pub fn new() -> Self {
        Self {
            assets: HashMap::new(),
        }
    }

    /// Create an asset bundle from a directory on disk.
    /// Reads all files recursively, storing them keyed by their relative path.
    pub fn from_directory(dir: &Path) -> Result<Self, std::io::Error> {
        let mut assets = HashMap::new();
        let mut visited_dirs = HashSet::new();
        collect_files(dir, dir, &mut assets, &mut visited_dirs, 0)?;
        Ok(Self { assets })
    }

    /// Insert an asset into the bundle.
    pub fn insert(&mut self, path: String, data: Vec<u8>) {
        self.assets.insert(path, data);
    }

    /// Look up an asset by relative path.
    pub fn get(&self, path: &str) -> Option<&[u8]> {
        self.assets.get(path).map(|v| v.as_slice())
    }

    /// Get the number of assets in the bundle.
    pub fn len(&self) -> usize {
        self.assets.len()
    }

    /// Check if the bundle is empty.
    pub fn is_empty(&self) -> bool {
        self.assets.is_empty()
    }

    /// Serialize the bundle to bytes (simple format: count + [path_len + path + data_len + data]).
    pub fn to_bytes(&self) -> Result<Vec<u8>, String> {
        fn as_u32_len(value: usize, field: &str) -> Result<u32, String> {
            u32::try_from(value).map_err(|_| format!("{field} exceeds u32::MAX"))
        }

        let mut buf = Vec::new();
        let count = as_u32_len(self.assets.len(), "asset count")?;
        buf.extend_from_slice(&count.to_le_bytes());

        for (path, data) in &self.assets {
            let path_bytes = path.as_bytes();
            let path_len = as_u32_len(path_bytes.len(), "path length")?;
            buf.extend_from_slice(&path_len.to_le_bytes());
            buf.extend_from_slice(path_bytes);

            let data_len = as_u32_len(data.len(), "asset byte length")?;
            buf.extend_from_slice(&data_len.to_le_bytes());
            buf.extend_from_slice(data);
        }

        Ok(buf)
    }

    /// Deserialize a bundle from bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self, String> {
        let mut assets = HashMap::new();
        let mut offset = 0;

        if data.len() < 4 {
            return Err("Bundle data too short".to_string());
        }

        let count = u32::from_le_bytes(
            data[offset..offset + 4]
                .try_into()
                .map_err(|_| "Invalid count bytes")?,
        ) as usize;
        offset += 4;

        for _ in 0..count {
            if offset + 4 > data.len() {
                return Err("Unexpected end of bundle data (path length)".to_string());
            }
            let path_len = u32::from_le_bytes(
                data[offset..offset + 4]
                    .try_into()
                    .map_err(|_| "Invalid path length bytes")?,
            ) as usize;
            offset += 4;

            if offset + path_len > data.len() {
                return Err("Unexpected end of bundle data (path)".to_string());
            }
            let path = String::from_utf8(data[offset..offset + path_len].to_vec())
                .map_err(|_| "Invalid UTF-8 in path")?;
            offset += path_len;

            if offset + 4 > data.len() {
                return Err("Unexpected end of bundle data (data length)".to_string());
            }
            let data_len = u32::from_le_bytes(
                data[offset..offset + 4]
                    .try_into()
                    .map_err(|_| "Invalid data length bytes")?,
            ) as usize;
            offset += 4;

            if offset + data_len > data.len() {
                return Err("Unexpected end of bundle data (data)".to_string());
            }
            let file_data = data[offset..offset + data_len].to_vec();
            offset += data_len;

            assets.insert(path, file_data);
        }

        Ok(Self { assets })
    }
}

impl Default for AssetBundle {
    fn default() -> Self {
        Self::new()
    }
}
