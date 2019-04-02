use std::error::Error;

use lambda_runtime::lambda;

use cutter::main::lambda_handler;

mod cutter;

fn main() -> Result<(), Box<dyn Error>> {
    lambda!(lambda_handler);

    Ok(())
}
