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
seasick = library(
    'seasick',
    sources: ['.vendor/seasick/src/lib.rs'],
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
)
