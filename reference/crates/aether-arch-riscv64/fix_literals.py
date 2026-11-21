content = open('src/main.rs', 'r').read()
content = content.replace('\\\\n', '\n')
open('src/main.rs', 'w').write(content)
print('Done')