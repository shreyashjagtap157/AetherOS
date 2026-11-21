lines = open('src/main.rs', 'r').readlines()
new_lines = []
inserted = False
for i, line in enumerate(lines):
    new_lines.append(line)
    if i == 130 and 'fn csrw_stvec(val: u64) {' in line:
        # Find the closing brace of csrw_stvec function
        for j in range(i+1, len(lines)):
            new_lines.append(lines[j])
            if lines[j].strip() == '}':
                # Found closing brace of csrw_stvec, insert csrw_sepc after it
                new_lines.append('')
                new_lines.append('fn csrw_sepc(val: u64) {')
                new_lines.append('    unsafe {')
                new_lines.append('        core::arch::asm!("csrw sepc, {}", in(reg) val, options(nomem));')
                new_lines.append('    }')
                new_lines.append('')
                break
open('src/main.rs', 'w').writelines(new_lines)
print('Done')