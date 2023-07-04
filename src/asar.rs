
use std::{
    fs::File,
    path::{Path, PathBuf},
};

use byteorder::LittleEndian;
use positioned_io::{ReadAt, ReadBytesExt};
use serde_json::Value;

use crate::{
    asar_error::{self, Error},
    content::Content,
};



const JSON_LEN_OFFSET: u64 = 12;
const JSON_OFFSET: u64 = 16;
const HEADER_LEN_OFFSET: u64 = 8;

/// Asar represents the structure of an Asar archive file.
///
/// Values to contruct/deconstruct archive file:
/// - src_path: Path to either directory or Asar archive file
/// - content: Content enum to represent the file structure within an Asar archive file
/// - start: offset at which content begins (after the header)

#[derive(Clone, Debug)]
pub struct Asar {
    src_path: PathBuf,
    content: Content,
    start: u64,
}

impl Asar {

    /// Opens either an Asar archive file or a directory.
    /// Initializes necessary fields within Asar struct, returning instantiated struct or Err.

    pub fn open(src_path: &Path) -> Result<Asar, asar_error::Error> {
        if src_path.is_dir() {
            todo!("TODO: Asar archive deconstruction")
        } else {
            //src must be asar, assume it is and dont check
            let file = File::open(src_path)?;

            if let Ok((value, start)) = Self::get_asar_header(&file) {
                
                Ok(Asar {
                    src_path: src_path.to_path_buf(),
                    content: Content::new(value)?,
                    start: start,
                })
            } else {
                Err(Error::ParseHeaderError(
                    "Failed to parse archive header, check format".to_string(),
                ))
            }
        }
    }

    fn get_asar_header(file: &File) -> Result<(Value, u64), asar_error::Error> {
        let json_len = file.read_u32_at::<LittleEndian>(JSON_LEN_OFFSET)?;
        let mut json_u8: Vec<u8> = vec![0; json_len as usize];

        file.read_exact_at(JSON_OFFSET, &mut json_u8)?;

        let value = serde_json::from_slice(&json_u8)?;
        let start = {
            // 12 bytes prior to header must be included
            (file.read_u32_at::<LittleEndian>(HEADER_LEN_OFFSET)? + 12) as u64
        };

        Ok((value, start))
    }

    /// Returns a vector of all Paths of archive as Strings.
    pub fn list(&self) -> Result<Vec<String>, asar_error::Error> {
        Ok(self
            .content
            .paths_to_vec()?
            .iter()
            .map(|path| path.to_str().unwrap_or_default().to_string())
            .collect::<Vec<String>>())
    }

    /// Extracts the Asar archive to specified extraction path.
    pub fn extract(&self, extract_path: &Path) -> Result<(), asar_error::Error> {
        let file = File::open(self.src_path.as_path())?;

        self.content
            .write_to_dir(extract_path, &file, self.start)?;

        Ok(())
    }

    /// If the file exists at the path provided, the file will return as a vector of bytes, otherwise 'None'.
    pub fn get_file(&self, path: &Path) -> Option<Vec<u8>> {
        if let Some(Content::File(_, offset, size)) = self.content.find(path) {
            let file = File::open(self.src_path.as_path());
            if let Ok(file) = file {
                let mut result: Vec<u8> = vec![0; size as usize];

                if let Ok(()) = file.read_exact_at(self.start + offset, &mut result) {
                    return Some(result)
                }
            } 
        }
        None
    }

    /// Returns a vector of all paths as PathBufs that contain the &str provided.
    pub fn get_paths_contain(&self, pat: &str) -> Vec<PathBuf> {
        let mut paths: Vec<PathBuf> = Vec::new();

        if let Ok(list) = self.content.paths_to_vec() {
            for path in list {
                if let Some(file_os_str) = path.file_name() {
                    if file_os_str.to_str().unwrap().contains(pat) {
                        paths.push(path);
                    }
                }
            }
        }

        paths
    }
}
