mod support;

use RustNES::core::bus::CpuBus;
use RustNES::core::cpu::format_trace_line;
use similar_asserts::assert_eq;

use support::console_from_program;

#[test]
fn nestest_style_trace_matches_the_checked_in_golden_log() {
    let mut console = console_from_program(
        &[
            (0xC000, 0xA2),
            (0xC001, 0x02),
            (0xC002, 0xA9),
            (0xC003, 0x80),
            (0xC004, 0x95),
            (0xC005, 0x10),
            (0xC006, 0xB5),
            (0xC007, 0x10),
            (0xC008, 0xD0),
            (0xC009, 0x02),
            (0xC00A, 0xEA),
            (0xC00B, 0x00),
            (0xC00C, 0x00),
        ],
        0xC000,
    );
    console.bus_mut().write(0x0012, 0x11);

    let mut lines = Vec::new();
    for _ in 0..6 {
        let record = console
            .step_instruction()
            .expect("trace ROM should execute");
        lines.push(format_trace_line(&record));
    }

    let actual = lines.join("\n") + "\n";
    let expected =
        std::fs::read_to_string("tests/roms/nestest.log").expect("golden log should exist");
    assert_eq!(actual, expected);
}
