rust3p_trybox = library(
    'trybox',
    sources: ['vendor/trybox/src/lib.rs'],
    rust_args: [
        '--edition=2021',
        '--cfg', 'feature="default"',
        '--cfg', 'feature="std"',
    ],
)

rust3p_minimal_lexical = library(
    'minimal_lexical',
    sources: ['vendor/minimal-lexical/src/lib.rs'],
    rust_args: [
        '--edition=2018',
        '--cfg', 'feature="std"',
    ],
)

rust3p_memchr = library(
    'memchr',
    sources: ['vendor/memchr/src/lib.rs'],
    rust_args: [
        '--edition=2021',
        '--cfg', 'feature="alloc"',
        '--cfg', 'feature="std"',
    ],
)

rust3p_nom = library(
    'nom',
    sources: ['vendor/nom/src/lib.rs'],
    rust_args: [
        '--edition=2018',
        '--cfg', 'feature="alloc"',
        '--cfg', 'feature="default"',
        '--cfg', 'feature="std"',
    ],
    link_with: [
        rust3p_minimal_lexical,
        rust3p_memchr,
    ],
)

rust3p_libc = library(
    'libc',
    # ignoring build script
    sources: ['vendor/libc/src/lib.rs'],
    rust_args: [
        '--edition=2015',
        '--cfg', 'feature="std"',
    ],
)

rust3p_seasick = library(
    'seasick',
    sources: ['vendor/seasick/src/lib.rs'],
    rust_args: [
        '--edition=2021',
        '--cfg', 'feature="alloc"',
        '--cfg', 'feature="default"',
        '--cfg', 'feature="libc"',
        '--cfg', 'feature="std"',
    ],
    link_with: [
        rust3p_libc,
    ],
)

rust3p_rust3p = library(
    'rust3p',
    sources: ['empty.rs'],
    rust_args: [
        '--edition=2021',
    ],
    link_with: [
        rust3p_trybox,
        rust3p_seasick,
        rust3p_nom,
    ],
)

