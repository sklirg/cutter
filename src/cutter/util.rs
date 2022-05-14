use std::cmp;
use std::fs;
use std::path::Path;
use std::str;

pub fn generate_thumb_path(path: &str, w: i32, h: i32, path_suffix: &str) -> String {
    return format!("{}_{}x{}px_{}w.{}", path, w, h, w, path_suffix);
}

// @ToDo: Skip if not .jpg
pub fn get_file_name(path: &str) -> String {
    return Path::new(path)
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
}

pub fn get_files_in_dir(dirpath: &str) -> Vec<String> {
    let dir = Path::new(dirpath);
    let mut files = Vec::new();
    if dir.is_dir() {
        for entry in fs::read_dir(dir).unwrap() {
            let filename = entry.unwrap().path().to_str().unwrap().to_owned();
            // Skip filenames with _ in them as that's used to denote file sizes/formats.
            // !! The 400D shot images with names IMG_num so they won't work with this :D
            if !filename.contains('_') {
                files.push(filename);
            }
        }
    }

    files
}

pub fn print_list_iter_status(current: u32, len: u32, prefix: &str, verbose: bool) {
    let total = len;
    let threshold = cmp::max(1, cmp::min(25, len * 25 / 100));
    if verbose || (current == 0 || current == total || current % threshold == 0) {
        println!("{} {}/{}", prefix, current, total);
    }
}
