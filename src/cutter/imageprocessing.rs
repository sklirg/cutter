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
    let numfiles = files.len();
    let operations = numfiles * sizes.len();
    println!("Processing {} files, {} operations", numfiles, operations);

    let mut tasks = Vec::new();
    for f in files {
        for size in sizes {
            let width = size[0];
            let height = size[1];

            let ff = f.to_owned();
            let op = output_path.to_owned();

            let task: tokio::task::JoinHandle<Result<String, _>> = tokio::spawn(async move {
                let thumb_path = format!(
                    "{}/{}",
                    op,
                    generate_thumb_path(&get_file_name(&ff.to_owned()), width, height, "jpg")
                );
                let image = match transform_image(&ff, width, height) {
                    Ok(i) => i,
                    Err(err) => {
                        println!("transform error: {:?}", err);
                        return Err("a");
                    }
                };

                save_image(&image, &thumb_path);
                Ok(thumb_path)
            });

            tasks.push(task);
        }
    }

    let mut created_files = Vec::new();
    let mut counter = 1;
    for task in tasks.into_iter() {
        print_list_iter_status(counter, operations as u32, "Processing", verbose);
        match task.await {
            Ok(res) => {
                let path = match res {
                    Ok(p) => p,
                    Err(err) => {
                        println!("task result err: {}", err);
                        continue;
                    }
                };

                counter += 1;
                created_files.push(path);
            }
            Err(err) => println!("task panicked: {}", err),
        };
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
