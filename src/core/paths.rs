use std::env;
use std::path::PathBuf;

const APP_NAME: &str = "trustbridge";

pub fn state_path() -> PathBuf {
    data_dir().join("state.json")
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.yaml")
}

fn data_dir() -> PathBuf {
    if dev_local_enabled() {
        return PathBuf::from(".tbridge");
    }

    platform_data_dir().unwrap_or_else(|| PathBuf::from(".tbridge"))
}

fn config_dir() -> PathBuf {
    if dev_local_enabled() {
        return PathBuf::from(".tbridge");
    }

    platform_config_dir().unwrap_or_else(|| PathBuf::from(".tbridge"))
}

fn dev_local_enabled() -> bool {
    if is_running_via_cargo() {
        return true;
    }

    matches!(
        env::var("TBRIDGE_DEV_LOCAL").ok().as_deref(),
        Some("1") | Some("true") | Some("TRUE") | Some("yes") | Some("YES")
    )
}

fn is_running_via_cargo() -> bool {
    env::var_os("CARGO").is_some() && env::var_os("CARGO_MANIFEST_DIR").is_some()
}

fn platform_data_dir() -> Option<PathBuf> {
    if cfg!(target_os = "macos") {
        let home = env::var("HOME").ok()?;
        return Some(
            PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join(APP_NAME),
        );
    }

    if cfg!(target_os = "windows") {
        let appdata = env::var("APPDATA").ok()?;
        return Some(PathBuf::from(appdata).join(APP_NAME));
    }

    if let Ok(xdg_data_home) = env::var("XDG_DATA_HOME") {
        return Some(PathBuf::from(xdg_data_home).join(APP_NAME));
    }

    let home = env::var("HOME").ok()?;
    Some(PathBuf::from(home).join(".local").join("share").join(APP_NAME))
}

fn platform_config_dir() -> Option<PathBuf> {
    if cfg!(target_os = "macos") {
        let home = env::var("HOME").ok()?;
        return Some(
            PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join(APP_NAME),
        );
    }

    if cfg!(target_os = "windows") {
        let appdata = env::var("APPDATA").ok()?;
        return Some(PathBuf::from(appdata).join(APP_NAME));
    }

    if let Ok(xdg_config_home) = env::var("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(xdg_config_home).join(APP_NAME));
    }

    let home = env::var("HOME").ok()?;
    Some(PathBuf::from(home).join(".config").join(APP_NAME))
}
