pub fn assert_wpc(size: usize, wpc: usize) {
    assert!(
        size % wpc == 0,
        "Assertion failed: Size ({}) must be divisible by WPC ({})",
        size,
        wpc
    );
}
