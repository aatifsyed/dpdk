fn main() {
    let includes = [
        "lib/eal/include/",
        "lib/eal/common/",
        "lib/eal/linux/include/",
        "config/",
        "prefix/include/",
        "lib/ethdev/",
        "drivers/bus/vdev/",
    ];
    bindgen::builder()
        .header("drivers/bus/vdev/bus_vdev_driver.h")
        .header("drivers/net/pcap/pcap_osdep.h")
        .clang_args(includes.iter().map(|it| format!("-I{it}")))
        .layout_tests(false)
        .allowlist_item("rte_.*")
        .allowlist_item("osdep.*")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .unwrap()
        .write_to_file("bindings.rs")
        .unwrap();

    println!("cargo::rustc-link-search=native=prefix/lib/x86_64-linux-gnu/");
    println!("cargo::rustc-link-lib=static=rte_bus_vdev");

    // println!("cargo:rustc-link-arg=-Wl,--whole-archive"); // __attribute__((constructor))
    // cc::Build::new()
    //     .files(rr([
    //         "drivers/bus/vdev/vdev_params.c",
    //         "drivers/bus/vdev/vdev.c",
    //     ]))
    //     .files(rr(["drivers/net/null/rte_eth_null.c"]))
    //     .files(rr([
    //         "drivers/net/pcap/pcap_ethdev.c",
    //         "drivers/net/pcap/pcap_osdep_linux.c",
    //     ]))
    //     .files(rr([
    //         "lib/log/log_color.c",
    //         "lib/log/log_journal.c",
    //         "lib/log/log_syslog.c",
    //         "lib/log/log_timestamp.c",
    //         "lib/log/log.c",
    //     ]))
    //     .files(rr([
    //         "lib/eal/common/eal_common_bus.c",
    //         "lib/eal/common/eal_common_config.c",
    //         "lib/eal/common/eal_common_debug.c",
    //         "lib/eal/common/eal_common_dev.c",
    //         "lib/eal/common/eal_common_devargs.c",
    //         "lib/eal/common/eal_common_errno.c",
    //         "lib/eal/common/eal_common_proc.c",
    //         "lib/eal/common/eal_common_thread.c",
    //         "lib/eal/linux/eal_thread.c",
    //         "lib/eal/unix/eal_debug.c",
    //     ]))
    //     .files(rr(["lib/kvargs/rte_kvargs.c"]))
    //     .flag("-Wno-format-truncation") // eek!
    //     .flag("-mssse3")
    //     .define("ALLOW_INTERNAL_API", "1") // __rte_internal
    //     .define("ALLOW_EXPERIMENTAL_API", "1") // __rte_internal
    //     .define("_GNU_SOURCE", "1")
    //     .shared_flag(true) // fopencookie
    //     .includes(includes)
    //     .compile("deepeedeekay");
}

fn rr<const N: usize>(f: [&str; N]) -> [&str; N] {
    f.map(|it| {
        println!("cargo::rerun-if-changed={it}");
        it
    })
}
