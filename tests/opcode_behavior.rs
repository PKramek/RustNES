use RustNES::core::cpu::{Cpu, StepRecord};

#[test]
fn step_record_contract_is_available_for_opcode_tests() {
    let cpu = Cpu::default();
    let record = StepRecord {
        pc_before: cpu.pc,
        opcode: 0,
        bytes: [0; 3],
        byte_len: 1,
        a: cpu.a,
        x: cpu.x,
        y: cpu.y,
        status: cpu.status,
        sp: cpu.sp,
        cyc_before: cpu.total_cycles,
    };

    assert_eq!(record.byte_len, 1);
}