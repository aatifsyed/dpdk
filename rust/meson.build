test(
    'rust-test',
    executable(
        'rust-test',
        structured_sources('test.rs'),
        rust_args: [
            '--edition=2021',
            '--test'
        ],
        link_with: [bindings]
    )
)
