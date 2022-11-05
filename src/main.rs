use std::{fs::{File, OpenOptions}, io::Write};

use content::Content;
use asar::{Asar, AsarBuilder};
use positioned_io::{self, ByteIo, WriteAt};
use byteorder::LittleEndian;
//use by
use positioned_io::WriteBytesExt;

pub mod content;
pub mod asar;
pub mod test;



fn main() {
    //test_extract();
   /*if test_list() {
    if test_get_file() {
        if test_extract() {
            println!("===Success===");
        }
    }
   }*/


   let mut fisle = OpenOptions::new().write(true).create(true).open("foo.data").unwrap();
   fisle.write_u32_at::<LittleEndian>(1 << 20, 1234).unwrap();

   /* 
   let t = test::gen_value_from_dir("test1_dir".as_ref(), &mut 0);
   let header = test::get_header(&t);
   let mut file = OpenOptions::new().write(true).create(true).open("t1.asar").unwrap();
   //file.set_len((header.len() + 1) as u64).expect("error changing file size");
   file.write_at(0, &header).expect("error writing header");
   let mut io: ByteIo<File, byteorder::LittleEndian> = positioned_io::ByteIo::new(file);
   
   
   let s = (8 - (header.len() % 8) + header.len()) as u64;
   //test::write_to_asar(&t, "test1_dir".as_ref(), 0, &mut io);
   //print!("{:?}", test::get_header(&t));
    */
}

fn test_extract() -> bool {
    if let Ok(asar) = AsarBuilder::new().set_archive("test_asar_og.asar").set_dir("test1_dir").open_asar().build() {
        asar.extract();
        return true;

    } else {
        println!("[Test Extract]");
        return false;
    }
}

fn test_list() -> bool {
    let t = AsarBuilder::new();
    t.open_dir();
    t.set_archive("sdsd");

    if let Ok(asar) = AsarBuilder::new().set_archive("test_asar_og.asar").set_dir("test1_dir").open_asar().build() {
        if asar.list().is_empty() {
            return false;
        }

        for s in asar.list() {
            println!("{}", s);
        }
        return true;

    } else {
        println!("[Test List]");
        return false;
    }
}

fn test_get_file() -> bool {
    if let Ok(asar) = AsarBuilder::new().set_archive("test_asar_og.asar").set_dir("test1_dir").open_asar().build() {
        println!("{:?}", String::from_utf8(asar.get_file("panoptocmsc.txt").unwrap()));
        return true;

    } else {
        println!("[Test Get File]");
        return false;
    }
}