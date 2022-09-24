
use std::{path::Path, fs::File};

use byteorder::LittleEndian;
use positioned_io::{ByteIo, ReadBytesExt, ReadAt};

use crate::content::Content;

#[derive(Copy, Clone, Debug)]
enum State {
    Asar,
    Dir
}


pub struct AsarBuilder {
    archive_name: Option<String>,
    dir_name: Option<String>,
    state: Option<State>
}

impl AsarBuilder {
    pub fn new() -> AsarBuilder {
        AsarBuilder {archive_name: None, dir_name: None, state: None}
    }

    pub fn set_dir(mut self, name: &str) -> AsarBuilder {
        self.dir_name = Some(name.to_string());
        self
    }

    pub fn set_archive(mut self, name: &str) -> AsarBuilder {
        self.archive_name = Some(name.to_string());
        self
    }

    pub fn open_asar(mut self) -> AsarBuilder {
        self.state = Some(State::Asar); 
        self
    }

    pub fn open_dir(mut self) -> AsarBuilder {
        self.state = Some(State::Dir);
        self
    }

    pub fn build(self) -> Result<Asar, AsarBuilder> {
        match (&self.archive_name, &self.dir_name, self.state) {
            (Some(archive_name), Some(dir_name), Some(state)) => {
                let (content, start): (Content, u64) = {
                    if let State::Asar = state { //get the correct content
                        let file = File::open(Path::new(archive_name)).unwrap();
                        let io: ByteIo<_, LittleEndian> = ByteIo::new(file);
                        
                        let json_len = io.read_u32_at::<LittleEndian>(12).unwrap();
                        let mut json_u8: Vec<u8> = vec![0; json_len as usize];
                        let _ = io.read_exact_at(16, &mut json_u8);
                        
                        let value = serde_json::from_slice(&json_u8).expect("Error parsing json...");
                        let start = {
                            let header_len = io.read_u32_at::<LittleEndian>(8).unwrap();
                            (header_len + 12) as u64
                        };
                        (Content::new(&value), start)
                    } else {
                        return Err(self);
                    }
                };
                
                Ok(Asar {
                    archive_name: archive_name.to_string(),
                    dir_name: dir_name.to_string(),
                    state: state,
                    content: content,
                    start: start
                })
            },
            _ => Err(self)
        }
    }
}



#[derive(Debug)]
pub struct Asar {
    dir_name: String,
    archive_name: String,
    state: State,
    content: Content,
    start: u64,
}

impl Asar {
    pub fn list(&self) -> Vec<String> {
        //map is cool
        
        self.content.paths_to_vec().iter().map(
            |p| String::from(p.to_str().expect("Unsupported Path")))
            .collect::<Vec<String>>()
    }

    pub fn extract(&self) {
        let io = {
            let file = File::open(Path::new(&self.archive_name)).expect("Error opening archive");
            let io: ByteIo<File, LittleEndian> = ByteIo::new(file);
            io
        };
        
        //self.content.write(Path::new(self.dir_name), io, self.start);
        self.content.write_to_dir(Path::new(&self.dir_name), &io, self.start)
    }


    pub fn get_file(&self, name: &str) -> Option<Vec<u8>> {
        if let State::Asar = self.state {
            if let Some(Content::File(_name, offset, size)) = self.content.find(name, None) {
                let bytes = {
                    let file = File::open(Path::new(&self.archive_name)).expect("TODO error msg");
                    let io: ByteIo<_, LittleEndian> = ByteIo::new(file);
                    
                    let mut v: Vec<u8> = vec![0; size as usize];
                    io.read_exact_at(self.start + offset, &mut v).expect("Error reading archive");
                    
                    v
                };
                return Some(bytes);
            }
        }
        None
    }
}