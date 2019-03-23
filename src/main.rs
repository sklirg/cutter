use std::env;
use std::fs;
use std::path::Path;
use std::process;

extern crate raster;

fn main() {
    let filepath = process_args();
    // if args.len() == 1 {
    //     eprintln!("Missing path argument! Simply supply it after the binary.");
    //     process::exit(1);
    // }

    // let filepath = &args[1];

    println!("Path arg: {}", filepath);

    let files = get_files_in_dir(&filepath);

    transform_images(files, &filepath);

    println!("Done!")
}

// App config
fn process_args() -> String {
    let args: Vec<String> = env::args().collect();

    match args.len() {
        1...2 => panic!("Missing args"),
        // 2 => println!("Missing args"),
        3 => process_two_args(args),
        _ => panic!("Yikes"),
    }
}

fn process_two_args(args: Vec<String>) -> String {
    let first_arg = &args[1];

    match first_arg.as_str() {
        "path" => return args[1].to_owned(),
        "s3" => return download_from_s3(&args[2]),
        _ => panic!("Unknown operation"),
    }
}

fn download_from_s3(bucket: &str) -> String {
    println!("Downloading files from S3 bucket '{}'...", bucket);
    // @ ToDo
    return "local_path".to_owned();
}
// End config

fn transform_images(files: Vec<String>, output_path: &str) {
    let numfiles = files.len().to_owned();
    println!("Processing {} files", numfiles);

    let mut counter = 0;
    for f in files {
        counter += 1;
        if counter % 10 == 0 {
            println!("... {}/{}", counter, numfiles);
        }
        let thumb_path = format!(
            "{}-thumbs/{}",
            output_path,
            generate_thumb_path(&get_file_name(&f), "jpg")
        );
        let mut image = raster::open(&f).unwrap();
        transform_image(&mut image);
        save_image(&image, &thumb_path);
    }
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
