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
