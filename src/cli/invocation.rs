use std::ffi::OsStr;

const DEFAULT_CLI_NAME: &str = "jcode";

pub(crate) fn invoked_cli_name() -> &'static str {
    invoked_cli_name_from(std::env::args_os().next().as_deref()).unwrap_or(DEFAULT_CLI_NAME)
}

pub(crate) fn invoked_cli_command(subcommand: &str) -> String {
    command_for_cli_name(invoked_cli_name(), subcommand)
}

pub(crate) fn command_for_cli_name(cli_name: &str, subcommand: &str) -> String {
    format!("{cli_name} {subcommand}")
}

pub(crate) fn invoked_cli_name_from(argv0: Option<&OsStr>) -> Option<&'static str> {
    let value = argv0.and_then(OsStr::to_str)?;
    let file_name = value
        .rsplit(['/', '\\'])
        .next()
        .filter(|name| !name.is_empty())?;
    let stem = if file_name
        .get(file_name.len().saturating_sub(4)..)
        .is_some_and(|suffix| suffix.eq_ignore_ascii_case(".exe"))
    {
        &file_name[..file_name.len() - 4]
    } else {
        file_name
    };

    if stem.eq_ignore_ascii_case("mercury") {
        Some("mercury")
    } else if stem.eq_ignore_ascii_case("jcode") {
        Some("jcode")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;

    #[test]
    fn invoked_cli_name_from_detects_mercury_alias() {
        assert_eq!(
            invoked_cli_name_from(Some(OsStr::new("mercury"))),
            Some("mercury")
        );
        assert_eq!(
            invoked_cli_name_from(Some(OsStr::new("C:\\tools\\mercury.exe"))),
            Some("mercury")
        );
    }

    #[test]
    fn invoked_cli_name_from_preserves_jcode_binary() {
        assert_eq!(
            invoked_cli_name_from(Some(OsStr::new("jcode"))),
            Some("jcode")
        );
        assert_eq!(
            invoked_cli_name_from(Some(OsStr::new("/usr/local/bin/jcode"))),
            Some("jcode")
        );
    }

    #[test]
    fn invoked_cli_name_from_falls_back_for_unknown_invocations() {
        assert_eq!(invoked_cli_name_from(None), None);
        assert_eq!(invoked_cli_name_from(Some(OsStr::new("cargo-test"))), None);
    }

    #[test]
    fn command_for_cli_name_formats_subcommands() {
        assert_eq!(
            command_for_cli_name("mercury", "workspace init"),
            "mercury workspace init"
        );
    }
}
