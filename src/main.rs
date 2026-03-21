fn main() -> anyhow::Result<()> {
    RustNES::shell::run(RustNES::shell::BootOptions::from_env())
}