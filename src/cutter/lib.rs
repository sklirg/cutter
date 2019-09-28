use std::fs;
use std::path::Path;
use std::str;

use clap::{App, Arg};

use super::config::Config;
use super::imageprocessing::transform_images;
use super::s3::{download_from_s3, upload_to_s3};
use super::util::get_files_in_dir;

extern crate clap;
extern crate raster;

pub const DEFAULT_REGION: &str = "eu-central-1";

pub fn run(config: &Config) {
    println!("Executing with config: {:?}", config);

    if config.verbose {
        explain_config(config);
    }

    if Path::new(&config.tmp_dir).exists() && (config.clean || config.overwrite) {
        fs::remove_dir_all(&config.tmp_dir).unwrap();
    }

    if !Path::new(&config.tmp_dir).exists() {
        fs::create_dir(&config.tmp_dir).unwrap();
    }

    if config.fetch_remote && config.s3_bucket_name != "" {
        download_from_s3(&config);
    }

    println!("Finding files in {}", &config.files_path);
    let files = get_files_in_dir(&config.files_path);

    let processed_files =
        transform_images(files, &config.tmp_dir, &config.crop_sizes, config.verbose);

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
