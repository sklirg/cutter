use std::str;

use image::io::Reader as ImageReader;

use super::util::{generate_thumb_path, get_file_name, print_list_iter_status};

extern crate clap;
extern crate image;

pub async fn transform_images(
    files: Vec<String>,
    output_path: String,
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

            let fp = f.to_owned();
            let op = output_path.to_owned();

            let thumb_path = format!(
                "{}/{}",
                op,
                generate_thumb_path(&get_file_name(&fp.to_owned()), width, height, "jpg")
            );
            let thumb_path2 = thumb_path.to_owned();
            match tokio::spawn(async move {
                let image = match transform_image(&fp, width, height) {
                    Ok(i) => i,
                    Err(err) => {
                        println!("transform error: {:?}", err);
                        return;
                    }
                };

                save_image(&image, &thumb_path2);
            }).await {
                Ok(()) => (),
                Err(err) => println!("failed to spawn task: {}", err),
            };

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
