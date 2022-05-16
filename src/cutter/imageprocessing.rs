use std::str;

use image::io::Reader as ImageReader;

use super::util::{generate_thumb_path, get_file_name, print_list_iter_status};

extern crate clap;
extern crate image;

pub fn transform_images(
    files: Vec<String>,
    output_path: &str,
    sizes: &Vec<[u32; 2]>,
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
            let image = match transform_image(&f, width, height) {
                Ok(i) => i,
                Err(err) => {
                    println!("transform error: {:?}", err);
                    continue;
                }
            };

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

    created_files
}

#[derive(Debug)]
pub enum TransformError {
    RasterError(String),
}

fn transform_image(
    path: &str,
    width: u32,
    height: u32,
) -> Result<image::DynamicImage, TransformError> {
    let image_loader = match ImageReader::open(path) {
        Ok(i) => i,
        Err(err) => {
            print!("err open: {:?}", err);
            return Err(TransformError::RasterError(err.to_string()));
        }
    };
    let image = match image_loader.decode() {
        Ok(i) => i,
        Err(err) => return Err(TransformError::RasterError(err.to_string())),
    };
    Ok(image.resize_to_fill(width, height, image::imageops::FilterType::Triangle))
}

pub fn save_image(image: &image::DynamicImage, path: &str) {
    image.save(path).expect("failed to save image")
}
