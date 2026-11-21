lines = open('src/main.rs', 'r').readlines()
new_lines = []
for i, line in enumerate(lines):
    new_lines.append(line)
    if i == 56 and 'core::arch::asm!("csrw stvec' in line:
        new_lines.append('')
        new_lines.append('fn csrw_sepc(val: u64) {')
        new_lines.append('    unsafe {')
        new_lines.append('        core::arch::asm!("csrw sepc, {}", in(reg) val, options(nomem));')
        new_lines.append('    }')
        new_lines.append('')
open('src/main.rs', 'w').writelines(new_lines)
print('Done')