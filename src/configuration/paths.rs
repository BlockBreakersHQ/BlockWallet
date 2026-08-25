use directories::ProjectDirs;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const QUALIFIER: &str = "org";
const ORGANIZATION: &str = "BlockBreakers";
const APPLICATION: &str = "blockwallet";

fn project_dirs() -> io::Result<ProjectDirs> {
    ProjectDirs::from(QUALIFIER, ORGANIZATION, APPLICATION).ok_or_else(|| {
        io::Error::new(io::ErrorKind::NotFound, "could not determine home directory")
    })
}

fn ensure_dir(path: PathBuf) -> io::Result<PathBuf> {
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn override_root() -> Option<PathBuf> {
    std::env::var_os("BLOCKWALLET_HOME").map(PathBuf::from)
}

pub fn config_dir() -> io::Result<PathBuf> {
    if let Some(root) = override_root() {
        return ensure_dir(root.join("config"));
    }
    ensure_dir(project_dirs()?.config_dir().to_path_buf())
}

pub fn data_dir() -> io::Result<PathBuf> {
    if let Some(root) = override_root() {
        return ensure_dir(root.join("data"));
    }
    ensure_dir(project_dirs()?.data_dir().to_path_buf())
}

pub fn cache_dir() -> io::Result<PathBuf> {
    if let Some(root) = override_root() {
        return ensure_dir(root.join("cache"));
    }
    ensure_dir(project_dirs()?.cache_dir().to_path_buf())
}

pub fn state_dir() -> io::Result<PathBuf> {
    if let Some(root) = override_root() {
        return ensure_dir(root.join("state"));
    }
    let dirs = project_dirs()?;
    match dirs.state_dir() {
        Some(path) => ensure_dir(path.to_path_buf()),
        None => ensure_dir(dirs.data_dir().join("log")),
    }
}

pub fn wallet_store_path() -> io::Result<PathBuf> {
    Ok(data_dir()?.join("Config.dic"))
}

pub fn network_config_path() -> io::Result<PathBuf> {
    Ok(config_dir()?.join("network.yml"))
}

pub fn log_path() -> io::Result<PathBuf> {
    let path = state_dir()?.join("blockwallet.log");
    if !path.exists() {
        fs::File::create(&path)?;
    }
    Ok(path)
}

pub fn backup_dir() -> io::Result<PathBuf> {
    ensure_dir(data_dir()?.join("backups"))
}

pub fn currency_details_path() -> io::Result<PathBuf> {
    Ok(cache_dir()?.join("CurrencyDetails.json"))
}

pub fn images_path() -> io::Result<PathBuf> {
    let mut candidates = Vec::new();
    let bundled = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Images");
    candidates.push(bundled);
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("Images"));
            candidates.push(dir.join("../share/blockwallet/Images"));
        }
    }
    candidates.push(PathBuf::from("/app/share/blockwallet/Images"));
    candidates.push(PathBuf::from("/usr/share/blockwallet/Images"));
    candidates.push(PathBuf::from("/usr/local/share/blockwallet/Images"));
    for path in candidates {
        if path.is_dir() {
            return Ok(path);
        }
    }
    ensure_dir(data_dir()?.join("images"))
}

pub fn icon_cache_path() -> io::Result<PathBuf> {
    ensure_dir(cache_dir()?.join("icons"))
}

pub fn btc_cache_dir() -> io::Result<PathBuf> {
    ensure_dir(cache_dir()?.join("btc"))
}

pub fn token_icon_path(symbol: &str) -> PathBuf {
    let file = format!("{}.png", symbol);
    let cached = icon_cache_path()
        .map(|dir| dir.join(&file))
        .unwrap_or_default();
    if cached.is_file() {
        return cached;
    }
    match images_path() {
        Ok(mut bundled) => {
            bundled.push("Icons");
            bundled.push(&file);
            bundled
        }
        Err(_) => cached,
    }
}

pub fn is_user_writable(path: &Path) -> bool {
    let Ok(data) = data_dir() else { return false };
    let Ok(config) = config_dir() else { return false };
    let Ok(cache) = cache_dir() else { return false };
    let Ok(state) = state_dir() else { return false };
    path.starts_with(&data)
        || path.starts_with(&config)
        || path.starts_with(&cache)
        || path.starts_with(&state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wallet_store_lives_under_xdg_not_next_to_the_binary() {
        let store = wallet_store_path().unwrap();
        assert_eq!(store.file_name().unwrap(), "Config.dic");
        assert!(store.parent().unwrap().exists());

        let mut exe_dir = std::env::current_exe().unwrap();
        exe_dir.pop();
        assert_ne!(store.parent().unwrap(), exe_dir.as_path());
        assert!(!store.to_string_lossy().contains("/Users/andy"));
    }

    #[test]
    fn default_usdc_icon_is_not_a_hardcoded_dev_path() {
        let icon = token_icon_path("USDC");
        assert!(!icon.to_string_lossy().contains("/Users/andy"));
    }

    #[test]
    fn images_path_resolves_a_directory() {
        let path = images_path().unwrap();
        assert!(path.is_dir());
        assert!(!path.to_string_lossy().contains("/Users/andy/Documents/Dev"));
    }
}
