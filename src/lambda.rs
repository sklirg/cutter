use std::error::Error;

use lambda_runtime::{error::HandlerError, lambda, Context};
use serde::{Deserialize, Serialize};

use cutter::config::Config;
use cutter::lib::{run, DEFAULT_REGION};

mod cutter;

fn main() -> Result<(), Box<dyn Error>> {
    lambda!(lambda_handler);

    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct LambdaEvent {
    bucket: String,
    prefix: String,
}

#[derive(Serialize)]
pub struct LambdaOutput {
    message: String,
}

fn lambda_handler(event: LambdaEvent, _context: Context) -> Result<LambdaOutput, HandlerError> {
    if event.bucket == "" {
        eprintln!("Missing bucket name");
        panic!("Missing bucket name");
    }

    let mut path = event.bucket.to_owned();

    if event.prefix != "" {
        path = event.prefix.to_owned();
    }

    let sizes = vec![
        // Thumbs
        [200, 200],
        [400, 400],
        [800, 800],
        // Full size preview
        [1920, 1080],
    ];

    let config = Config {
        clean: false,
        crop_sizes: sizes,
        fetch_remote: true,
        files_path: format!("/tmp/{}/{}", event.bucket, event.prefix),
        overwrite: false,
        s3_bucket_name: event.bucket.to_owned(),
        s3_prefix: path,
        s3_region: DEFAULT_REGION.to_owned(),
        tmp_dir: format!("/tmp/{}", event.bucket),
        verbose: true,
    };

    run(&config);

    Ok(LambdaOutput {
        message: format!("Success!"),
    })
}
