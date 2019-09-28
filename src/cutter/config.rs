#[derive(Debug)]
pub struct Config {
    pub clean: bool,
    pub fetch_remote: bool,
    pub files_path: String,
    pub overwrite: bool,
    pub s3_bucket_name: String,
    pub s3_region: String,
    pub s3_prefix: String,
    pub crop_sizes: Vec<[i32; 2]>,
    pub tmp_dir: String,
    pub verbose: bool,
}