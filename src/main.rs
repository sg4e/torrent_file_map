/*
 * The MIT License
 *
 * Copyright 2020 sg4e.
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in
 * all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
 * THE SOFTWARE.
 */

use glob::glob;
use lava_torrent::torrent::v1::Torrent;
use std::fs::read_dir;
use std::fs::DirEntry;
use std::path::PathBuf;
use std::env;
use lava_torrent::torrent::v1::File as LavaFile;
use std::fs::File as OsFile;
use read_byte_slice::{ByteSliceIter, FallibleStreamingIterator};
use sha1::{Sha1, Digest};

const SIGNIFICANT_SIZE: i64 = 16 * 1024; //16 kb
const SIGNIFICANT_FILE_COUNT: u32 = 1;
const SIGNIFICANT_PIECE_MATCH: u64 = 1;

fn main() {
    let args: Vec<String> = env::args().collect();
    let torrent_file_dir = args.get(1).expect("Torrent files directory not specified");
    let torrent_data_root = args.get(2).expect("Torrent data directory not specified");
    println!("Building directory structure...");
    let files = get_directory(torrent_data_root);
    println!("Building complete");
    let mut total = 0;
    let mut matched = 0;
    for entry in glob(torrent_file_dir).expect("Failed to read glob pattern") {
        let path = entry.unwrap();
        match Torrent::read_from_file(&path) {
            Ok(torrent) => {
                total +=1;
                let mut possible_matches = Vec::new();
                for f in &files {
                    //length-based search
                    if torrent.length == f.total_size.unwrap() as i64 {
                        possible_matches.push(f);
                    }
                    //per-file length-based search
                    if torrent.files.is_some() {
                        let torrent_sizes: Vec<u64> = torrent.files.as_ref().unwrap().iter()
                            .map(|tf| tf.length)
                            .filter(|s| s > &SIGNIFICANT_SIZE)
                            .map(|i| i as u64)
                            .collect();
                        let mut file_sizes = f.get_children_sizes();
                        let mut dup = 0;
                        for s in torrent_sizes {
                            if let Some(pos) = file_sizes.iter().position(|x| *x == s) {
                                dup += 1;
                                file_sizes.swap_remove(pos);
                            }
                        }
                        if dup >= SIGNIFICANT_FILE_COUNT {
                            if !possible_matches.contains(&f) {
                                possible_matches.push(f);
                            }
                        }
                    }
                    //name-based search
                    if f.has_filename(torrent.name.as_str()) {
                        if !possible_matches.contains(&f) {
                            possible_matches.push(f);
                        }
                    }
                }
                //check hashes
                let verified_matches: Vec<&File> = possible_matches.into_iter().filter(|f| f.hashes_match(&torrent, SIGNIFICANT_PIECE_MATCH)).collect();
                if verified_matches.is_empty() {
                    //println!("All matches for {:?} failed hash verification", &torrent.name);
                }
                else {//if verified_matches.len() > 1 {
                    //println!("Multiple matches for {:?}: {:?}", &torrent.name, &verified_matches);
                    //matched += 1;
                //}
                //else { //exactly 1 match
                    println!("{:?}: {:?}", &path.file_name().unwrap(), verified_matches[0].path);
                    matched += 1;
                }
            }
            Err(e) => println!("Error {:?} with path {:?}", e, &path)
        }
    }
    println!("Matched%: {:?}", (matched as f64)/(total as f64) * 100f64);
}

fn get_directory(root : &str) -> Vec<File> {
    let mut files = Vec::new();
    for root_file in read_dir(root).expect("Cannot open directory") {
        files.push(get_file(root_file.unwrap()));
    }
    //lazy load all sizes in this tree
    #[allow(unused_variables)]
    let size: u64 = files.iter_mut().map(|f| f.calc_total_size()).sum();
    files
}

fn get_file(file : DirEntry) -> File {
    let filename = file.file_name().into_string().expect("Cannot convert filename to string");
    let filepath = file.path();
    let metadata = file.metadata().expect("Cannot get file metadata");
    if metadata.is_dir() {
        let inside = get_directory(file.path().to_str().unwrap());
        let this_dir = File {
            path: filepath,
            name: filename,
            size: None,
            is_dir: true,
            children: inside,
            total_size: None
        };
        this_dir
    }
    else {
        let size = Some(metadata.len());
        let meta_obj = File {
            path: filepath,
            name: filename,
            size,
            is_dir: false,
            children: Vec::with_capacity(0),
            total_size: size
        };
        meta_obj
    }
}

#[derive(Debug)]
struct File {
    path: PathBuf,
    name: String,
    size: Option<u64>,
    is_dir: bool,
    children: Vec<File>,
    total_size: Option<u64>
}

impl PartialEq for File {
    fn eq(&self, other: &File) -> bool {
        self.path == other.path
    }
}

impl Eq for File {}

impl File {
    fn calc_total_size(&mut self) -> u64 {
        match self.total_size {
            Some(size) => size,
            None => {
                let children = &mut self.children;
                self.total_size = match self.is_dir {
                    true => Some(children.iter_mut().map(|c| c.calc_total_size()).sum()),
                    false => Some(self.size.unwrap())
                };
                self.total_size.unwrap()
            }
        }
    }

    fn get_children_sizes(&self) -> Vec<u64> {
        if self.is_dir {
            self.children.iter().flat_map(|c| c.get_children_sizes()).collect()
        }
        else {
            let mut v = Vec::with_capacity(1);
            v.push(self.size.unwrap());
            v
        }
    }

    fn has_filename(&self, name: &str) -> bool {
        self.name.as_str() == name || self.children.iter().any(|c| c.has_filename(name))
    }

    fn hashes_match(&self, torrent: &Torrent, match_threshold: u64) -> bool {
        match torrent.files.as_ref() {
            Some(files) => {
                return true; //TODO implement
            }
            None => {
                //single file
                if self.is_dir {
                    //println!("Single-file torrent {:?} matched to directory {:?}", torrent.name, self.path);
                    //this can happen when a higher-level directory is incorrectly identified as owning the inner file
                    return false;
                }
                let piece_size = torrent.piece_length;
                let local_file = match OsFile::open(&self.path) {
                    Ok(f) => f,
                    Err(e) => {
                        println!("Error opening file {:?}: {:?}", &self.path, e);
                        return false;
                    }
                };
                //let mut local_piece_hashes = Vec::with_capacity(torrent.pieces.len());
                let mut iter = ByteSliceIter::new(local_file, piece_size as usize);
                let mut hasher = Sha1::new();
                let mut piece_index = 0;
                let mut matching_pieces = 0;
                let total_pieces = torrent.pieces.len() as usize;
                while let Some(chunk) = iter.next().expect("Error reading file as chunks") {
                    let h = &mut hasher;
                    h.update(chunk);
                    let result = h.finalize_reset();
                    if piece_index >= total_pieces {
                        return false;
                    }
                    let torrent_piece = &torrent.pieces[piece_index];
                    if result.as_slice() == torrent_piece.as_slice() {
                        matching_pieces += 1;
                        if matching_pieces >= match_threshold {
                            return true;
                        }
                    }

                    piece_index += 1;
                }

                return false;
            }
        }
    }
}

struct Correspondence<'a> {
    torrent_file: &'a LavaFile,
    local_file: &'a File
}
