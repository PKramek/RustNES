use RustNES::core::console::Console;
use RustNES::core::cartridge::load_cartridge_from_bytes;

fn cartridge_with_vectors(reset: u16) -> RustNES::core::cartridge::Cartridge {
    let mut rom = vec![b'N', b'E', b'S', 0x1A, 2, 1, 0, 0];
    rom.extend_from_slice(&[0; 8]);
    let mut prg = vec![0x00; 0x8000];
    prg[0x7FFC] = (reset & 0x00FF) as u8;
    prg[0x7FFD] = (reset >> 8) as u8;
    rom.extend(prg);
    rom.extend(std::iter::repeat_n(0x00, 0x2000));
    load_cartridge_from_bytes(&rom).expect("fixture cartridge should build")
}

#[test]
fn reset_vector_contract_is_exposed_through_console() {
    let mut console = Console::new(cartridge_with_vectors(0xC123));
    console.reset();
    assert_eq!(console.cpu().pc, 0xC123);
}