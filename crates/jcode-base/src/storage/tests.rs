use super::*;

#[cfg(unix)]
#[test]
fn harden_secret_file_permissions_sets_owner_only_modes() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::TempDir::new().expect("create temp dir");
    let secret_dir = dir.path().join("jcode");
    std::fs::create_dir_all(&secret_dir).expect("create secret dir");

    let secret_file = secret_dir.join("openrouter.env");
    std::fs::write(&secret_file, "OPENROUTER_API_KEY=sk-or-v1-test\n").expect("write secret file");

    std::fs::set_permissions(&secret_dir, std::fs::Permissions::from_mode(0o755))
        .expect("set initial dir perms");
    std::fs::set_permissions(&secret_file, std::fs::Permissions::from_mode(0o644))
        .expect("set initial file perms");

    harden_secret_file_permissions(&secret_file);

    let dir_mode = std::fs::metadata(&secret_dir)
        .expect("stat dir")
        .permissions()
        .mode()
        & 0o777;
    let file_mode = std::fs::metadata(&secret_file)
        .expect("stat file")
        .permissions()
        .mode()
        & 0o777;

    assert_eq!(dir_mode, 0o700);
    assert_eq!(file_mode, 0o600);
}

#[test]
fn user_home_path_uses_external_dir_under_jcode_home() {
    let _guard = lock_test_env();
    let prev_mercury_home = std::env::var_os("MERCURY_HOME");
    let prev_home = std::env::var_os("JCODE_HOME");
    let temp = tempfile::TempDir::new().expect("create temp dir");
    crate::env::remove_var("MERCURY_HOME");
    crate::env::set_var("JCODE_HOME", temp.path());

    let resolved = user_home_path(".codex/auth.json").expect("resolve user home path");
    assert_eq!(
        resolved,
        temp.path()
            .join("external")
            .join(".codex")
            .join("auth.json")
    );

    if let Some(prev_mercury_home) = prev_mercury_home {
        crate::env::set_var("MERCURY_HOME", prev_mercury_home);
    } else {
        crate::env::remove_var("MERCURY_HOME");
    }
    if let Some(prev_home) = prev_home {
        crate::env::set_var("JCODE_HOME", prev_home);
    } else {
        crate::env::remove_var("JCODE_HOME");
    }
}

#[cfg(unix)]
#[test]
fn validate_external_auth_file_rejects_symlink() {
    use std::os::unix::fs as unix_fs;

    let dir = tempfile::TempDir::new().expect("create temp dir");
    let target = dir.path().join("auth.json");
    let link = dir.path().join("auth-link.json");
    std::fs::write(&target, "{}\n").expect("write target");
    unix_fs::symlink(&target, &link).expect("create symlink");

    let err = validate_external_auth_file(&link).expect_err("symlink should be rejected");
    assert!(err.to_string().contains("symlink"));
}

#[test]
fn app_config_dir_uses_jcode_home_when_set() {
    let _guard = lock_test_env();
    let prev_mercury_home = std::env::var_os("MERCURY_HOME");
    let prev_home = std::env::var_os("JCODE_HOME");
    let temp = tempfile::TempDir::new().expect("create temp dir");
    crate::env::remove_var("MERCURY_HOME");
    crate::env::set_var("JCODE_HOME", temp.path());

    let resolved = app_config_dir().expect("resolve app config dir");
    assert_eq!(resolved, temp.path().join("config").join("jcode"));

    if let Some(prev_mercury_home) = prev_mercury_home {
        crate::env::set_var("MERCURY_HOME", prev_mercury_home);
    } else {
        crate::env::remove_var("MERCURY_HOME");
    }
    if let Some(prev_home) = prev_home {
        crate::env::set_var("JCODE_HOME", prev_home);
    } else {
        crate::env::remove_var("JCODE_HOME");
    }
}

