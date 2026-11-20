path = 'src/main.rs'
content = open(path, 'r').read()

old_start = '''pub extern "C" fn start() -> ! {
    sbi_console_write(b"start\\n");
    sbi_console_write(b"a\\n");

    // Install trap vector (M-mode).
    write_mtvec(trap_entry as *const () as usize as u64);
    sbi_console_write(b"b\\n");

    // Enable machine timer interrupt.
    set_mie(MIE_MTIE);
    sbi_console_write(b"c\\n");
    set_mstatus(MSTATUS_MIE);
    sbi_console_write(b"d\\n");

    // Arm first timer interrupt.
    let now = clint_read_mtime();
    sbi_console_write(b"e\\n");
    clint_write_mtimecmp0(now.wrapping_add(TIMER_INTERVAL));
    sbi_console_write(b"f\\n");

    rust_main()
}'''

new_start = '''pub extern "C" fn start() -> ! {
    // Install trap vector FIRST.
    write_mtvec(trap_entry as *const () as usize as u64);

    sbi_console_write(b"start\\n");

    // Enable machine timer interrupt.
    set_mie(MIE_MTIE);
    set_mstatus(MSTATUS_MIE);

    // Arm first timer interrupt.
    let now = clint_read_mtime();
    clint_write_mtimecmp0(now.wrapping_add(TIMER_INTERVAL));

    sbi_console_write(b"ready\\n");

    rust_main()
}'''

assert old_start in content, "old_start not found"
content = content.replace(old_start, new_start)
open(path, 'w').write(content)
print("Done")'''

# Write and run
open(path, 'w').write(content)
print("Done")'''

# Just do it
path = 'src/main.rs'
content = open(path, 'r').read()

old_start = '''pub extern "C" fn start() -> ! {
    sbi_console_write(b"start\\n");
    sbi_console_write(b"a\\n");

    // Install trap vector (M-mode).
    write_mtvec(trap_entry as *const () as usize as u64);
    sbi_console_write(b"b\\n");

    // Enable machine timer interrupt.
    set_mie(MIE_MTIE);
    sbi_console_write(b"c\\n");
    set_mstatus(MSTATUS_MIE);
    sbi_console_write(b"d\\n");

    // Arm first timer interrupt.
    let now = clint_read_mtime();
    sbi_console_write(b"e\\n");
    clint_write_mtimecmp0(now.wrapping_add(TIMER_INTERVAL));
    sbi_console_write(b"f\\n");

    rust_main()
}'''

new_start = '''pub extern "C" fn start() -> ! {
    // Install trap vector FIRST.
    write_mtvec(trap_entry as *const () as usize as u64);

    sbi_console_write(b"start\\n");

    // Enable machine timer interrupt.
    set_mie(MIE_MTIE);
    set_mstatus(MSTATUS_MIE);

    // Arm first timer interrupt.
    let now = clint_read_mtime();
    clint_write_mtimecmp0(now.wrapping_add(TIMER_INTERVAL));

    sbi_console_write(b"ready\\n");

    rust_main()
}'''

assert old_start in content, "old_start not found"
content = content.replace(old_start, new_start)
open(path, 'w').write(content)
print("Done")