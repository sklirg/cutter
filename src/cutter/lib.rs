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
    pub crop_sizes: Vec<[i32; 2]>,
    pub tmp_dir: String,
    pub verbose: bool,
}

pub fn run(config: &Config) {
    println!("Executing with config: {:?}", config);

    if config.verbose {
        explain_config(config);
    }

    if !Path::new(&config.tmp_dir).exists() {
        fs::create_dir(&config.tmp_dir).unwrap();
    }
    if config.clean || config.overwrite {
        fs::remove_dir_all(&config.tmp_dir).unwrap();
    }

    if config.fetch_remote && config.s3_bucket_name != "" {
        download_from_s3(&config);
    }

    println!("Finding files in {}", &config.files_path);
    let files = get_files_in_dir(&config.files_path);

    let processed_files = transform_images(
        files,
        &config.files_path,
        &config.crop_sizes,
        config.verbose,
    );

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

fn explain_config(config: &Config) {
    println!("Explaining configuration: {:?}", config);

    println!("*************** CONFIGURATION ***************");

    if config.s3_bucket_name != "" {
        println!(
            "Will publish files to S3 bucket '{}' after completion",
            config.s3_bucket_name
        );

        println!("Will overwrite files on remote: {}", config.overwrite);
    }

    if config.fetch_remote {
        println!(
            "Fetching files from remote: {}/{}",
            config.s3_bucket_name, config.s3_prefix
        );
    } else {
        println!(
            "Path to source files locally on this host: {}",
            config.files_path
        );
    }

    println!("Working/temporary directory: {}", config.tmp_dir);

    if config.clean {
        println!("Will clean working directory before starting");
    }

    println!(
        "Will crop to the following {} size(s):",
        config.crop_sizes.len()
    );
    for size in &config.crop_sizes {
        println!("\t{:?}", size);
    }

    println!("*************** END CONFIGURATION ***************");
}

// App config
fn process_args() -> Config {
    let default_crop_sizes = ["200x200", "400x400", "800x800", "1920x1080"];

    let matches = App::new("Cutter")
        .version("0.3.0")
        .author("Sklirg")
        .about("Image cropping tool")
        .arg(
            Arg::with_name("path")
                .short("p")
                .long("path")
                .takes_value(true)
                .conflicts_with("fetch-remote")
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
                .takes_value(false)
                .requires("s3-bucket")
                .conflicts_with("path")
                .help("Fetch images from S3 bucket specified in --s3-bucket"),
        )
        .arg(
            Arg::with_name("s3-prefix")
                .long("s3-prefix")
                .takes_value(true)
                .requires("s3-bucket")
                .help("Used to filter the start of the s3 object key"),
        )
        .arg(
            Arg::with_name("overwrite")
                .short("o")
                .long("overwrite")
                .takes_value(false)
                .help("Whether to overwrite files already present on the remote or not"),
        )
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .takes_value(false)
                .help("Print verbose output to stdout"),
        )
        .arg(
            Arg::with_name("size")
                .short("s")
                .long("size")
                .multiple(true)
                .takes_value(true)
                .help("Crop sizes specified as WxH (e.g. 200x200) (overrides defaults). Use the argument one time per crop size."),
        )
        .get_matches();

    let tmp_dir = "/tmp/cutter";

    let local_path = process_arg_with_default(matches.value_of("path"), tmp_dir);
    let s3_bucket = process_arg_with_default(matches.value_of("s3-bucket"), "");
    let s3_prefix = process_arg_with_default(matches.value_of("s3-prefix"), "");
    let fetch_remote = matches.is_present("fetch-remote");
    let overwrite = matches.is_present("overwrite");
    let verbose = matches.is_present("verbose");
    let mut crop_sizes = Vec::new();

    let mut _crop_sizes_options = Vec::new();

    if matches.is_present("size") {
        for size in matches.values_of("size").unwrap() {
            _crop_sizes_options.push(size);
        }
    } else {
        _crop_sizes_options = default_crop_sizes.to_vec();
    }

    for size in _crop_sizes_options {
        if !size.contains("x") || size.split("x").collect::<Vec<&str>>().len() != 2 {
            panic!("Invalid sizes configuration. Use the expected format: WIDTHxHEIGHT, e.g.: 1920x1080");
        }

        let height_str = size.split("x").collect::<Vec<&str>>()[1];
        let width_str = size.split("x").collect::<Vec<&str>>()[0];

        let height: i32 = height_str.parse().unwrap();
        let width: i32 = width_str.parse().unwrap();
        crop_sizes.push([width, height]);
    }

    if local_path == "" && (fetch_remote && s3_bucket == "") {
        panic!("Missing required arguments to run.");
    }

    let mut files_path = local_path;

    if fetch_remote {
        let mut prefix_path = s3_prefix.to_owned();
        if prefix_path.contains("/") {
            let splits: Vec<&str> = prefix_path.split("/").collect::<Vec<&str>>();
            prefix_path = splits[0].to_owned();
        }
        files_path = format!("{}/{}", files_path, prefix_path);
    }

    let config: Config = Config {
        clean: true,
        crop_sizes: crop_sizes.to_vec(),
        fetch_remote: fetch_remote,
        files_path: files_path.to_owned(),
        overwrite: overwrite,
        s3_bucket_name: s3_bucket,
        s3_prefix: s3_prefix.to_owned(),
        s3_region: DEFAULT_REGION.to_owned(),
        tmp_dir: "/tmp/cutter".to_owned(),
        verbose: verbose,
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
        if file.contains("_200")
            || file.contains("_400")
            || file.contains("_800")
            || file.contains("_1920")
            || file.contains("_thumb")
        {
            skipped += 1;
            continue;
        }

        let thumb_key = &file.replace(".jpg", "_thumb.jpg");

        let valid_file_name = file != "" && file != &format!("{}/", &config.s3_prefix);
        let has_sizes = file.contains("_");

        if valid_file_name && (config.overwrite || (!config.overwrite && !has_sizes)) {
            files.push(file);
        } else {
            skipped += 1;
        }
    }

    let root_dir = config.files_path.to_owned();

    println!(
        "Downloading {} files to {} (skipped {})",
        files.len(),
        &root_dir,
        skipped
    );
    let numfiles = files.len();
    let mut counter = 0;

    if Path::new(&root_dir).exists() && (config.clean || config.overwrite) {
        println!("Removing existing directory...");
        fs::remove_dir_all(&root_dir).unwrap();
    }
    fs::create_dir_all(&root_dir).unwrap();

    for file in &files {
        let gallery_image: Vec<&str> = file.split("/").collect();
        let mut path = format!("{}/{}", &config.files_path, &file);
        if gallery_image.len() > 1 {
            path = format!("{}/{}", &config.files_path, &gallery_image[1]);
        }
        print_list_iter_status(counter, numfiles as u32, "Downloaded", config.verbose);
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
        print_list_iter_status(counter, numfiles as u32, "Uploaded", config.verbose);
        let mut buf = Vec::new();
        File::open(&file).unwrap().read_to_end(&mut buf).unwrap();
        // @ToDo: Fix output if files are served locally.
        // They're currently prefixed with the folder name sent in through config
        // But need the prefix from S3.
        bucket
            .put(&file.replace(&config.tmp_dir, ""), &buf, "image/jpeg")
            .unwrap();
        counter += 1;
    }
}

