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
    /// Print trace to the output
    #[clap(short='o', long)]
    output_trace: bool,
    /// Find duplicate files
    #[clap(short='d', long)]
    find_duplicate: bool,
    /// Find with name expression
    #[clap(short='n', long)]
    find_name: Option<String>,

    /// Persist the detected files to a local file
    #[clap(short='s', long)]
    persist_file: Option<String>,

    /// Remove (delete) the result files
    #[clap(short='r', long)]
    remove_result: bool,
    
}

fn main() {
    let args = Args::parse();
    let path = args.path.to_owned();
    let show_trace = args.output_trace;
    
    let persist_file = args.persist_file.to_owned();
    let remove_result = args.remove_result;

    let find_duplicates = args.find_duplicate;
    let find_name = args.find_name.to_owned();

    println!("Start media-scanner");
    
    let root_path = Path::new(&path);
    if !root_path.is_dir() {
        println!("Invalid Path:{:?}", root_path);
        return;
    } else {
        println!("Use absolute Path:{}", root_path.canonicalize().unwrap().display())
    }

    let tree = build_tree(root_path, 0, show_trace).unwrap();
    
    let mut matches = vec![];

    if find_duplicates {
        matches = find_duplicate_files(tree).unwrap();
        println!("Duplicates:{:?}", matches.len());
    } else if find_name.is_some() {
        matches = find_matching_files(tree, find_name.unwrap()).unwrap();
        println!("Name Matches:{:?}", matches.len());
    }
    if show_trace {
        for entry in &matches {
            println!("{}", &entry);
        }
    }

    if persist_file.is_some() {
        let mut dup_file = File::create(persist_file.unwrap()).unwrap();
        for entry in &matches {
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

    if remove_result {
        for entry in &matches {
            println!("Delete Match for Hash:{}", entry.hash.unwrap());
            for (index, match_entry) in entry.matches.iter().enumerate() {
                if index == 0 {
                    continue;
                }
                let delete_result = std::fs::remove_file(match_entry.path.to_owned());
                if delete_result.is_err() {
                    println!("Could not delete File:{}", match_entry.path.to_owned());
                } else {
                    println!("Deleted file:{}", match_entry.path.to_owned());
                }
            }
        }
    } else {
        println!("{} matches collected. No future action!", matches.len());
    }
}

fn build_tree(directory: &Path, parent_level: u8, show_trace: bool) -> Option<Vec<Entry>> {
    if show_trace {
        println!("Directory:{}", directory.display())
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
            if show_trace {
                println!("File:{}", path.display())
            }
            new_entry.is_file = true;
            let mut file = std::fs::File::open(path).unwrap();
            let mut file_content = vec![];
            if file.read_to_end(&mut file_content).is_ok() {
                new_entry.hash = Some(blake3::hash(&file_content));
            }
        } else if path.is_dir() {
            new_entry.is_file = false;
            let children = build_tree(path.as_path().clone(), new_entry.level + 1, show_trace);
            if children.is_some() {
                new_entry.children = children.unwrap();
            }
        }
        entries.push(new_entry);
    }
    Some(entries)
}

fn find_matching_files(tree: Vec<Entry>, key: String) -> Option<Vec<Match>> {
    let mut matches: Vec<Match> = vec![];
    let mut flat_list = vec![];
    for entry in tree {
        create_flat_list(&mut flat_list, entry);
    }
    let value = key.to_uppercase();

    for entry in &flat_list {
        if entry.name.to_uppercase().contains(&value) {
            let dupl_entry: Option<&mut Match> = matches.iter_mut().find(|x| x.hash.unwrap() == entry.hash.unwrap());
            if dupl_entry.is_some() {
                let dupl_match = dupl_entry.unwrap();
                dupl_match.matches.push(MatchEntry {
                    name: entry.name.to_owned(),
                    path: entry.path.to_owned(),
                });
            } else {
                let mut duplicate = Match::new();
                duplicate.hash = Some(entry.hash.unwrap().clone());
                duplicate.matches.push(MatchEntry {
                    name: entry.name.to_owned(),
                    path: entry.path.to_owned(),
                });
                matches.push(duplicate);
            }
        }
    }

    Some(matches)
}

fn find_duplicate_files(tree: Vec<Entry>) -> Option<Vec<Match>> {
    let mut duplicates: Vec<Match> = vec![];
    let mut flat_list = vec![];
    for entry in tree {
        create_flat_list(&mut flat_list, entry);    
    }
    println!("Total files:{:?}", flat_list.len());

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
    Some(duplicates)
}

fn get_items_by_hash(hash: Hash, items: &Vec<Entry>) -> Option<Match> {
    let mut duplicate = Match::new();
    duplicate.hash = Some(hash.clone());
    for entry in items {
        if entry.hash.is_some() {
            if entry.hash.unwrap() == hash {
                duplicate.matches.push(MatchEntry {
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

#[derive(Debug, Clone)]
struct Match {
    hash: Option<Hash>,
    matches: Vec<MatchEntry>,
}

impl Match {
    fn new() -> Self {
        Self {
            hash: None,
            matches: vec![],
        }
    }
}

impl std::fmt::Display for Match {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        println!("Match {:?}", self.hash.unwrap());
        for entry in &self.matches {
            println!("{}", &entry);
        }
        write!(f, "Entires {:?}", self.matches.len())
    }
}

#[derive(Debug, Clone)]
struct MatchEntry {
    name: String,
    path: String
}

impl std::fmt::Display for MatchEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} {:?}", self.name, self.path)
    }
}
