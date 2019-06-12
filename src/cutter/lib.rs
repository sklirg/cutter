use std::cmp;
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::str;

use clap::{App, Arg};
use s3::bucket::Bucket;
use s3::credentials::Credentials;

extern crate clap;
extern crate raster;

pub const DEFAULT_REGION: &str = "eu-central-1";

#[derive(Debug)]
pub struct Config {
    pub clean: bool,
    pub fetch_remote: bool,
    pub files_path: String,
    pub overwrite: bool,
    pub s3_bucket_name: String,
    pub s3_region: String,
    pub s3_prefix: String,
    pub tmp_dir: String,
}

pub fn run(config: &Config) {
    println!("Executing with config: {:?}", config);

    if Path::new(&config.tmp_dir).exists() && (config.clean || config.overwrite) {
        println!("Removing existing directory...");
        fs::remove_dir_all(&config.tmp_dir).unwrap();
    }

    println!("Tmp path {}", &config.tmp_dir);
    fs::create_dir(&config.tmp_dir).unwrap();

    if config.fetch_remote && config.s3_bucket_name != "" {
        download_from_s3(&config);
    }

    println!("Finding files in {}", &config.files_path);
    let files = get_files_in_dir(&config.files_path);

    let processed_files = transform_images(files, &config.files_path);

    if config.s3_bucket_name != "" {
        upload_to_s3(&config, processed_files);
    }

    println!("Done!");
}

// Public entrypoint for lib
#[allow(dead_code)]
pub fn main() {
    let config = process_args();
    run(&config);
}

// App config
fn process_args() -> Config {
    let matches = App::new("Cutter")
        .version("0.3.0")
        .author("Sklirg")
        .about("Image cropping tool")
        .arg(
            Arg::with_name("path")
                .short("p")
                .long("path")
                .takes_value(true)
                .help("Local file path of gallery to generate crops for"),
        )
        .arg(
            Arg::with_name("s3-bucket")
                .short("b")
                .long("s3-bucket")
                .takes_value(true)
                .help("S3 bucket to sync files to (and fetch from, if --fetch-remote is specified"),
        )
        .arg(
            Arg::with_name("fetch-remote")
                .short("r")
                .long("fetch-remote")
                .takes_value(true)
                .help("Fetch images from S3 bucket specified in --s3-bucket")
                .default_value("false"),
        )
        .arg(
            Arg::with_name("s3-prefix")
                .long("s3-prefix")
                .takes_value(true)
                .help("Used to filter the start of the s3 object key"),
        )
        .get_matches();

    let local_path = process_arg_with_default(matches.value_of("path"), "");
    let s3_bucket = process_arg_with_default(matches.value_of("s3-bucket"), "");
    let s3_prefix = process_arg_with_default(matches.value_of("s3-prefix"), "");
    let fetch_remote = process_arg_with_default(matches.value_of("fetch-remote"), "") == "true";

    if local_path == "" && (fetch_remote && s3_bucket == "") {
        panic!("Missing required arguments to run.");
    }

    let config: Config = Config {
        clean: true,
        fetch_remote: fetch_remote,
        files_path: local_path.to_owned(),
        overwrite: false,
        s3_bucket_name: s3_bucket,
        s3_prefix: s3_prefix.to_owned(),
        s3_region: DEFAULT_REGION.to_owned(),
        tmp_dir: "/tmp/cutter".to_owned(),
    };
    return config;
}

fn process_arg_with_default(arg: Option<&str>, default: &str) -> String {
    match arg {
        None => return default.to_owned(),
        Some(s) => return s.to_owned(),
    };
}
// End config

