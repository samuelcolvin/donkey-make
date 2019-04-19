pub const BASH_SMART: &str = "bash-smart";
pub const BASH: &str = "bash";
pub const DONKEY_DEPTH_ENV: &str = "DONKEY_MAKE_DEPTH";
pub const DONKEY_FILE_ENV: &str = "DONKEY_MAKE_CONFIG_FILE";
pub const DONKEY_COMMAND_ENV: &str = "DONKEY_MAKE_COMMAND";
pub const DONKEY_KEEP_ENV: &str = "DONKEY_MAKE_KEEP";
pub const PATH_STR: &str = ".donkey-make.tmp";
pub const BAR: &str = "==========================================================================================";

#[derive(Debug)]
pub struct CliArgs {
    pub file_path: Option<String>,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub keep_tmp: bool,
}
