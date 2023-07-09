//gotta fix this real quick
//add param: lst: &mut Vec<(PathBuf, u64)> -> add each file_path and offset to vec
/*pub fn gen_value_from_dir(dir: &Path, offset: &mut u64) -> serde_json::Value { //design err: offset is unnecessarily mut
    if let Ok(entries) = fs::read_dir(dir) {
        let mut map: Map<String, Value> = Map::new(); //TODO: Here
        
        for entry in entries { //bad doesnt work.. remove
            if let Ok(entry) = entry {
                if let Ok(metadata) = entry.metadata() {//metadata obtained
                    if metadata.is_dir() { //entry is a folder
                        return Value::Object({ //TODO add a loop here:
                            let mut m: Map<String, Value> = Map::new();
                            m.insert(String::from(entry.file_name().to_str().expect("Error 1 - 4434")), Value::Object({
                                let mut files: Map<String, Value> = Map::new();
                                files.insert("files".to_string(), gen_value_from_dir(entry.path().as_path(), offset));
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
}*/

/* 
=====Notes=====
- If write to disk at the same time as generating the header, 
  will have to shift data in file later... slower
- could load bytes into memory, but could be a problem with larg files
- solution: do separately -> gen header than iterate through json value and write
*/



//fix: iterate on every entry in a folder, recursing on folders within the folder
pub fn gen_value_from_dir(dir: &Path, offset: &mut u64) -> serde_json::Value { //design err: offset is unnecessarily mut
    //determine if dir is directory or file
    let mut map = Map::new(); //map to be returned...
    let metadata = fs::metadata(dir).unwrap();
    if metadata.is_dir() { //is a folder
        
        let mut m = Map::new();
        
        for entry in fs::read_dir(dir).unwrap() {
            if let Ok(entry) = entry {
                m.insert(String::from(entry.file_name().to_str().expect("error: filename")), 
                gen_value_from_dir(entry.path().as_path(), offset));
            } else {
                return Value::Null;
            }
        }
        map.insert("files".to_owned(), Value::Object(m));
    } else { //must be a file
        return Value::Object({ // files :{} //fix os_str
            let mut details: Map<String, Value> = Map::new();// interesting below...
            details.insert("size".to_string(), json!(metadata.len() as u64)); //size: Number(size)
            details.insert("offset".to_string(), serde_json::Value::String(offset.to_string())); //offset: String(offset)
            *offset += metadata.len(); //update offset for the rest of files
            details
            
        });
    }
    
    Value::Object(map)
}

/* 
fn gen_value_from_dir_helper(entry: DirEntry, offset: u64) -> Value {
    
    if let Ok(metadata) = entry.metadata() {//metadata obtained
        if metadata.is_dir() { //entry is a folder
            return Value::Object({ //TODO add a loop here:
                let mut m: Map<String, Value> = Map::new();
                m.insert(String::from(entry.file_name().to_str().expect("Error 1 - 4434")), Value::Object({
                    let mut files: Map<String, Value> = Map::new();
                    files.insert("files".to_string(), gen_value_from_dir(entry.path().as_path(), offset));
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
    
}*/

pub fn get_header(json: &Value) -> Vec<u8> {
    let mut json_bytes = serde_json::to_vec(json).expect("Error parsing Value to bytes");
    let len = json_bytes.len() as u32;
    let size = len + (8 - len % 8);
    
    let mut result: Vec<u8> = (4 as u32).to_le_bytes().to_vec(); //experimental
    result.append(&mut (size + 8 as u32).to_le_bytes().to_vec());
    result.append(&mut (size + 4 as u32).to_le_bytes().to_vec());
    result.append(&mut len.to_le_bytes().to_vec());
    result.append(&mut json_bytes);
    //print!("Debug: {} ; {}", len, size);

    result
}

//
pub fn write_to_asar(value: &Value, path: &Path, start: u64, io: &mut ByteIo<File, LittleEndian>) -> bool {
    if let Value::Object(map) = value {
        //check each entry's k (k, v) if file or dir:
        if map.contains_key("files") && map.len() == 1 { //files means directory
            //iterate through directory:
            if let Value::Object(m) = &map["files"] {
                for (k, v) in m.into_iter() {
                    write_to_asar(v, path.join(k).as_path(), start, io);
                }
            }
        } else if map.contains_key("size") && map.contains_key("offset") { //change json value as needed
            //write to file with bytes of path
            //let mut v: Vec<u8> = vec![0; *size as usize];
            let pos:u64 = start + map["offset"].as_str().unwrap().parse::<u64>().unwrap();
            let mut file = File::open(path).unwrap();
            let mut v: Vec<u8> = vec![0; map["size"].as_u64().unwrap() as usize];
            file.read(&mut v).expect("Error reading file");
            io.write_at(pos, &v).expect("Error writing to asar");
            //println!("[Debug]: size: {}, offset: {}, path: {}", map["size"], map["offset"], path.to_str().unwrap());
        }
    } else {
        return false;
    }


    true
}