fn transform_images(
    files: Vec<String>,
    output_path: &str,
    sizes: &Vec<[i32; 2]>,
    verbose: bool,
) -> Vec<String> {
    let numfiles = files.len().to_owned();
    println!("Processing {} files", numfiles);

    let mut created_files = Vec::new();

    let mut counter = 0;
    for f in files {
        print_list_iter_status(counter, numfiles as u32, "Processed", verbose);
        for size in sizes {
            let width = size[0];
            let height = size[1];
            let image = transform_image(&f, width, height);

            let thumb_path = format!(
                "{}/{}",
                output_path,
                generate_thumb_path(&get_file_name(&f), width, height, "jpg")
            );

            save_image(&image, &thumb_path);
            created_files.push(thumb_path);
        }
        counter += 1;
    }

    return created_files;
}

fn transform_image(path: &str, width: i32, height: i32) -> raster::Image {
    let mut image = raster::open(path).unwrap();
    raster::transform::resize_fill(&mut image, width, height).unwrap();
    return image;
}

fn save_image(image: &raster::Image, path: &str) {
    raster::save(&image, &path).unwrap();
}

fn generate_thumb_path(path: &str, w: i32, h: i32, path_suffix: &str) -> String {
    return format!("{}_{}x{}px_{}w.{}", path, w, h, w, path_suffix);
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
            let filename = entry.unwrap().path().to_str().unwrap().to_owned();
            // Skip filenames with _ in them as that's used to denote file sizes/formats.
            // !! The 400D shot images with names IMG_num so they won't work with this :D
            if !filename.contains("_") {
                files.push(filename);
            }
        }
    }

    return files;
}

fn print_list_iter_status(current: u32, len: u32, prefix: &str, verbose: bool) {
    let total = len;
    let threshold = cmp::max(1, cmp::min(25, len * 25 / 100));
    if verbose {
        println!("{} {}/{}", prefix, current, total);
    } else if current == 0 || current == total || current % threshold == 0 {
        println!("{} {}/{}", prefix, current, total);
    }
}
