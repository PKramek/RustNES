mod support;

use std::ffi::OsString;
use std::path::PathBuf;

use RustNES::shell::{ShellCommand, TraceOptions};
use assert_cmd::Command;

use support::{unique_temp_path, write_rom};

#[test]
fn trace_command_contract_parses_expected_flags() {
    let args = vec![
        OsString::from("rustnes"),
        OsString::from("trace"),
        OsString::from("example.nes"),
        OsString::from("--pc"),
        OsString::from("C000"),
        OsString::from("--output"),
        OsString::from("trace.log"),
        OsString::from("--max-instructions"),
        OsString::from("5003"),
    ];

    let command = ShellCommand::from_args(args).expect("trace command should parse");
    match command {
        ShellCommand::Trace(TraceOptions {
            rom_path,
            start_pc,
            output,
            max_instructions,
        }) => {
            assert_eq!(rom_path, PathBuf::from("example.nes"));
            assert_eq!(start_pc, Some(0xC000));
            assert_eq!(output, Some(PathBuf::from("trace.log")));
            assert_eq!(max_instructions, Some(5003));
        }
        other => panic!("expected trace command, got {other:?}"),
    }
}

#[test]
fn trace_command_writes_stdout_and_honors_instruction_cap() {
    let rom_path = unique_temp_path("trace-stdout", "nes");
    write_rom(
        &rom_path,
        &[
            (0xC000, 0xA9),
            (0xC001, 0x01),
            (0xC002, 0xEA),
            (0xC003, 0x00),
        ],
        0xC000,
    );

    let assert = Command::cargo_bin("RustNES")
        .expect("binary should build")
        .args([
            "trace",
            rom_path.to_str().expect("temp path should be utf-8"),
            "--max-instructions",
            "2",
        ])
        .assert()
        .success();

    let output = String::from_utf8(assert.get_output().stdout.clone())
        .expect("trace stdout should be valid utf-8");
    assert!(output.contains("A9 01"));
    assert!(output.contains("EA"));
    assert!(!output.contains("BRK"));
    assert_eq!(output.lines().count(), 2);

    let _ = std::fs::remove_file(&rom_path);
}

#[test]
fn trace_command_can_write_to_a_file() {
    let rom_path = unique_temp_path("trace-file-rom", "nes");
    let output_path = unique_temp_path("trace-file-out", "log");
    write_rom(&rom_path, &[(0xC000, 0xEA), (0xC001, 0x00)], 0xC000);

    Command::cargo_bin("RustNES")
        .expect("binary should build")
        .args([
            "trace",
            rom_path.to_str().expect("temp path should be utf-8"),
            "--output",
            output_path.to_str().expect("temp path should be utf-8"),
            "--max-instructions",
            "1",
        ])
        .assert()
        .success();

    let output = std::fs::read_to_string(&output_path).expect("trace file should exist");
    assert!(output.contains("EA"));

    let _ = std::fs::remove_file(&rom_path);
    let _ = std::fs::remove_file(&output_path);
}
