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
            .expect("trace program should execute");
        lines.push(format_trace_line(&record));
    }

    let actual = lines.join("\n") + "\n";
    let expected = concat!(
        "C000  A2 02     LDX #$02                        A:00 X:00 Y:00 P:24 SP:FD PPU:  0, 21 CYC:7\n",
        "C002  A9 80     LDA #$80                        A:00 X:02 Y:00 P:24 SP:FD PPU:  0, 27 CYC:9\n",
        "C004  95 10     STA $10,X @ 12 = 11             A:80 X:02 Y:00 P:A4 SP:FD PPU:  0, 33 CYC:11\n",
        "C006  B5 10     LDA $10,X @ 12 = 80             A:80 X:02 Y:00 P:A4 SP:FD PPU:  0, 45 CYC:15\n",
        "C008  D0 02     BNE $C00C                       A:80 X:02 Y:00 P:A4 SP:FD PPU:  0, 57 CYC:19\n",
        "C00C  00        BRK                             A:80 X:02 Y:00 P:A4 SP:FD PPU:  0, 66 CYC:22\n",
    );
    assert_eq!(actual, expected);
}
