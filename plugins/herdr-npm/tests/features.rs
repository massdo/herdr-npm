use cucumber::{StatsWriter as _, World as _};

mod steps;
mod support;

use support::world::BddWorld;

#[tokio::main]
async fn main() {
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
