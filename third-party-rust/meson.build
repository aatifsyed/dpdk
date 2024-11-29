# we're ignoring libc's build script... :D
libc = library('libc', sources: ['.vendor/libc/src/lib.rs'], rust_args: ['--edition=2015'])
# meson bug: using '--cfg=feature="libc"' gives us weird rust-project.json
seestr = library('seestr', sources: ['.vendor/seestr/src/lib.rs'], rust_args: ['--edition=2021', '--cfg', 'feature="libc"', '--cfg', 'feature="std"', '--cfg', 'feature="alloc"'], link_with: [libc])