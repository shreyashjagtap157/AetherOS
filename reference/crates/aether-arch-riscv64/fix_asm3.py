lines = open('src/main.rs', 'r').readlines()
new_lines = []
for i, line in enumerate(lines):
    if i == 316:  # Skip the old asm line outside unsafe block
        continue
    new_lines.append(line)
open('src/main.rs', 'w').writelines(new_lines)
print('Done')