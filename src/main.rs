fn main() -> anyhow::Result<()> {
    RustNES::shell::run(RustNES::shell::ShellCommand::from_env()?)
}