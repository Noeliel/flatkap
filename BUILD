load("@rules_rust//rust:defs.bzl", "rust_binary")

rust_binary(
    name = "flatkap",
    crate_root = "src/main.rs",
    srcs = glob(["src/**/*.rs"]),
    deps = [
        "@crates//:libc",
        "@crates//:serde",
        "@crates//:serde_json",
    ],
)
