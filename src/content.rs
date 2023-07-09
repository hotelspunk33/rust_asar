use std::{
    fs::{DirBuilder, File},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use positioned_io::ReadAt;
use serde_json::{Map, Value};

use crate::asar_error::{self, Error};

/// The maximum size of a file within an asar archive.
const MAX_SAFE_INTEGER: u64 = 9007199254740991; //for compatability with Electron's Asar library

/// Content enum keeps track of an asar file's internal structure, represented by
/// Files, Folders, and Home (the starting directory) for an Asar archive.
/// 
/// The List varient is reserved for opening folders as the source path.
///
/// The asar structure recursively consists of:
///
/// `File   (name, offset, size)`    -> `File   (PathBuf, u64, u64)`
///
/// `Folder (name, folder_contents)` -> `Folder (PathBuf, Map<String, Value>)`
///
/// `Home   (asar_contents)`         -> `Home   (Map<String, Value>)`
/// 
/// `List   (Vec<(path, size>)`    -> 'List   (Vec<(PathBuf, u64)>)'
///
/// Where:
///
/// - name (PathBuf):  The respective name of the content type -> PathBuf
///
/// - offset   (u64):  The offset in the Asar archive file at which the File content symbolically exists
///
/// - size     (u64):  The size of the File content
///
/// - folder_contents  (Map<String, Value>):  Represents the inside contents within a Folder content
///
/// - asar_contents    (Map<String, Value>):  Represents the inside contents of the base folder (base case)
/// 
/// - path (PathBuf):  The full path of a file that will be added to Asar archive file
/// 

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Content {
    File(PathBuf, u64, u64),             // (name, offset, size)
    Folder(PathBuf, Map<String, Value>), // (name, folder_content)
    Home(Map<String, Value>),            // (asar_content)
    List(Vec<(PathBuf, u64)>),           // (Listof (full_file_path, size))
}

impl Content {

    /// Instantiates a Content enum given the one argument provided of type `serde_json::Value`.
    ///
    /// The parameter `header` represents a JSON value found as the header in an Asar archive.
    /// Thus, used to enumerate internal strucuture of Asar archive file.
    ///
    /// Do not use with opened folder.
    ///
    /// Returns instantiated Content enum, otherwise Error.

    pub fn new_json(header: Value) -> Result<Content, asar_error::Error> {
        if let Value::Object(item) = header {
            Ok(lookahead("", &item)?)
        } else {
            Err(Error::UnknownContentType(
                "Expected Map<String, Value> for new content type".to_string(),
            ))
        }
    }


    /// Instantiates a Content enum given the one argument provided,
    /// representing a list of files and their size.
    ///
    /// - `(PathBuf, u64)` -> `(full file path, length of file)`
    ///
    /// This specific varient represents the files of a directory to be concatenated on to an Asar archive file.
    /// Due to this, functionality is limited to only the `dir_to_asar` function.
    /// 
    ///
    /// Do not use with opened Asar archive file.
    ///
    /// Returns the List varient of the Content enum.

    pub fn new_list(list: Vec<(PathBuf, u64)>) -> Content {
        Content::List(list)
    }


    /// Returns a vector of PathBufs representing all files and folders within Asar archive,
    /// otherwise an Error.

    pub fn paths_to_vec(&self) -> Result<Vec<PathBuf>, asar_error::Error> {
        let mut vec: Vec<PathBuf> = Vec::new(); //problematic for concurrency

        match self {
            Content::Home(dir) => {
                for (item_name, item_content) in dir.into_iter() {
                    //item_name

                    if let Value::Object(item) = item_content {
                        let next_content = lookahead(item_name, item)?;
                        paths_to_vec_aux(&next_content, Path::new(""), &mut vec)?;
                    }
                }
            }
            _ => {
                return Err(asar_error::Error::UnknownContentType(
                    "Unexpected Content Type: expected Content::Home".to_string(),
                ))
            }
        }

        Ok(vec)
    }
    

    /// Writes the files and folders of current Content enum to the provided base_path folder.
    ///
    /// - base_path: the destination folder (home folder) where archive content will be written
    ///
    /// - file: the Asar archive file to obtain the Content to be written
    ///
    /// - start: the offset at which the file content start within Asar archive file
    ///
    /// > The Asar archive file must be passed in the `file` parameter,
    /// otherwise unintended behavior may occur.
    ///
    /// Returns (), otherwise Error.

    pub fn asar_to_dir<P: AsRef<Path>>(
        &self,
        base_path: P,
        file: &File,
        start: u64,
    ) -> Result<(), asar_error::Error> {
        let base_path = base_path.as_ref();

        match self {
            // Create folder for home directory of Asar
            Content::Home(dir) => {
                for (name, value) in dir.iter() {
                    if let Value::Object(content) = value {
                        //cast
                        DirBuilder::new().recursive(true).create(base_path)?; //Create parent directory
                        lookahead(name, content)?.asar_to_dir(base_path, file, start)?;
                    }
                }

                Ok(())
            }

            // Create folder
            Content::Folder(name, dir) => {
                let path = base_path.join(name);
                DirBuilder::new().recursive(true).create(&path)?; //create folder

                for (name, value) in dir.iter() {
                    if let Value::Object(content) = value {
                        lookahead(name, content)?.asar_to_dir(path.as_path(), file, start)?;
                    }
                }

                Ok(())
            }

            //create file
            Content::File(name, offset, size) => {
                let path = base_path.join(name);

                let mut file_as_vec: Vec<u8> = vec![0; *size as usize]; //init vec of bytes for file
                                                                        //io.read_exact_at(start + offset, &mut file_as_vec)?;
                file.read_exact_at(start + offset, &mut file_as_vec)?;

                let mut file = File::create(&path)?;
                file.write_all(&file_as_vec)?; //write file to fs

                Ok(())
            }

            _ => Err(Error::UnknownContentType(format!(
                "Asar archive file must be src_path"
            ))),
        }
    }


    /// Concatenates all files (recursively) of a directory to the Asar archive file provided.
    /// 
    /// Files are represented by vec in Content::List(vec).
    /// 
    /// Takes in one argument of type `&mut File`, which must be the Asar archive file in creation.
    /// > The Asar archive file must have its header written prior to this function call, 
    /// as only files are concatenated.
    /// 

    pub fn dir_to_asar(&self, asar: &mut File) -> Result<(), asar_error::Error> {
        if let Content::List(paths) = &self {
            for (path, size) in paths {
                let mut buf: Vec<u8> = vec![0; *size as usize];

                {
                    let mut file = File::open(path)?;

                    file.read_to_end(&mut buf)?;
                }

                //write to asar...
                asar.write_all(&buf)?;
            }

            return Ok(());
        }

        Err(Error::UnknownContentType(format!(
            "Folder must be src_path, Asar archive file found"
        )))
    }


    /// Searches for a file by its full path name provided by the parameter `path`.
    ///
    /// Asar archive file must be opened.
    ///
    /// `None` will be returned if a directory is opened.
    ///
    /// Returns the Content enum of the `path` if found, otherwise `None`.
    /// > All Content types are valid to be returned.

    pub fn find<P>(&self, path: P) -> Option<Content>
    where
        P: AsRef<Path>,
    {
        // Recursively finds path in content
        fn find_aux<P>(content: &Content, path: P, curr_path: &Path) -> Option<Content>
        where
            P: AsRef<Path>,
        {
            match content {
                Content::Home(dir) => {
                    if let None = path.as_ref().file_stem() {
                        // path is home
                        return Some(content.clone());
                        //return Some(Content::Home(dir.clone()));
                    } else {
                        // iterate through home directory
                        for (name, object) in dir.into_iter() {
                            if path.as_ref().starts_with(curr_path.join(name)) {
                                //dont like might change
                                // check if item is correct
                                if let Value::Object(item) = object {
                                    return find_aux(
                                        &lookahead(&name, &item).unwrap(),
                                        path,
                                        curr_path,
                                    );
                                }
                            }
                        }

                        return None;
                    }
                }

                Content::File(name, _, _) => {
                    if path.as_ref().eq(curr_path.join(name).as_path()) {
                        return Some(content.clone());
                        //return Some(Content::File(*name, *offset, *size));
                    } else {
                        None
                    }
                }

                Content::Folder(name, dir) => {
                    let curr_path = curr_path.join(name);

                    if curr_path.eq(path.as_ref()) {
                        return Some(content.clone());
                    }

                    // iterate through folder to find next correct element
                    for (name, object) in dir.into_iter() {
                        if path.as_ref().starts_with(curr_path.join(name)) {
                            if let Value::Object(item) = object {
                                return find_aux(
                                    &lookahead(&name, &item).unwrap(),
                                    path,
                                    curr_path.as_path(),
                                );
                            }
                        }
                    }
                    None
                }

                _ => None,
            }
        }

        find_aux(self, path, Path::new(""))
        //None
    }
}

/// Returns the content value based on the paramters given.
/// Receives a name of type string slice and item of Serde Map,
/// returning a named Content value after parsing the item.

fn lookahead(
    name: &str,
    item: &serde_json::Map<String, Value>,
) -> Result<Content, asar_error::Error> {
    if name.is_empty() {
        //value is either home
        if let Some(Value::Object(dir)) = item.get("files") {
            return Ok(Content::Home(dir.clone()));
        } else {
            return Err(asar_error::Error::ParseHeaderError(
                "'files' not found in Home directory".to_string(),
            ));
        }
    }

    //check if "offset" & "size" are included:
    match (item.get("offset"), item.get("size")) {
        (Some(Value::String(offset)), Some(Value::Number(size))) => {
            let size = size.as_u64();

            if size.is_none() {
                return Err(asar_error::Error::ParseHeaderError(format!(
                    "size nan for file: {}",
                    name
                )));
            }

            let size = size.unwrap();

            // check for max integer size
            if size > MAX_SAFE_INTEGER {
                return Err(asar_error::Error::ParseHeaderError(format!(
                    "size of {} is greater than MAX_SAFE_INTEGER",
                    name
                )));
            }

            let offset = offset.parse::<u64>()?;

            //Path::new(name).to_path_buf()

            return Ok(Content::File(
                PathBuf::new().join(name), //experimental
                offset,
                size,
            ));
        }

        _ => {
            //offset and size not found in lookahead, check for files

            if let Some(Value::Object(dir)) = item.get("files") {
                return Ok(Content::Folder(PathBuf::new().join(name), dir.clone()));
            } else {
                return Err(asar_error::Error::ParseHeaderError(format!(
                    "Error parsing header for entity: {}",
                    name
                )));
            }
        }
    }
}

fn paths_to_vec_aux(
    content: &Content,
    path: &Path,
    vec: &mut Vec<PathBuf>,
) -> Result<(), asar_error::Error> {
    match &content {
        Content::Folder(name, dir) => {
            let path = path.join(name);

            vec.push(path.clone()); //add folder to vec

            for (name, object) in dir.iter() {
                if let Value::Object(content) = object {
                    let next_content = lookahead(name, content)?;
                    paths_to_vec_aux(&next_content, path.as_path(), vec)?;
                } else {
                    return Err(Error::UnknownContentType(
                        "Uknown content type, expected Object".to_string(),
                    )); //unecessary i think
                }
            }

            Ok(())
        }

        Content::File(name, _, _) => {
            let path = path.join(name);

            vec.push(path.clone());

            Ok(())
        }

        _ => Err(asar_error::Error::UnknownContentType(
            "Unexepcted Content Type".to_string(),
        )),
    }
    //Ok(())
}

// TODO: Implement fold for Content