fn download_from_s3(config: &Config) {
    println!(
        "Downloading files from S3 bucket '{}' ({})...",
        &config.s3_bucket_name, &config.s3_prefix
    );
    let credentials = Credentials::default();
    let bucket = Bucket::new(
        &config.s3_bucket_name,
        config.s3_region.parse().unwrap(),
        credentials,
    )
    .unwrap();
    let bucket_contents = bucket.list(&config.s3_prefix, None).unwrap();

    let mut all_files = Vec::new();

    for (list, _) in bucket_contents {
        for obj in list.contents {
            all_files.push(obj.key);
        }
    }

    let mut files = Vec::new();

    let mut skipped = 0;

    for file in &all_files {
        let thumb_key = &file.replace(".jpg", "_thumb.jpg");
        if !&file.contains("_thumb")
            && file.contains(".jpg")
            // Skip files with existing thumbs
            && !all_files.contains(thumb_key)
        {
            files.push(file);
        } else {
            skipped += 1;
        }
    }

    println!(
        "Downloading {} files to {} (skipped {})",
        files.len(),
        &config.s3_prefix,
        skipped
    );
    let numfiles = files.len();
    let mut counter = 0;

    let root_dir = format!("{}/{}", &config.tmp_dir, &config.s3_prefix);
    if Path::new(&root_dir).exists() && (config.clean || config.overwrite) {
        println!("Removing existing directory...");
        fs::remove_dir_all(&root_dir).unwrap();
    }
    fs::create_dir_all(&root_dir).unwrap();

    for file in &files {
        let path = format!("{}/{}", &config.tmp_dir, &file);
        print_list_iter_status(counter, numfiles as u32, "Downloaded");
        let (data, _) = &bucket.get(&file).unwrap();
        let mut buffer = File::create(&path.to_owned()).unwrap();
        buffer.write(data).unwrap();
        counter += 1;
    }
}

fn upload_to_s3(config: &Config, files: Vec<String>) {
    let credentials = Credentials::default();
    let bucket = Bucket::new(
        &config.s3_bucket_name,
        config.s3_region.parse().unwrap(),
        credentials,
    )
    .unwrap();

    println!(
        "Uploading {} files to S3 bucket '{}'",
        files.len(),
        &config.s3_bucket_name
    );
    let mut counter = 0;
    let numfiles = files.len();
    for file in &files {
        print_list_iter_status(counter, numfiles as u32, "Uploaded");
        let mut buf = Vec::new();
        File::open(&file).unwrap().read_to_end(&mut buf).unwrap();
        bucket
            .put(&file.replace(&config.tmp_dir, ""), &buf, "image/jpeg")
            .unwrap();
        counter += 1;
    }
}

fn transform_images(files: Vec<String>, output_path: &str) -> Vec<String> {
    let numfiles = files.len().to_owned();
    println!("Processing {} files", numfiles);

    let mut created_files = Vec::new();

    let mut counter = 0;
    for f in files {
        print_list_iter_status(counter, numfiles as u32, "Processed");
        let thumb_path = format!(
            "{}/{}",
            output_path,
            generate_thumb_path(&get_file_name(&f), "jpg")
        );
        let mut image = raster::open(&f).unwrap();
        transform_image(&mut image);
        save_image(&image, &thumb_path);
        created_files.push(thumb_path);
        counter += 1;
    }

    return created_files;
}

fn transform_image(image: &mut raster::Image) {
    raster::transform::resize_fill(image, 200, 200).unwrap();
}

fn save_image(image: &raster::Image, path: &str) {
    raster::save(&image, &path).unwrap();
}

fn generate_thumb_path(path: &str, path_suffix: &str) -> String {
    return format!("{}_thumb.{}", path, path_suffix);
}

// @ToDo: Skip if not .jpg
fn get_file_name(path: &str) -> String {
    return Path::new(path)
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
}

fn get_files_in_dir(dirpath: &str) -> Vec<String> {
    let dir = Path::new(dirpath);
    let mut files = Vec::new();
    if dir.is_dir() {
        for entry in fs::read_dir(dir).unwrap() {
            files.push(entry.unwrap().path().to_str().unwrap().to_owned());
        }
    }

    return files;
}

fn print_list_iter_status(current: u32, len: u32, prefix: &str) {
    let total = len - 1;
    let threshold = cmp::max(1, cmp::min(25, len * 25 / 100));
    if current == 0 || current == total || current % threshold == 0 {
        println!("{} {}/{}", prefix, current, total);
    }
}
