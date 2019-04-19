pub const BASH_SMART: &str = "bash-smart";
pub const BASH: &str = "bash";
pub const DONKEY_DEPTH_ENV: &str = "__donkey_make_depth__";
pub const DONKEY_FILE_ENV: &str = "__donkey_make_file__";
pub const PATH_STR: &str = ".donkey-make.tmp";
pub const BAR: &str = "==========================================================================================";

#[derive(Debug)]
pub struct CliArgs {
    pub file_path: Option<String>,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub delete_tmp: bool,
}
