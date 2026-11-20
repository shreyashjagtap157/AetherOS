lines = open('src/main.rs', 'r').readlines()
new_lines = []
inserted = False
for i, line in enumerate(lines):
    new_lines.append(line)
    # After line 136 (index 135) which is '}', insert unsafe extern block
    if i == 135 and lines[i].strip() == '}':
        new_lines.append('unsafe extern "C" {\n')
        new_lines.append('    fn trap_entry();\n')
        new_lines.append('}\n')
        inserted = True
open('src/main.rs', 'w').writelines(new_lines)
print(f'Done. Inserted: {inserted}')
print(f'Total lines: {len(new_lines)}')