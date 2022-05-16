use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::str;

use super::util::print_list_iter_status;

pub async fn download_from_s3(
    bucket: &str,
    _region: &str,
    prefix: &str,
    local_path: &str,
    overwrite: bool,
    clean: bool,
    verbose: bool,
) {
    println!(
        "Downloading files from S3 bucket '{}' ({})...",
        bucket, prefix
    );
    let config = aws_config::load_from_env().await;
    let client = aws_sdk_s3::Client::new(&config);

    let resp = client
        .list_objects_v2()
        .bucket(bucket)
        .send()
        .await
        .expect("failed to send s3 request");
    let bucket_contents = resp.contents().unwrap_or_default();

    let mut all_files = Vec::new();

    for obj in bucket_contents {
        all_files.push(obj.key().expect("failed to get object key"));
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

        let _thumb_key = &file.replace(".jpg", "_thumb.jpg");

        let valid_file_name = !file.is_empty() && file != &format!("{}/", prefix);
        let has_sizes = file.contains('_');

        if valid_file_name && overwrite || !has_sizes {
            files.push(file);
        } else {
            skipped += 1;
        }
    }

    let root_dir = local_path;

    println!(
        "Downloading {} files to {} (skipped {})",
        files.len(),
        &root_dir,
        skipped
    );
    let numfiles = files.len();
    let mut counter = 1;

    if Path::new(&root_dir).exists() && (clean || overwrite) {
        println!("Removing existing directory...");
        fs::remove_dir_all(&root_dir).unwrap();
    }
    fs::create_dir_all(&root_dir).unwrap();

    for file in &files {
        let gallery_image: Vec<&str> = file.split('/').collect();
        let mut path = format!("{}/{}", local_path, &file);
        if gallery_image.len() > 1 {
            path = format!("{}/{}", local_path, &gallery_image[1]);
        }
        print_list_iter_status(counter, numfiles as u32, "Downloaded", verbose);

        let resp = client
            .get_object()
            .bucket(bucket)
            .key(file.to_string())
            .send()
            .await
            .expect("failed to download file");
        let data = resp.body.collect().await.expect("failed to collect data");
        let mut buffer = File::create(path).unwrap();
        buffer.write_all(&data.into_bytes()).unwrap();
        counter += 1;
    }
}

pub async fn upload_to_s3(
    bucket: &str,
    _region: &str,
    prefix: &str,
    _tmp_dir: &str,
    files: Vec<String>,
    verbose: bool,
) {
    let config = aws_config::load_from_env().await;
    let client = aws_sdk_s3::Client::new(&config);

    println!("Uploading {} files to S3 bucket '{}'", files.len(), bucket,);

    let mut counter = 1;
    let numfiles = files.len();
    for file in &files {
        print_list_iter_status(counter, numfiles as u32, "Uploaded", verbose);
        let body = aws_sdk_s3::types::ByteStream::from_path(Path::new(file))
            .await
            .expect("failed to read file contents");
        // @ToDo: Fix output if files are served locally.
        // They're currently prefixed with the folder name sent in through config
        // But need the prefix from S3.
        let file_name = Path::new(file)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();

        let s3_file_path = format!("{}/{}", prefix, &file_name);
        client
            .put_object()
            .bucket(bucket)
            .key(s3_file_path)
            .body(body)
            .send()
            .await
            .expect("failed to upload");
        counter += 1;
    }
}
