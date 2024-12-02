rust_trybox = library(
    'trybox',
    sources: ['/home/aatif/code/dpdk/third-party-rust/.vendor/trybox/src/lib.rs'],
    rust_args: [
        '--edition=2021',
        '--cfg', 'feature="default"',
        '--cfg', 'feature="std"',
    ],
)

rust_minimal_lexical = library(
    'minimal_lexical',
    sources: ['/home/aatif/code/dpdk/third-party-rust/.vendor/minimal-lexical/src/lib.rs'],
    rust_args: [
        '--edition=2018',
        '--cfg', 'feature="std"',
    ],
)

rust_memchr = library(
    'memchr',
    sources: ['/home/aatif/code/dpdk/third-party-rust/.vendor/memchr/src/lib.rs'],
    rust_args: [
        '--edition=2021',
        '--cfg', 'feature="alloc"',
        '--cfg', 'feature="std"',
    ],
)

rust_nom = library(
    'nom',
    sources: ['/home/aatif/code/dpdk/third-party-rust/.vendor/nom/src/lib.rs'],
    rust_args: [
        '--edition=2018',
        '--cfg', 'feature="alloc"',
        '--cfg', 'feature="default"',
        '--cfg', 'feature="std"',
    ],
    link_with: [
        rust_minimal_lexical,
        rust_memchr,
    ],
)

rust_libc = library(
    'libc',
    # ignoring build script
    sources: ['/home/aatif/code/dpdk/third-party-rust/.vendor/libc/src/lib.rs'],
    rust_args: [
        '--edition=2015',
        '--cfg', 'feature="std"',
    ],
)

rust_seasick = library(
    'seasick',
    sources: ['/home/aatif/code/dpdk/third-party-rust/.vendor/seasick/src/lib.rs'],
    rust_args: [
        '--edition=2021',
        '--cfg', 'feature="alloc"',
        '--cfg', 'feature="default"',
        '--cfg', 'feature="libc"',
        '--cfg', 'feature="std"',
    ],
    link_with: [
        rust_libc,
    ],
)

rust_third_party_rust = library(
    'third_party_rust',
    sources: ['/dev/null'],
    rust_args: [
        '--edition=2021',
    ],
    link_with: [
        rust_trybox,
        rust_seasick,
        rust_nom,
    ],
)

