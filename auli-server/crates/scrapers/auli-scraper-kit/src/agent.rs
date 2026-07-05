use std::time::Duration;

use ureq::Agent;

/// Builds a `ureq` agent with the given browser-like User-Agent and an optional global timeout.
/// (Accept headers, quando necessários, são setados por request pelo chamador.)
pub fn build_agent(user_agent: &str, timeout: Option<Duration>) -> Agent {
    let builder = Agent::config_builder().user_agent(user_agent);
    let builder = match timeout {
        Some(t) => builder.timeout_global(Some(t)),
        None => builder,
    };
    builder.build().into()
}
