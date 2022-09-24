use std::{path::Path, fs::{self, File}};

use byteorder::LittleEndian;
use positioned_io::ByteIo;
use serde_json::{Map, Value};



//add param: lst: &mut Vec<(PathBuf, u64)> -> add each file_path and offset to vec
fn dir_to_value(dir: &Path, offset: &mut u64) -> serde_json::Value {
    if let Ok(entries) = fs::read_dir(dir) {
        let mut map: Map<String, Value> = Map::new();
        for entry in entries {
            if let Ok(entry) = entry {
                if let Ok(metadata) = entry.metadata() {//metadata obtained
                    if metadata.is_dir() { //entry is a folder
                        return Value::Object({ //TODO add a loop here:
                            let mut m: Map<String, Value> = Map::new();
                            m. insert(String::from(entry.file_name().to_str().expect("Error 1 - 4434")), Value::Object({
                                let mut files: Map<String, Value> = Map::new();
                                files.insert("files".to_string(), dir_to_value(entry.path().as_path(), offset));
                                files
                            }));
                            m
                        });
                    } else if metadata.is_file() { //entry is a file
                        return Value::Object({
                            let mut m: Map<String, Value> = Map::new();
                            m.insert(String::from(entry.file_name().to_str().expect("Error 1 - 4434")), Value::Object({ // files :{} //fix os_str
                                let mut details: Map<String, Value> = Map::new();// interesting below...
                                details.insert("size".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(metadata.len() as f64).unwrap())); //size: Number(size)
                                details.insert("offset".to_string(), serde_json::Value::String(offset.to_string())); //offset: String(offset)
                                *offset += metadata.len(); //update offset for the rest of files
                                details
                            }));
                            m
                        });
                    } else { //unsupported symbolic link
                        return Value::Null;
                    }
                }
            }
        }
        return Value::Object({
            let mut m: Map<String, Value> = Map::new();
            m.insert("files".to_string(), serde_json::Value::Object(map));
            m
        });
    }
    Value::Null
}


/*fn get_header(json: &Value) -> Vec<u8> {
    let mut json_bytes = serde_json::to_vec(json).expect("Error parsing Value to bytes");
    
    let mut result: Vec<u8> = (4 as u32).to_le_bytes().to_vec(); //experimental
    result.append((n as u32).to_le_bytes().as_mut_slice());
    result.append((n as u32).to_le_bytes().as_mut_slice());
    result.append(&mut (json_bytes.len() as u32).to_le_bytes().to_vec());
    result.append(&mut json_bytes);

    result
}*/


fn write_to_asar(map: &Map<String, Value>, path: &Path, start: u64, io: &mut ByteIo<File, LittleEndian>) {
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries {
            if let Ok(entry) = entry {
                if let Ok(metadata) = entry.metadata() {//metadata obtained
                    if metadata.is_dir() { //entry is a folder
                        if let serde_json::Value::Object(m) = map[&entry.file_name()]["files"] {
                            write_to_asar(&m, entry.path().as_path(), start, io);
                        }
                    } else if metadata.is_file() { //entry is a file
                        //write file
                        let bytes: Vec<u8> = {}; //read file bytes into ...
                        
                        let offset = { 
                            
                        };
                        
                        io.write_all_at(start + offset, &bytes); //TODO: Handle errors
                    } else { //unsupported symbolic link
                        panic!("something went wrong...");
                        //handle error or something
                    }
                }
            }
        }
    }
}