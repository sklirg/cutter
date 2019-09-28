use std::str;

use super::util::{generate_thumb_path, get_file_name, print_list_iter_status};

extern crate clap;
extern crate raster;

pub fn transform_images(
    files: Vec<String>,
    output_path: &str,
    sizes: &Vec<[i32; 2]>,
    verbose: bool,
) -> Vec<String> {
    let numfiles = files.len().to_owned();
    println!("Processing {} files", numfiles);

    let mut created_files = Vec::new();

    let mut counter = 1;
    for f in files {
        print_list_iter_status(counter, numfiles as u32, "Processing", verbose);
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

pub fn save_image(image: &raster::Image, path: &str) {
    raster::save(&image, &path).unwrap();
}
