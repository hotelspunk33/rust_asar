
use std::{
    fs::{File, self, OpenOptions, remove_file},
    path::{Path, PathBuf}, io::Write,
};

use byteorder::{LittleEndian, WriteBytesExt};
use positioned_io::{ReadAt, ReadBytesExt};
use serde_json::{Value, Map, json};

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
    pub header: Option<Value>
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
            
            let (header, list) = Self::gen_header_from_dir(src_path)?;

            Ok(Asar {
                src_path: src_path.to_path_buf(),
                content: Content::new_list(list),
                start: (serde_json::to_vec(&header)?.len() + 16) as u64,
                header: Some(header)
            })

        } else {
            //src must be asar, assume it is and dont check
            let file = File::open(src_path)?;

            if let Ok((header, start)) = Self::get_asar_header(&file) {
                
                Ok(Asar {
                    src_path: src_path.to_path_buf(),
                    content: Content::new_json(header)?,
                    start: start,
                    header: None
                })
            } else {
                Err(Error::ParseHeaderError(
                    "Failed to parse archive header, check format".to_string(),
                ))
            }
        }
    }

    /// Returns a tuple of the header of an Asar archive file as `serde_json::Value`, and the start offset as `u64`, 
    /// otherwise Error.
    ///
    /// The file provided must be an Asar archive file, otherwise unintended behavior may occur.
    
    pub fn get_asar_header(file: &File) -> Result<(Value, u64), asar_error::Error> {
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

    /// Generates a header for the Asar archive file from the provided directory, along with a JSON header length.
    /// 
    /// Takes one argument of type Path which must be a folder- unintended behavior may occur if a file is passed.
    /// 
    /// `(Value, u64, Vec<(PathBuf, u64)>)`  ->  `(json_value, json_length, Vec<(file_path, file_size)>)`
    /// 
    /// Returns a tuple of `serde_json::Value` and `u64`, otherwise Error.
    /// 

    pub fn gen_header_from_dir<P: AsRef<Path>>(path: P) -> Result<(Value, Vec<(PathBuf, u64)>), asar_error::Error> {
        let mut offset: u64 = 0;
        let mut list_of_paths: Vec<(PathBuf, u64)> = Vec::new();

        /*let header = {
            let mut header = Map::new();
            
            header.insert("files".to_string(), Self::dir_to_value(path, &mut json_length, &mut list_of_paths)?);

            Value::Object(header)
        };*/

        Ok((Self::dir_to_value(path, &mut offset, &mut list_of_paths)?, list_of_paths))
    }

    
    // Auxiliary function
    fn dir_to_value<P: AsRef<Path>>(path: P, offset: &mut u64, list: &mut Vec<(PathBuf, u64)>) -> Result<Value, asar_error::Error> {
        let mut result = Map::new(); //result -> will be object
        
        let path = path.as_ref(); //current path

        let metadata = path.metadata()?;

        if metadata.is_dir() { //add folder and recurse 

            let mut folder_content = Map::new();

            for entry in fs::read_dir(path)? {
                let entry = entry?;
                folder_content.insert(entry.file_name().to_str().unwrap().to_string(), Self::dir_to_value(entry.path(), offset, list)?);
            }

            result.insert("files".to_string(), Value::Object(folder_content));

        } else if metadata.is_file() { //add file

            result.insert("size".to_string(), json!(metadata.len()));
            result.insert("offset".to_string(), Value::String(offset.to_string()));

            // push relevant data to list
            list.push((path.to_path_buf(), metadata.len()));

            *offset += metadata.len();
        }

        Ok(Value::Object(result))
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
            .asar_to_dir(destination, &file, self.start)?;

        Ok(())
    }


    ///
    /// 
    
    pub fn pack<P: AsRef<Path>>(&self, destination: P) -> Result<(), asar_error::Error> {


        if destination.as_ref().try_exists()? {
            remove_file(&destination)?;
        }


        let mut asar = OpenOptions::new().create(true).append(true).open(destination)?;

        
        let mut header_value: Vec<u8> = {
            if let Some(header) = &self.header {
                //println!("{:?}", header);
                serde_json::to_vec(header)?
            } else {
                return Err(Error::UnknownContentType("Can not have Asar archive file open".to_string()))
            }
        };

        //println!("len: {}\n", header_value.len());

        //write to shit
        asar.write_u32::<LittleEndian>(4 as u32)?;

        //println!("{}", self.start);


        asar.write_u32::<LittleEndian>((self.start - 8) as u32)?;
        asar.write_u32::<LittleEndian>((self.start - 12) as u32)?;
        asar.write_u32::<LittleEndian>((self.start - 16) as u32)?;

        asar.write_all(&mut header_value)?;

        self.content.dir_to_asar(&mut asar)

        /*let mut header_meta: Vec<u8> = Vec::new();

        header_meta.append(&mut (4 as u32).to_le_bytes().to_vec());
        header_meta.append(&mut ((self.start - 8) as u32).to_le_bytes().to_vec());
        header_meta.append(&mut ((self.start - 12) as u32).to_le_bytes().to_vec());
        header_meta.append(&mut ((self.start - 16) as u32).to_le_bytes().to_vec());
        header_meta.append(&mut header_value);

        
        
        asar.write_all(&header_meta)?;

        self.content.dir_to_asar(&mut asar)?;*/

        //Ok(())
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
