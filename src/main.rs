use std::{path::Path, collections::hash_map::DefaultHasher, hash::Hasher, io::Read};

use blake3::Hash;

fn main() {
    println!("Start media-scanner");
    let root_path = Path::new(".");
    let tree = build_tree(root_path, 0);
    for entry in tree.unwrap() {
        println!("{}", &entry);
    }
}

fn build_tree(directory: &Path, parent_level: u8) -> Option<Vec<Entry>> {
    if !directory.is_dir() {
        println!("Invalid Path:{:?}", directory);
        return None;
    }

    let mut entries = vec![];
    for entry in std::fs::read_dir(directory).unwrap() {
        let mut new_entry = Entry::new();
        new_entry.level = parent_level;
        let entry = entry.unwrap();
        let path = entry.path();
        new_entry.name = entry.file_name().into_string().unwrap();
        new_entry.path = String::from(path.as_path().to_str().unwrap());
        if path.is_file() {
            new_entry.is_file = true;
            
            let mut file = std::fs::File::open(path).unwrap();
            let mut file_content = vec![];
            if file.read_to_end(&mut file_content).is_ok() {
                new_entry.hash = Some(blake3::hash(&file_content));
            }
        } else if path.is_dir() {
            new_entry.is_file = false;
            let children = build_tree(path.as_path().clone(), new_entry.level + 1);
            if children.is_some() {
                new_entry.children = children.unwrap();
            }
        }
        entries.push(new_entry);
    }
    Some(entries)
}

#[derive(Debug)]
struct Entry {
    is_file: bool,
    path: String,
    name: String,
    children: Vec<Entry>,
    level: u8,
    hash: Option<Hash>,
}

impl Entry {
    fn new() -> Self {
        Self {
            is_file: Default::default(),
            path: Default::default(),
            name: Default::default(),
            children: vec![],
            level: Default::default(),
            hash: None,
        }
    }
}

impl std::fmt::Display for Entry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_file {
            if self.hash.is_some() {
                write!(f, "{:?} {:?}", self.name, self.hash.unwrap())
            } else {
                write!(f, "{:?}", self.name)
            }
        } else {
            let place_holder = String::from("    ").repeat(self.level as usize);
            println!("{}=> {:?} (level {:?})", place_holder, self.name, self.level);
            if self.children.len() > 0 {
                for child_entry in &self.children {
                    println!("{}{}", String::from("    ").repeat(child_entry.level as usize), child_entry);
                }
            }
            write!(f, "{}-----Folders:{:?} Files:{:?}-----", place_holder, self.children.iter().filter(|&item| !item.is_file).count(), self.children.iter().filter(|&item| item.is_file).count())
        }
    }
}
