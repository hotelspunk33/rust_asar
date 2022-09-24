

//======content.rs======
//Content contains necessary functions and stuctures for asar.rs

//Content can be:
//  File(name, offset, size)
//  Folder(name, map)
//  Home(map)
//  Error



use std::{path::{PathBuf, Path}, fs::{File, DirBuilder}, io::Write};

use byteorder::LittleEndian;
use positioned_io::{ByteIo, ReadAt};
use serde_json::{json, Map, Value};

const MAX_SAFE_INTEGER: u64 = 9007199254740991;


#[derive(Debug)]
pub enum Content {
    File(String, u64, u64),
    Folder(String, Map<String, Value>),
    Home(Map<String, Value>),
    Error
}


impl Content {

    // Desc: Creates a new Content enum from a serde_json::Value enum
    // @param: value: &serde_json::Value
    // returns Content enum, Content::Error on error
    pub fn new(value: &Value) -> Content {
        if let Value::Object(map) = value {
            return lookahead("", map);
        }
        Content::Error
    }

    // Desc: Creates a Vector of type PathBuf containing all file and dir paths in the Content
    // @param: &self: &Content
    // returns Vec<PathBuf>, empty Vec on error
    pub fn paths_to_vec(&self) -> Vec<PathBuf> {
        let mut vec: Vec<PathBuf> = Vec::new();
        paths_to_vec_aux(self, Path::new(""), &mut vec);
        vec
    }


    // Desc: Write a content from Asar (io) to dir
    // @param: ...
    //returns (), TODO add Result<>
    pub fn write_to_dir(&self, p: &Path, io: &ByteIo<File, LittleEndian>, start: u64) {
        match self {
            Content::Home(dir) => {
                for (k, v) in dir.iter() {
                    if let Value::Object(map) = v {
                        DirBuilder::new().recursive(true).create(p).unwrap(); //experimental false
                        lookahead(k, map).write_to_dir(p, io, start)
                    }
                }
            },
            Content::Folder(name, dir) => {
                let path = {
                    let mut path = p.to_path_buf();
                    path.push(name);
                    path
                };
                DirBuilder::new().recursive(true).create(&path).unwrap();
                for (k, v) in dir.iter() {
                    if let Value::Object(map) = v {
                        lookahead(k, map).write_to_dir(path.as_path(), io, start)
                    }
                }
            },
            Content::File(name, offset, size) => {
                let path = {
                    let mut path = p.to_path_buf();
                    path.push(name);
                    path
                };
                //action here:
                //create file in respective dir p
                let mut v: Vec<u8> = vec![0; *size as usize];
                io.read_exact_at(start + offset, &mut v).unwrap();
    
                let mut file = File::create(&path).expect("Error creating file");
    
                file.write_all(&v).expect("Error writing to disk");
            },
            Content::Error => ()//panic!("Error: Expected content of Home, File, or Folder")
        }
        ()
    }

    //Desc: finds a the first file in Content with file_name
    //TODO: Add optional path to implement
    pub fn find(&self, file_name: &str, _path: Option<&Path>) -> Option<Content> {
        match self {
            Content::Home(dir) => {
                for (k, v) in dir.iter() {
                    if let Value::Object(map) = v {
                        let f = lookahead(k, map).find(file_name, None);
                        
                        if f.is_some() {
                            return f;
                        }
                    }
                }
            },
            Content::Folder(name, dir) => {
                for (k, v) in dir.iter() {
                    if let Value::Object(map) = v {
                        let f = lookahead(k, map).find(name, None);
                        
                        if f.is_some() {
                            return f;
                        }
                    }
                }
            },
            Content::File(name, offset, size) => {
                if file_name == name {
                    return Some(Content::File(name.to_string(), *offset, *size));
                }
            },
            _ => {}
        }
        None
    }

}







//Associated Functions:

fn lookahead(tok: &str, m: &serde_json::Map<String, Value>) -> Content {
    if tok.is_empty() { //value is either home or error
        if let Some(Value::Object(h)) = m.get("files") {
            return Content::Home(h.clone());
        } else {
            return Content::Error;
        }
    }
    //check if "offset" & "size" are included:
    match (m.get("offset"), m.get("size")) {
        (Some(Value::String(offset)), Some(Value::Number(size))) => { //v1 = offset, v2 = number.as_u64()
            if size.as_u64().unwrap() > MAX_SAFE_INTEGER {
                panic!("Error: size of {} is greater than MAX_SAFE_INTEGER", tok);
            }
            return Content::File(tok.to_string(), offset.parse::<u64>().unwrap(), size.as_u64().unwrap());
        },
        _ => {//offset and size not found in lookahead, check for files
            if let Some(Value::Object(f)) = m.get("files") {
                return Content::Folder(tok.to_string(), f.clone());
            }
        }
    }
    //lookahead is not a file, folder, or home... must be an error
    Content::Error
}




fn paths_to_vec_aux(c: &Content, p: &Path, vec: &mut Vec<PathBuf>) -> () {
    match c {
        Content::Home(dir) => {
            for (k, v) in dir.iter() {
                if let Value::Object(map) = v {
                    paths_to_vec_aux(&lookahead(k, map), p, vec);
                }
            }
        },
        Content::Folder(name, dir) => {
            let path_buf = {
                let mut path = p.to_path_buf();
                path.push(name);
                path
            };

            vec.push(path_buf.clone()); //add folder to vec
            
            for (k, v) in dir.iter() {
                if let Value::Object(map) = v {
                    paths_to_vec_aux(&lookahead(k, map), path_buf.as_path(), vec);
                }
            }
        },
        Content::File(name, _offset, _size) => {
            let path_buf = {
                let mut path = p.to_path_buf();
                path.push(name);
                path
            };
            
            vec.push(path_buf); //add file to vec
        },
        _ => ()
    }
    ()
}