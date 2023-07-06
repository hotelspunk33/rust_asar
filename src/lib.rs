

pub mod asar;
pub mod asar_error;
pub mod content;



#[cfg(test)]
mod tests {
    use std::{
        fs::File,
        io::{BufReader, Read},
        path::Path,
    };

    use byteorder::LittleEndian;
    use positioned_io::{ReadAt, ReadBytesExt};
    use serde_json::{Value};

    use crate::{asar::Asar, content::Content};

    #[test]
    fn test_header_1() {
        let file = File::open("test_header.json").unwrap();
        let reader = BufReader::new(file);

        let dummy: Content = {
            let header_json: Value = serde_json::from_reader(reader).unwrap();
            Content::new(header_json).unwrap()
        };

        //println!("test_header_1: dummy created");

        let paths = dummy.paths_to_vec().unwrap();

        assert!(paths.contains(&Path::new("folder1").to_path_buf()));
        assert!(paths.contains(&Path::new("folder1/script.py").to_path_buf()));
        assert!(paths.contains(&Path::new("test1.txt").to_path_buf()));
        assert!(paths.contains(&Path::new("folder1/test_image.jpg").to_path_buf()));
        assert_eq!(paths.len(), 4);

        
        // testing value generated for directory

        let val = Asar::gen_header_from_dir("test_folder");

        let content = Content::new(val.unwrap().0);

        assert!(content.is_ok());
        let paths = content.unwrap().paths_to_vec().unwrap();

        assert!(paths.contains(&Path::new("folder1").to_path_buf()));
        assert!(paths.contains(&Path::new("folder1/script.py").to_path_buf()));
        assert!(paths.contains(&Path::new("test1.txt").to_path_buf()));
        assert!(paths.contains(&Path::new("folder1/test_image.jpg").to_path_buf()));
        assert_eq!(paths.len(), 4);


        
    }

    #[test]
    fn test_header_2() {
        let file = File::open("test_header.json").unwrap();
        let reader = BufReader::new(file);

        let dummy: Content = {
            let header_json: Value = serde_json::from_reader(reader).unwrap();
            Content::new(header_json).unwrap()
        };

        let content = dummy.find(Path::new("test1.txt")).unwrap();

        if let Content::File(name, offset, size) = content {
            assert_eq!(name, Path::new("test1.txt").to_path_buf());
            assert_eq!(offset, 30023 as u64);
            assert_eq!(size, 21 as u64);
        }

        let content = dummy.find(Path::new("folder1/test_image.jpg")).unwrap();

        if let Content::File(name, offset, size) = content {
            assert_eq!(name, Path::new("test_image.jpg").to_path_buf());
            assert_eq!(offset, 55 as u64);
            assert_eq!(size, 29968 as u64);
        }

        let content = dummy.find(Path::new("folder1")).unwrap();

        if let Content::Folder(name, _) = content {
            assert_eq!(name, Path::new("folder1").to_path_buf());
            // rest is assumed
        }

        assert_eq!(dummy.find(Path::new("")).unwrap(), dummy);

        assert!(dummy.find(Path::new("test")).is_none());
    }

    #[test]
    fn test_asar1() {
        // tests opening asar archive
        let file = File::open("test_asar.asar").unwrap();

        const JSON_LEN_OFFSET: u64 = 12;
        const JSON_OFFSET: u64 = 16;
        const HEADER_LEN_OFFSET: u64 = 8;
        let json_len = file.read_u32_at::<LittleEndian>(JSON_LEN_OFFSET).unwrap();
        let mut json_u8: Vec<u8> = vec![0; json_len as usize];

        file.read_exact_at(JSON_OFFSET, &mut json_u8).unwrap();

        //let value = serde_json::to_value(json_u8).unwrap();
        let value: Value = serde_json::from_slice(&json_u8).unwrap();

        let start = {
            // 12 bytes prior to header must be included
            (file.read_u32_at::<LittleEndian>(HEADER_LEN_OFFSET).unwrap() + 12) as u64
        };

        let mut file = File::open("test_header.json").unwrap();
        let mut buf: Vec<u8> = vec![0; file.metadata().unwrap().len() as usize];
        file.read(&mut buf).unwrap();

        assert_eq!(value, serde_json::from_slice::<Value>(&buf).unwrap());
        assert_eq!(start, 796);
    }

    #[test]
    fn test_asar2() { // tests reading file names from archive
        let asar = Asar::open(Path::new("test_asar.asar")).unwrap();
        let list = asar.list().unwrap();

        assert!(list.contains(&"folder1".to_string()));
        assert!(list.contains(&"folder1/script.py".to_string()));
        assert!(list.contains(&"folder1/test_image.jpg".to_string()));
        assert!(list.contains(&"test1.txt".to_string()));
        assert_eq!(list.len(), 4);

        let list = asar.get_paths_contain("test");
        assert!(list.contains(&Path::new("test1.txt").to_path_buf()));
        assert!(list.contains(&Path::new("folder1/test_image.jpg").to_path_buf()));
        assert_eq!(list.len(), 2);

        //test reading file names from directory

        let asar = Asar::open(Path::new("test_folder")).unwrap();
        let list = asar.list().unwrap();

        assert!(list.contains(&"folder1".to_string()));
        assert!(list.contains(&"folder1/script.py".to_string()));
        assert!(list.contains(&"folder1/test_image.jpg".to_string()));
        assert!(list.contains(&"test1.txt".to_string()));
        assert_eq!(list.len(), 4);

        let list = asar.get_paths_contain("test");
        assert!(list.contains(&Path::new("test1.txt").to_path_buf()));
        assert!(list.contains(&Path::new("folder1/test_image.jpg").to_path_buf()));
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn test_asar3() { // tests reading file contents from archive
        let asar = Asar::open(Path::new("test_asar.asar")).unwrap();


        { // test get_file()
            let mut file = File::open("test_folder/folder1/test_image.jpg").unwrap();
            let mut buf: Vec<u8> = vec![0; file.metadata().unwrap().len() as usize];
            file.read(&mut buf).unwrap();

            assert_eq!(asar.get_file(Path::new("folder1/test_image.jpg")).unwrap(), buf);
            assert!(asar.get_file("folder1").is_none());
        };

        
        // test extract()
        asar.extract(Path::new("test_extract")).unwrap();

        let files = ["test1.txt", "folder1/script.py", "folder1/test_image.jpg"];

        for file in files {
            let test = {
                let mut f = File::open(Path::new("test_extract").join(file)).unwrap();
                let mut buf: Vec<u8> = vec![0; f.metadata().unwrap().len() as usize];
                f.read(&mut buf).unwrap();
                buf
            };

            let correct = {
                let mut f = File::open(Path::new("test_folder").join(file)).unwrap();
                let mut buf: Vec<u8> = vec![0; f.metadata().unwrap().len() as usize];
                f.read(&mut buf).unwrap();
                buf
            };

            assert_eq!(test, correct);
        }


    }
}

// TODO:
// - change path args to AsRef<Path> from &Path
