use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::str;

use s3::bucket::Bucket;
use s3::credentials::Credentials;

use super::config::Config;
use super::util::print_list_iter_status;

pub fn download_from_s3(config: &Config) {
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
    let mut counter = 1;

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

pub fn upload_to_s3(config: &Config, files: Vec<String>) {
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
    let mut counter = 1;
    let numfiles = files.len();
    for file in &files {
        print_list_iter_status(counter, numfiles as u32, "Uploaded", config.verbose);
        let mut buf = Vec::new();
        File::open(&file).unwrap().read_to_end(&mut buf).unwrap();
        // @ToDo: Fix output if files are served locally.
        // They're currently prefixed with the folder name sent in through config
        // But need the prefix from S3.
        let file_name = &file.replace(&config.tmp_dir, "");
        let s3_file_path = format!("{}/{}", &config.s3_prefix, &file_name);
        bucket.put(&s3_file_path, &buf, "image/jpeg").unwrap();
        counter += 1;
    }
}
