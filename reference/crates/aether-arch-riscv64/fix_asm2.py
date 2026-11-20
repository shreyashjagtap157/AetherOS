lines = open('src/main.rs', 'r').readlines()
new_lines = []
for i, line in enumerate(lines):
    new_lines.append(line)
    if i == 312:  # After line 313 (*next_frame.add(29) = next_task.t6;)
        new_lines.append('        core::arch::asm!("mv sp, {}", in(reg) next_sp, options(nomem));\n')
    if i == 315:  # Skip line 316 (the old asm call outside unsafe block)
        pass
open('src/main.rs', 'w').writelines(new_lines)
print('Done')