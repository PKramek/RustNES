use RustNES::core::bus::InterruptLines;
use RustNES::core::cpu::Cpu;

#[test]
fn interrupt_line_contract_exists_for_targeted_tests() {
    let cpu = Cpu::default();
    let lines = InterruptLines {
        irq: false,
        nmi: true,
        reset: false,
    };

    assert_eq!(cpu.pending_interrupts.irq, false);
    assert!(lines.nmi);
}