//! Markdown frontmatter CLI application entrypoint.
//!
//! A thin binary dispatch wrapper delegating execution to the `mdfm` library crate.

fn main() -> anyhow::Result<()> {
    mdfm::run()
}
