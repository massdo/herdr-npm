use crate::domain::catalog::PackageManager;

/// Quote a script name so a POSIX shell passes it as one argument.
/// Names without shell metacharacters stay unquoted (shortest form).
pub fn quote_script_name(name: &str) -> String {
    if is_safe_unquoted(name) {
        name.to_string()
    } else {
        posix_single_quote(name)
    }
}

fn is_safe_unquoted(name: &str) -> bool {
    !name.is_empty()
        && name.chars().all(|ch| {
            ch.is_ascii_alphanumeric()
                || matches!(ch, '_' | '.' | ':' | '/' | '-' | '+' | '=' | '@' | '%')
        })
}

fn posix_single_quote(name: &str) -> String {
    let mut out = String::from("'");
    for (index, part) in name.split('\'').enumerate() {
        if index > 0 {
            out.push_str("'\\''");
        }
        out.push_str(part);
    }
    out.push('\'');
    out
}

/// `npm run -- <script>` / `pnpm run -- <script>`.
/// `--` keeps names like `--prod` from being read as manager options.
pub fn run_invocation(manager: PackageManager, script: &str) -> String {
    format!("{} run -- {}", manager.as_str(), quote_script_name(script))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_name_is_unquoted() {
        assert_eq!(quote_script_name("build"), "build");
        assert_eq!(quote_script_name("build:prod"), "build:prod");
        assert_eq!(quote_script_name("--prod"), "--prod");
    }

    #[test]
    fn spaces_and_meta_are_single_quoted() {
        assert_eq!(quote_script_name("test watch"), "'test watch'");
        assert_eq!(quote_script_name("say\"hi\""), "'say\"hi\"'");
        assert_eq!(quote_script_name("$HOME"), "'$HOME'");
        assert_eq!(quote_script_name("x$(id)"), "'x$(id)'");
        assert_eq!(quote_script_name("a; id"), "'a; id'");
        assert_eq!(quote_script_name("it's"), "'it'\\''s'");
        assert_eq!(quote_script_name(""), "''");
    }

    #[test]
    fn invocation_inserts_double_dash() {
        assert_eq!(
            run_invocation(PackageManager::Npm, "--prod"),
            "npm run -- --prod"
        );
        assert_eq!(
            run_invocation(PackageManager::Pnpm, "test watch"),
            "pnpm run -- 'test watch'"
        );
    }
}
