use cucumber::{StatsWriter as _, World as _};

mod steps;
mod support;

use support::world::BddWorld;

#[tokio::main]
async fn main() {
    let mut cli =
        cucumber::cli::Opts::<_, cucumber::runner::basic::Cli, _, cucumber::cli::Empty>::parsed();
    if cli.tags_filter.is_none() {
        cli.tags_filter = Some("not @e2e".parse().expect("default tag expression"));
    }
    if support::e2e::requested() {
        support::e2e::require_isolated();
        // The CLI concurrency setting otherwise overrides the runner default.
        cli.runner.concurrency = Some(1);
    }

    let cucumber = BddWorld::cucumber().fail_on_skipped();
    let cucumber = cucumber.with_cli(cli);
    let writer = cucumber
        .before(|feature, rule, scenario, world| {
            let tagged = has_e2e_tag(&scenario.tags)
                || has_e2e_tag(&feature.tags)
                || rule.is_some_and(|rule| has_e2e_tag(&rule.tags));
            Box::pin(async move {
                if tagged {
                    support::e2e::prepare(world);
                }
            })
        })
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

fn has_e2e_tag(tags: &[String]) -> bool {
    tags.iter().any(|tag| tag == "e2e" || tag == "@e2e")
}
