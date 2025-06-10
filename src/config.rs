use std::{env, path::PathBuf};

#[derive(Clone)]
pub struct CapturebotConfig {
    pub user_id: u64,
    pub read_dir: PathBuf,
    pub save_dir: PathBuf,
    pub backup_json: Option<PathBuf>,
}

impl CapturebotConfig {
    pub fn from_env() -> Self {
	let save_dir = PathBuf::from(
                env::var("CAPTUREBOT_SAVE_DIR").unwrap_or_else(|_| "./out/".to_string()),
        );
	let r = Self {
            user_id: env::var("CAPTUREBOT_USER_ID")
                .expect("Specify user ID")
                .parse::<u64>()
                .expect("User ID should be an integer"),
	    read_dir: env::var("CAPTUREBOT_READ_DIR").map_or(save_dir.clone(), PathBuf::from),
            save_dir,
            backup_json: env::var("CAPTUREBOT_BACKUP_LOCATION")
                .ok()
                .map(PathBuf::from),
        };
	println!("{:?} {:?} {:?}", r.user_id, r.save_dir, r.backup_json);
	r
    }

    #[cfg(test)]
    pub fn for_testing(test_name: &str) -> Self {
        Self {
            user_id: 12345,
            save_dir: PathBuf::from(format!("/tmp/test_out/{}/", test_name)),
            backup_json: Some(PathBuf::from("./test_backup.json".to_string())),
	    read_dir: PathBuf::from(format!("/tmp/test_out/read/{}/", test_name))
        }
    }
}
