# rust_asar

Library to create, modify, and extract Asar archives, as does Electron's Asar [library](https://github.com/electron/asar).

> I implemented this project with no inspiration from Electron's library, as the goal was not to create a rewrite but to reverse-engineer the Asar archive.

------------

## Documentation

View the documentation [here](https://hotelspunk33.github.io/rust_asar/).

### Examples

Additionally, refer to the test cases in src/lib.rs as an example.

```Rust
// Extracting an Asar archive file:
let asar: Asar = Asar::open("app.asar").unwrap();

{
    // Additional functions available only for opened archive files:
    let list_of_paths: Vec<String> = asar.list().unwrap();

    let test1: Vec<u8> = asar.get_file("test1.txt").unwrap();

    // etc.. see documentation
}

if asar.extract("test_extract").is_ok() {
    println!("Done.");
} else {
    println!("Failed.");
}

// Packing a directory into an Asar archive file:
let dir: Asar = Asar::open("app").unwrap();

if asar.pack("app.asar").is_ok() {
    println!("Done.");
} else {
    println!("Failed.");
}
```

### Asar Archive Represented Structure

The Content enum keeps track of an asar file's internal structure, represented by
Files, Folders, and Home (the starting directory / base case).

The Content enum for an Asar archive recursively consists of:

`File   (name, offset, size)`    -> `File   (PathBuf, u64, u64)`

`Folder (name, folder_contents)` -> `Folder (PathBuf, Map<String, Value>)`

`Home   (asar_contents)`         -> `Home   (Map<String, Value>)`

The Content enum for an opened directory only consists of:

`List (files)` -> `List (Vec<(PathBuf, u64)>)`

- Where files represents full file paths and file sizes.

------------

## Idea

The Asar archive file is a flat archive file that concatenates files together, allowing for random file access.

The file format is quite simple, as it is super flat and encoded in bytes.

The header of an Asar archive file contains the size of the header, along with offsets that point toward where the file contents begin (after the header), which can also be derived from the header size.

```text
Header Size (length of JSON + padding) | JSON Length | JSON value| File Contents
```

The JSON value represents the file structure within the Asar archive.

A simple example:

```json

{
    "files": {
        "folder1": {
            "files": {
                "script.py": {
                    "size": 55,
                    "offset": "0",
                },
                "test_image.jpg": {
                    "size": 29968,
                    "offset": "55",
                }
            }
        },
        "test1.txt": {
            "size": 21,
            "offset": "30023",
        }
    }
}
```

> Integrity, symbolic links, and executables have not been implemented, so such functionality is not shown in this example.

------------

## Interoperability

This library is not compatible with modern versions of the Asar archive. The full functionality of the Asar file is not implemented, as rust_asar serves its purpose more as a proof-of-concept than a stable and usable library.

The following features have yet to be implemented:

- file integrity (algorithm, hash, blockSize, blocks)

- executable functionality for Linux and Mac

- symbolic link support

> Missing functionality should not be difficult to implement if needed in the future.

------------

## TODO

> This project is on hold for now, and I am not entirely sure I will continue.
> Regardless here are some ideas that I thought about but have not acted on (all of them are simple but tedious).

- Only use Rust's standard library for filesystems and input/output.
- Optimize organization Asar archive- the current use of serde_json library is inefficient.
- Stop loading entire archive into memory- this should be a priority...
- Implement iterators for the Content enum.
- Rewrite error handling, it's not so hot rn
- Create fold and map functions for Asar enum and use for (practically) all current functions.
- Fix Asar archive header to be interoperable with modern Asar archives.
