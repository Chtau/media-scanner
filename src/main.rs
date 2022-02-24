use std::{path::Path, io::{Read, Write}, fs::File};

use blake3::Hash;
use clap::Parser;

/// Simple duplicates check
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Path to check for duplicates ("." to use thestartup path)
    #[clap(short, long)]
    path: String,
    /// Print file tree to the output
    #[clap(short='t', long)]
    output_tree: bool,
    /// Print the duplicates to the output
    #[clap(short='d', long)]
    output_duplicates: bool,
    /// Saves the duplicates the given file
    #[clap(short='s', long)]
    save_duplicates: Option<String>,
}

fn main() {
    let args = Args::parse();
    let path = args.path.to_owned();
    let show_tree = args.output_tree;
    let show_duplicates = args.output_duplicates;
    let save_path = args.save_duplicates.to_owned();

    println!("Start media-scanner");
    
    let root_path = Path::new(&path);
    let tree = build_tree(root_path, 0).unwrap();
    if show_tree {
        for entry in &tree {
            println!("{}", &entry);
        }
    }
    
    let duplicates = find_duplicates(tree).unwrap();
    println!("Duplicates:{:?}", duplicates.len());
    if show_duplicates {
        for entry in &duplicates {
            println!("{}", &entry);
        }
    }
    if save_path.is_some() {
        let mut dup_file = File::create(save_path.unwrap()).unwrap();
        for entry in &duplicates {
            let line = "Hash: ".to_owned() + &entry.hash.unwrap().to_string() + "\n";
            let result = dup_file.write(line.as_bytes());
            if result.is_ok() {
                for match_entry in &entry.matches {
                    let line = match_entry.path.to_owned() + "\n";
                    let result = dup_file.write(line.as_bytes());
                    if !result.is_ok() {
                        println!(".Could not write Entry: {} to file", &match_entry);
                    }
                }
            }
        }
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

fn find_duplicates(tree: Vec<Entry>) -> Option<Vec<Duplicates>> {
    let mut duplicates: Vec<Duplicates> = vec![];
    let mut flat_list = vec![];
    for entry in tree {
        create_flat_list(&mut flat_list, entry);    
    }
    println!("Flat items:{:?}", flat_list.len());

    for entry in &flat_list {
        if entry.hash.is_some() {
            let hash = entry.hash.unwrap();
            if duplicates.iter().filter(|x|x.hash.unwrap() == hash).count() == 0 {
                // unchecked hash values
                let duplicate = get_items_by_hash(hash, &flat_list);
                if duplicate.is_some() {
                    duplicates.push(duplicate.unwrap());
                }
            }
        }
    }

    println!("Duplicate items:{:?}", duplicates.len());
    Some(duplicates)
}

fn get_items_by_hash(hash: Hash, items: &Vec<Entry>) -> Option<Duplicates> {
    let mut duplicate = Duplicates::new();
    duplicate.hash = Some(hash.clone());
    for entry in items {
        if entry.hash.is_some() {
            if entry.hash.unwrap() == hash {
                duplicate.matches.push(DuplicateEntry {
                    name: entry.name.to_owned(),
                    path: entry.path.to_owned(),
                });
            }
        }
    }
    // atleast 2 matches because we add our self
    if duplicate.matches.len() > 1 {
        return Some(duplicate);
    }
    None
}

fn create_flat_list(flat_list: &mut Vec<Entry>, parent: Entry) {
    flat_list.push(parent.clone());
    if !parent.children.is_empty() {
        for child in parent.children {
            create_flat_list(flat_list, child);
        }
    }
}

#[derive(Debug, Clone)]
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

#[derive(Debug)]
struct Duplicates {
    hash: Option<Hash>,
    matches: Vec<DuplicateEntry>,
}

impl Duplicates {
    fn new() -> Self {
        Self {
            hash: None,
            matches: vec![],
        }
    }
}

impl std::fmt::Display for Duplicates {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        println!("Match {:?}", self.hash.unwrap());
        for entry in &self.matches {
            println!("{}", &entry);
        }
        write!(f, "Entires {:?}", self.matches.len())
    }
}

#[derive(Debug)]
struct DuplicateEntry {
    name: String,
    path: String
}

impl std::fmt::Display for DuplicateEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} {:?}", self.name, self.path)
    }
}