#[test]
fn jcode_dir_uses_mercury_home_when_set() {
    let _guard = lock_test_env();
    let prev_mercury_home = std::env::var_os("MERCURY_HOME");
    let prev_jcode_home = std::env::var_os("JCODE_HOME");
    let mercury_home = tempfile::TempDir::new().expect("create mercury temp dir");

    crate::env::set_var("MERCURY_HOME", mercury_home.path());
    crate::env::remove_var("JCODE_HOME");

    assert_eq!(jcode_dir().expect("resolve jcode dir"), mercury_home.path());

    if let Some(prev_mercury_home) = prev_mercury_home {
        crate::env::set_var("MERCURY_HOME", prev_mercury_home);
    } else {
        crate::env::remove_var("MERCURY_HOME");
    }
    if let Some(prev_jcode_home) = prev_jcode_home {
        crate::env::set_var("JCODE_HOME", prev_jcode_home);
    } else {
        crate::env::remove_var("JCODE_HOME");
    }
}

#[test]
fn mercury_home_takes_precedence_over_jcode_home() {
    let _guard = lock_test_env();
    let prev_mercury_home = std::env::var_os("MERCURY_HOME");
    let prev_jcode_home = std::env::var_os("JCODE_HOME");
    let mercury_home = tempfile::TempDir::new().expect("create mercury temp dir");
    let jcode_home = tempfile::TempDir::new().expect("create jcode temp dir");

    crate::env::set_var("MERCURY_HOME", mercury_home.path());
    crate::env::set_var("JCODE_HOME", jcode_home.path());

    let resolution = config_home_env_resolution();
    assert_eq!(resolution.explicit_source, Some("MERCURY_HOME"));
    assert_eq!(resolution.explicit_home.as_deref(), Some(mercury_home.path()));
    assert!(resolution.has_conflict());
    assert_eq!(jcode_dir().expect("resolve jcode dir"), mercury_home.path());
    assert_eq!(
        app_config_dir().expect("resolve app config dir"),
        mercury_home.path().join("config").join("jcode")
    );

    if let Some(prev_mercury_home) = prev_mercury_home {
        crate::env::set_var("MERCURY_HOME", prev_mercury_home);
    } else {
        crate::env::remove_var("MERCURY_HOME");
    }
    if let Some(prev_jcode_home) = prev_jcode_home {
        crate::env::set_var("JCODE_HOME", prev_jcode_home);
    } else {
        crate::env::remove_var("JCODE_HOME");
    }
}

#[test]
fn upsert_env_file_value_writes_replaces_and_removes_entries() {
    let dir = tempfile::TempDir::new().expect("create temp dir");
    let file = dir.path().join("test.env");

    upsert_env_file_value(&file, "API_KEY", Some("one")).expect("write initial env value");
    assert_eq!(
        std::fs::read_to_string(&file).expect("read env file"),
        "API_KEY=one\n"
    );

    upsert_env_file_value(&file, "OTHER", Some("two")).expect("append second value");
    upsert_env_file_value(&file, "API_KEY", Some("updated")).expect("replace existing value");
    assert_eq!(
        std::fs::read_to_string(&file).expect("read env file after replace"),
        "API_KEY=updated\nOTHER=two\n"
    );

    upsert_env_file_value(&file, "API_KEY", None).expect("remove env value");
    assert_eq!(
        std::fs::read_to_string(&file).expect("read env file after remove"),
        "OTHER=two\n"
    );
}

#[cfg(unix)]
#[test]
fn write_text_secret_sets_owner_only_modes() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::TempDir::new().expect("create temp dir");
    let file = dir.path().join("secret.env");

    write_text_secret(&file, "SECRET=value\n").expect("write secret text");

    let dir_mode = std::fs::metadata(dir.path())
        .expect("stat dir")
        .permissions()
        .mode()
        & 0o777;
    let file_mode = std::fs::metadata(&file)
        .expect("stat file")
        .permissions()
        .mode()
        & 0o777;

    assert_eq!(dir_mode, 0o700);
    assert_eq!(file_mode, 0o600);
}
