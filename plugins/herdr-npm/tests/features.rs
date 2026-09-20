use cucumber::{StatsWriter as _, World as _};

mod steps;
mod support;

use support::world::BddWorld;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let explicit_tags = args.iter().any(|arg| {
        arg == "--tags" || arg == "-t" || arg.starts_with("--tags=") || arg.starts_with("-t=")
    });
    if !explicit_tags && std::env::var_os("CUCUMBER_FILTER_TAGS").is_none() {
        unsafe {
            std::env::set_var("CUCUMBER_FILTER_TAGS", "not @e2e");
        }
    }
    if e2e_selected(&args) && std::env::var("HERDR_NPM_E2E").ok().as_deref() != Some("1") {
        eprintln!(
            "@e2e requires HERDR_NPM_E2E=1 and the isolated Herdr recipe profile; refusing to mutate anything else"
        );
        std::process::exit(1);
    }

    // Parse real CLI (`--tags`, `CUCUMBER_FILTER_TAGS`). `with_default_cli()`
    // would ignore both and always run every scenario.
    let writer = BddWorld::cucumber()
        .fail_on_skipped()
        .run("tests/features")
        .await;

    let executed = writer.passed_steps()
        + writer.skipped_steps()
        + writer.failed_steps()
        + writer.parsing_errors()
        + writer.hook_errors();
    if executed == 0 {
        eprintln!("empty selection: no scenarios matched the tag filter");
        std::process::exit(1);
    }
    if writer.execution_has_failed() {
        std::process::exit(1);
    }
}

fn e2e_selected(args: &[String]) -> bool {
    let from_args = args
        .windows(2)
        .any(|pair| matches!(pair[0].as_str(), "--tags" | "-t") && selects_e2e(&pair[1]))
        || args.iter().any(|arg| {
            arg.strip_prefix("--tags=")
                .or_else(|| arg.strip_prefix("-t="))
                .is_some_and(selects_e2e)
        });
    let from_env = std::env::var("CUCUMBER_FILTER_TAGS")
        .ok()
        .as_deref()
        .is_some_and(selects_e2e);
    from_args || from_env
}

fn selects_e2e(expr: &str) -> bool {
    expr.split_whitespace().any(|token| token == "@e2e")
        && !expr.contains("not @e2e")
        && !expr.contains("not(@e2e)")
}
