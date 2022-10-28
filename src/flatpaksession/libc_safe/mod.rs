pub fn getuid() -> u32 {
    unsafe { libc::getuid() }
}
