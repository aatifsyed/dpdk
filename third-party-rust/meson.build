libc = library(
    'libc',
    # we're ignoring libc's build script... :D
    sources: ['.vendor/libc/src/lib.rs'],
    rust_args: ['--edition=2015'],
)
memchr = library(
    'memchr',
    sources: ['.vendor/memchr/src/lib.rs'],
    rust_args: ['--edition=2021'],
)
minimal_lexical = library(
    'minimal_lexical',
    sources: ['.vendor/minimal-lexical/src/lib.rs'],
    rust_args: [
        '--edition=2018',
        '--cfg', 'feature="std"',
    ],
)
nom = library(
    'nom',
    sources: ['.vendor/nom/src/lib.rs'],
    rust_args: [
        '--edition=2018',
        '--cfg', 'feature="std"',
        '--cfg', 'feature="alloc"',
    ],
    link_with: [memchr, minimal_lexical],
)
number_prefix = library(
    'number_prefix',
    sources: ['.vendor/number_prefix/src/lib.rs'],
    # meson bug: using '--cfg=feature="..."' gives us a malformed rust-project.json
    rust_args: [
        '--cfg', 'feature="std"',
    ],
)
seestr = library(
    'seestr',
    sources: ['.vendor/seestr/src/lib.rs'],
    rust_args: [
        '--edition=2021',
        '--cfg', 'feature="libc"',
        '--cfg', 'feature="std"',
        '--cfg', 'feature="alloc"',
    ],
    link_with: [libc],
)
trybox = library(
    'trybox',
    sources: ['.vendor/trybox/src/lib.rs'],
    rust_args: [
        '--edition=2021',
        '--cfg', 'feature="std"',
    ],
    link_with: [number_prefix],
)
