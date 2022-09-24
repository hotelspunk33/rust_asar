use content::Content;
use asar::{Asar, AsarBuilder};

pub mod content;
pub mod asar;
pub mod test;



fn main() {
    //test_extract();
   if test_list() {
    if test_get_file() {
        if test_extract() {
            println!("===Success===");
        }
    }
   }
    
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