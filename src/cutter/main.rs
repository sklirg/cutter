use std::cmp;
use std::env;
use std::error::Error;
use std::fs;
use std::fs::{File};
use std::io::{Read, Write};
use std::path::Path;
use std::str;

use lambda_runtime::{error::HandlerError, lambda, Context};
use s3::bucket::Bucket;
use s3::credentials::Credentials;
use serde::{Deserialize, Serialize};

extern crate raster;

const DEFAULT_REGION: &str = "eu-central-1";

#[derive(Debug)]
struct Config {
    clean: bool,
    files_path: String,
    overwrite: bool,
    s3_bucket_name: String,
    s3_region: String,
    s3_prefix: String,
}

#[derive(Debug,Deserialize)]
pub struct LambdaEvent {
    bucket: String,
    prefix: String,

}

#[derive(Serialize)]
pub struct LambdaOutput {
    message: String,
}

fn run(config: &Config) {
    println!("Executing with config: {:?}", config);

    if Path::new(&config.s3_prefix).exists() && (config.clean || config.overwrite) {
        println!("Removing existing directory...");
        fs::remove_dir_all(&config.s3_prefix).unwrap();
    }

    fs::create_dir(&config.s3_prefix).unwrap();

    if config.s3_bucket_name != "" {
        download_from_s3(&config);
    }

    let files = get_files_in_dir(&config.files_path);

    let processed_files = transform_images(files, &config.files_path);

    upload_to_s3(&config, processed_files);

    println!("Done!");
}

pub fn main() {
    let config = process_args();
    run(&config);
}

pub fn lambda_handler(event: LambdaEvent, context: Context) -> Result<LambdaOutput, HandlerError> {
    if event.bucket == "" {
        eprintln!("Missing bucket name");
        panic!("Missing bucket name");
    }

    let mut path = event.bucket.to_owned();

    if event.prefix != "" {
        path = event.prefix.to_owned();
    }

    let config = Config {
        clean: true,
        files_path: event.prefix.to_owned(),
        overwrite: true,
        s3_bucket_name: event.bucket.to_owned(),
        s3_prefix: event.prefix.to_owned(),
        s3_region: DEFAULT_REGION.to_owned(),
    };

    run(&config);

    Ok(LambdaOutput {
        message: format!("Success!"),
    })
}

// App config
fn process_args() -> Config {
    let args: Vec<String> = env::args().collect();

    match args.len() {
        1...2 => panic!("Missing args"),
        3...4 => return process_two_args(args),
        _ => panic!("Yikes"),
    }
}

fn process_two_args(args: Vec<String>) -> Config {
    let first_arg = &args[1];

    let mut config: Config = Config {
        clean: true,
        files_path: "".to_owned(),
        overwrite: false,
        s3_bucket_name: "".to_owned(),
        s3_prefix: "".to_owned(),
        s3_region: DEFAULT_REGION.to_owned(),
    };

    match first_arg.as_str() {
        "path" => {
            config.files_path = args[2].to_owned();
        }
        "s3" => {
            config.files_path = args[2].to_owned();
            config.s3_bucket_name = args[2].to_owned();
            if args.len() == 4 {
                config.files_path = args[3].to_owned();
                config.s3_prefix = args[3].to_owned();
            }
        }
        _ => panic!("Unknown operation"),
    }

    return config;
}
// End config

fn download_from_s3(config: &Config) {
    println!("Downloading files from S3 bucket '{}' ({})...", &config.s3_bucket_name, &config.s3_prefix);
    let credentials = Credentials::default();
    let bucket = Bucket::new(&config.s3_bucket_name, config.s3_region.parse().unwrap(), credentials).unwrap();
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
            && !all_files.contains(thumb_key) {
                files.push(file);
        }
        else {
            skipped += 1;
        }
    }

    println!("Downloading {} files to {} (skipped {})", files.len(), &config.s3_prefix, skipped);
    let numfiles = files.len();
    let mut counter = 0;

    for file in &files {
        print_list_iter_status(counter, numfiles as u32, "Downloaded");
        let (data, _) = &bucket.get(&file).unwrap();
        let mut buffer = File::create(&file.to_owned()).unwrap();
        buffer.write(data).unwrap();
        counter += 1;
    }
}

fn upload_to_s3(config: &Config, files: Vec<String>) {
    let credentials = Credentials::default();
    let bucket = Bucket::new(&config.s3_bucket_name, config.s3_region.parse().unwrap(), credentials).unwrap();

    println!("Uploading {} files to S3 bucket '{}'", files.len(), &config.s3_bucket_name);
    let mut counter = 0;
    let numfiles = files.len();
    for file in &files {
        print_list_iter_status(counter, numfiles as u32, "Uploaded");
        let mut buf = Vec::new();
        File::open(&file).unwrap().read_to_end(&mut buf).unwrap();
        bucket.put(file, &buf, "image/jpeg").unwrap();
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
