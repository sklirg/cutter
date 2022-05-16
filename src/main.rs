use std::fs;
use std::path::Path;
use std::str;

use clap::Parser;

use cutter::imageprocessing::{str_to_size, transform_images, Size};
use cutter::s3::{download_from_s3, upload_to_s3};
use cutter::util::get_files_in_dir;

mod cutter;

extern crate clap;

pub const DEFAULT_REGION: &str = "eu-central-1";
const DEFAULT_CROP_SIZES: [&str; 4] = ["200x200", "400x400", "800x800", "1920x1080"];

#[derive(Debug, Parser)]
pub struct Config {
    /// Path to files to run Cutter on.
    /// Cannot be used if files are fetched from a remote.
    #[clap(short = 'p', long = "path", conflicts_with = "fetch-remote")]
    pub files_path: String,

    /// Sizes to crop into. Can be used multiple times.
    /// format: WIDTHxHEIGHT
    #[clap(short='s', parse(try_from_str=str_to_size), default_values=&DEFAULT_CROP_SIZES)]
    pub crop_sizes: Vec<Size>,

    /// Clean output directory before starting.
    #[clap(short)]
    pub clean: bool,
    /// Overwrite existing files.
    #[clap(short, long)]
    pub overwrite: bool,
    /// Tmp dir to store output files in.
    #[clap(short, long, default_value = "/tmp/cutter")]
    pub tmp_dir: String,
    /// Enable verbose output.
    #[clap(short, long)]
    pub verbose: bool,

    /// Name of S3 bucket to upload files to.
    #[clap(short = 'b')]
    pub s3_bucket_name: Option<String>,
    /// Region of S3 bucket.
    #[clap(long)]
    pub s3_region: Option<String>,
    /// Prefix for files uploaded to S3.
    #[clap(long)]
    pub s3_prefix: Option<String>,
    /// Fetch files from S3 bucket for Cutting.
    #[clap(short = 'r', long)]
    pub fetch_remote: Option<bool>,
}

#[tokio::main]
pub async fn main() {
    let config = Config::parse();
    run(config).await;
}

pub async fn run(config: Config) {
    println!("Executing with config: {:?}", config);

    if config.verbose {
        explain_config(&config);
    }

    if Path::new(&config.tmp_dir).exists() && (config.clean || config.overwrite) {
        fs::remove_dir_all(&config.tmp_dir).unwrap();
    }

    if !Path::new(&config.tmp_dir).exists() {
        fs::create_dir(&config.tmp_dir).unwrap();
    }

    if let Some(fetch_remote) = config.fetch_remote {
        if config.s3_bucket_name.is_none() {
            panic!("shouldnt happen because config cheks for this :)");
        }
        if fetch_remote {
            if let Some(s3_bucket_name) = &config.s3_bucket_name {
                download_from_s3(
                    s3_bucket_name,
                    &config
                        .s3_region
                        .to_owned()
                        .unwrap_or_else(|| DEFAULT_REGION.to_string()),
                    &config
                        .s3_prefix
                        .to_owned()
                        .unwrap_or_else(|| "".to_string()),
                    &config.files_path,
                    config.overwrite,
                    config.clean,
                    config.verbose,
                )
                .await;
            }
        }
    }

    println!("Finding files in {}", &config.files_path);
    let files = get_files_in_dir(config.files_path);

    let processed_files = transform_images(
        files,
        config.tmp_dir.to_owned(),
        &config.crop_sizes,
        config.verbose,
    )
    .await;

    if let Some(s3_bucket_name) = config.s3_bucket_name {
        upload_to_s3(
            &s3_bucket_name,
            &config
                .s3_region
                .unwrap_or_else(|| DEFAULT_REGION.to_string()),
            &config.s3_prefix.unwrap_or_else(|| "".to_string()),
            &config.tmp_dir,
            processed_files,
            config.verbose,
        )
        .await;
    }

    println!("Done!");
}

fn explain_config(config: &Config) {
    println!("Explaining configuration: {:?}", config);

    println!("*************** CONFIGURATION ***************");

    if let Some(s3_bucket_name) = &config.s3_bucket_name {
        println!(
            "Will publish files to S3 bucket '{}' after completion",
            s3_bucket_name
        );

        println!("Will overwrite files on remote: {}", config.overwrite);
    }

    if let Some(fetch_remote) = config.fetch_remote {
        if fetch_remote {
            println!(
                "Fetching files from remote: {}/{}",
                config
                    .s3_bucket_name
                    .as_ref()
                    .expect("need s3 bucket name if going to fetch from remote"),
                config.s3_prefix.as_ref().unwrap_or(&"".to_string())
            );
        }
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
