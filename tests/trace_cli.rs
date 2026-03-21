use std::ffi::OsString;
use std::path::PathBuf;

use RustNES::shell::{ShellCommand, TraceOptions};

#[test]
fn trace_command_contract_parses_expected_flags() {
    let args = vec![
        OsString::from("rustnes"),
        OsString::from("trace"),
        OsString::from("tests/roms/nestest.nes"),
        OsString::from("--pc"),
        OsString::from("C000"),
        OsString::from("--output"),
        OsString::from("trace.log"),
        OsString::from("--max-instructions"),
        OsString::from("5003"),
    ];

    let command = ShellCommand::from_args(args).expect("trace command should parse");
    match command {
        ShellCommand::Trace(TraceOptions { rom_path, start_pc, output, max_instructions }) => {
            assert_eq!(rom_path, PathBuf::from("tests/roms/nestest.nes"));
            assert_eq!(start_pc, Some(0xC000));
            assert_eq!(output, Some(PathBuf::from("trace.log")));
            assert_eq!(max_instructions, Some(5003));
        }
        other => panic!("expected trace command, got {other:?}"),
    }
}