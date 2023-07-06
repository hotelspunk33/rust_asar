
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

/// Asar represents the structure of an Asar archive file, allowing for extraction, modification, and creation.
///
/// Values required to contruct/deconstruct archive file:
/// - src_path: Path to either directory or Asar archive file
/// - content: Content enum to represent the file structure within an Asar archive file
/// - start: Offset at which content begins (after the header) in archive file.

#[derive(Clone, Debug)]
pub struct Asar {
    pub src_path: PathBuf,
    pub content: Content,
    pub start: u64,
}

impl Asar {

    /// Opens either an Asar archive file or a directory.
    /// 
    /// Takes in one argument of type Path that represents either the Asar archive file or a directory/folder.
    /// 
    /// Initializes necessary fields within Asar struct, returning instantiated struct or Error.

    pub fn open<P: AsRef<Path>>(src_path: P) -> Result<Asar, asar_error::Error> {
        let src_path = src_path.as_ref();

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

    // Obtains header of Asar archive file, returing the header as a serde_json::Value and the start offset.
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

    /// Returns a vector of all Paths within an archive as Strings, otherwise an Error.
    /// 
    /// If a path is unable to be casted to a String, it will add as the default string `""`.
    
    pub fn list(&self) -> Result<Vec<String>, asar_error::Error> {
        Ok(self
            .content
            .paths_to_vec()?
            .iter()
            .map(|path| path.to_str().unwrap_or_default().to_string())
            .collect::<Vec<String>>())
    }

    /// Writes content of instantiated Asar struct at the specified destination (Path) as a folder. 
    /// > Similar to the extract function in Electron's Asar JS library.
    /// 
    /// Returns either () or an Error. 
    /// 
    /// At the moment, calling this function on an improperly instantiated Asar struct may
    /// result in unintended consequences.
     
    pub fn extract<P: AsRef<Path>>(&self, destination: P) -> Result<(), asar_error::Error> {
        let file = File::open(self.src_path.as_path())?;

        self.content
            .write_to_dir(destination, &file, self.start)?;

        Ok(())
    }

    /// Takes one argument of type Path and provides the file as a vector of bytes if it exists 
    /// and an Asar archive file is open.
    /// 
    /// 
    /// The path provided must be a file and it must exist otherwise `None` will be returned.
    /// If a directory/folder is open, `None` will be returned.
    /// > If an error occures while opening the file, `None` will be returned.
    
    pub fn get_file<P: AsRef<Path>>(&self, path: P) -> Option<Vec<u8>> {
        
        if self.src_path.is_dir() {
            return None
        }

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

    /// Takes in one argument of type `&str`, returning a vector of all paths
    /// that contain the provided pattern (argument).
    /// 
    /// Paths are checked to contain the pattern using the contains function with string slices.
    
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
