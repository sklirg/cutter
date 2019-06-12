use std::error::Error;

use lambda_runtime::{error::HandlerError, lambda, Context};
use serde::{Deserialize, Serialize};

use cutter::lib::{run, Config, DEFAULT_REGION};

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

fn lambda_handler(event: LambdaEvent, context: Context) -> Result<LambdaOutput, HandlerError> {
    if event.bucket == "" {
        eprintln!("Missing bucket name");
        panic!("Missing bucket name");
    }

    let mut path = event.bucket.to_owned();

    if event.prefix != "" {
        path = event.prefix.to_owned();
    }

    let config = Config {
        clean: false,
        fetch_remote: true,
        files_path: format!("/tmp/{}/{}", event.bucket, event.prefix),
        overwrite: false,
        s3_bucket_name: event.bucket.to_owned(),
        s3_prefix: event.prefix.to_owned(),
        s3_region: DEFAULT_REGION.to_owned(),
        tmp_dir: format!("/tmp/{}", event.bucket),
    };

    run(&config);

    Ok(LambdaOutput {
        message: format!("Success!"),
    })
}
