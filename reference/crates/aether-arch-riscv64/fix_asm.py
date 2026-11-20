content = open('src/main.rs', 'r').read()
old = '''        *next_frame.add(29) = next_task.t6;
    }
}

    core::arch::asm!("mv sp, {}", in(reg) next_sp, options(nomem));
}
'''
new = '''        *next_frame.add(29) = next_task.t6;
        core::arch::asm!("mv sp, {}", in(reg) next_sp, options(nomem));
    }
}
'''
if old in content:
    content = content.replace(old, new)
    open('src/main.rs', 'w').write(content)
    print('Replacement successful')
else:
    print('Old string not found!')