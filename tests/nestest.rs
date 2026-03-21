use std::path::Path;

#[test]
fn nestest_fixture_paths_exist_for_phase_two() {
    assert!(Path::new("tests/roms/nestest.nes").exists());
    assert!(Path::new("tests/roms/nestest.log").exists());
}