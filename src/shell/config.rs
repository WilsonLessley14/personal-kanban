use std::path::PathBuf;

/// Resolve the database path using priority: CLI flag > KANBAN_DB env var > XDG default.
///
/// If `cli_path` is `Some`, it takes highest priority.
/// Otherwise, the `KANBAN_DB` environment variable is checked.
/// Finally, the XDG default (`$XDG_DATA_HOME/kanban/kanban.db` or `~/.local/share/kanban/kanban.db`) is used.
pub fn resolve_db_path(cli_path: Option<&str>) -> PathBuf {
    if let Some(path) = cli_path {
        return PathBuf::from(path);
    }

    if let Ok(path) = std::env::var("KANBAN_DB") {
        return PathBuf::from(path);
    }

    xdg_default_path()
}

/// Compute the XDG default database path.
fn xdg_default_path() -> PathBuf {
    if let Some(data_home) = std::env::var_os("XDG_DATA_HOME") {
        return PathBuf::from(data_home).join("kanban").join("kanban.db");
    }

    // Fallback to ~/.local/share/kanban/kanban.db
    if let Some(base_dir) = dirs::data_local_dir() {
        return base_dir.join("kanban").join("kanban.db");
    }

    // Last resort fallback
    PathBuf::from(".kanban.db")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::Mutex;

    static LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn resolve_db_path_cli_flag_wins() {
        let path = resolve_db_path(Some("/custom/path.db"));
        assert_eq!(path, PathBuf::from("/custom/path.db"));
    }

    #[test]
    fn resolve_db_path_env_var_wins_over_xdg() {
        let _lock = LOCK.lock().unwrap();
        env::set_var("KANBAN_DB", "/env/path.db");
        let path = resolve_db_path(None);
        assert_eq!(path, PathBuf::from("/env/path.db"));
        env::remove_var("KANBAN_DB");
    }

    #[test]
    fn resolve_db_path_cli_wins_over_env() {
        let _lock = LOCK.lock().unwrap();
        env::set_var("KANBAN_DB", "/env/path.db");
        let path = resolve_db_path(Some("/cli/path.db"));
        assert_eq!(path, PathBuf::from("/cli/path.db"));
        env::remove_var("KANBAN_DB");
    }

    #[test]
    fn resolve_db_path_xdg_default() {
        let _lock = LOCK.lock().unwrap();
        env::remove_var("KANBAN_DB");
        env::remove_var("XDG_DATA_HOME");
        let path = resolve_db_path(None);
        // Should resolve to a kanban.db file
        assert!(path.file_name().map(|n| n == "kanban.db").unwrap_or(false));
        env::remove_var("KANBAN_DB");
        env::remove_var("XDG_DATA_HOME");
    }

    #[test]
    fn resolve_db_path_xdg_data_home() {
        let _lock = LOCK.lock().unwrap();
        env::remove_var("KANBAN_DB");
        env::set_var("XDG_DATA_HOME", "/custom/data");
        let path = resolve_db_path(None);
        assert_eq!(path, PathBuf::from("/custom/data/kanban/kanban.db"));
        env::remove_var("XDG_DATA_HOME");
        env::remove_var("KANBAN_DB");
    }
}
