#![cfg(test)]

use std::{
    collections::HashMap,
    fmt::{self, Write as _},
    process::{Command, Stdio},
};

use camino::Utf8Path;
use cargo_metadata::{CrateType, Edition, PackageId, Target, TargetKind};
use monostate::MustBe;
use petgraph::{graph::DiGraph, visit::EdgeRef as _, Direction};
use serde::Deserialize;

#[test]
fn third_party_meson_build() {
    let cmd = Command::new(env!("CARGO"))
        .args(["build"])
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../third-party-rust/"))
        .args(["-Zunstable-options", "--unit-graph"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .output()
        .unwrap();
    assert!(cmd.status.success());
    let UnitGraph {
        version: _,
        units,
        roots: _,
    } = serde_ignored::deserialize(
        &mut serde_json::Deserializer::from_slice(&cmd.stdout),
        |path| eprintln!("warn: unknown member in unit graph: {path}"),
    )
    .unwrap();

    let mut graph = DiGraph::new();
    let mut ix2ix = HashMap::new();
    for (
        ix,
        Unit {
            pkg_id: _,
            target,
            profile: _,
            platform: (),
            mode,
            features,
            dependencies: _, // handled later
        },
    ) in units.iter().enumerate()
    {
        let Target {
            name,
            kind,
            crate_types,
            src_path,
            edition,
            ..
        } = target;

        let g = match (&**kind, &**crate_types, mode) {
            ([TargetKind::Lib], [CrateType::Lib], Mode::Build) => N::Library {
                name,
                source: Utf8Path::new(src_path),
                edition: *edition,
                features,
            },
            ([TargetKind::CustomBuild], [CrateType::Bin], Mode::Build) => N::CustomBuild,
            ([TargetKind::CustomBuild], [CrateType::Bin], Mode::RunCustomBuild) => N::CustomBuild,
            unknown => todo!("{unknown:?}"),
        };

        ix2ix.insert(ix, graph.add_node(g));
    }

    for (to, unit) in units.iter().enumerate() {
        for from in &unit.dependencies {
            graph.add_edge(
                ix2ix[&from.index],
                ix2ix[&to],
                E {
                    extern_crate_name: &from.extern_crate_name,
                },
            );
        }
    }

    let mut invocations = vec![];

    for ix in petgraph::algo::toposort(&graph, None).unwrap() {
        let N::Library {
            name,
            source,
            edition,
            features,
        } = graph[ix]
        else {
            continue;
        };
        let mut link_with = vec![];
        let mut has_build_script = false;

        for edge in graph.edges_directed(ix, Direction::Incoming) {
            let E { extern_crate_name } = edge.weight();
            let N::Library { name: depends, .. } = graph[edge.source()] else {
                has_build_script = true;
                continue;
            };
            assert_eq!(
                depends, *extern_crate_name,
                "crate renaming is not supported"
            );
            link_with.push(depends);
        }
        invocations.push(Invocation {
            name,
            source,
            edition,
            features,
            link_with,
            has_build_script,
        });
    }

    let expected = invocations.iter().fold(String::new(), |mut acc, el| {
        writeln!(acc, "{el}").unwrap();
        acc
    });
    expect_test::expect_file!["../../third-party-rust/meson.build"].assert_eq(&expected);
}

struct Invocation<'a> {
    name: &'a str,
    source: &'a Utf8Path,
    edition: Edition,
    features: &'a [String],
    link_with: Vec<&'a str>,
    has_build_script: bool,
}

impl fmt::Display for Invocation<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            name,
            source,
            edition,
            features,
            link_with,
            has_build_script,
        } = self;

        writeln!(f, "rust3p_{name} = library(")?;
        writeln!(f, "    '{name}',")?;
        if *has_build_script {
            writeln!(f, "    # ignoring build script")?;
        }
        writeln!(f, "    sources: ['{source}'],")?;
        writeln!(f, "    rust_args: [")?;
        writeln!(f, "        '--edition={edition}',")?;
        for feature in *features {
            writeln!(f, "        '--cfg', 'feature=\"{feature}\"',")?; // putting this in one string hits a meson bug
        }
        writeln!(f, "    ],")?; // rust_args
        if !link_with.is_empty() {
            writeln!(f, "    link_with: [")?;
            for link in link_with {
                writeln!(f, "        rust3p_{link},")?;
            }
            writeln!(f, "    ],")?; // link_with
        }
        writeln!(f, ")")?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
enum N<'a> {
    Library {
        name: &'a str,
        source: &'a Utf8Path,
        edition: Edition,
        features: &'a [String],
    },
    CustomBuild,
}
#[derive(Clone, Copy, Debug)]
struct E<'a> {
    extern_crate_name: &'a str,
}

#[derive(Debug, Deserialize)]
struct UnitGraph {
    #[allow(unused)]
    version: MustBe!(1),
    units: Vec<Unit>,
    #[allow(unused)]
    roots: Vec<usize>,
}

#[derive(Debug, Deserialize)]
struct Unit {
    #[allow(unused)]
    pkg_id: PackageId,
    target: Target,
    #[allow(unused)]
    profile: serde_json::Value,
    platform: (),
    mode: Mode,
    features: Vec<String>,
    dependencies: Vec<Dependency>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Mode {
    Build,
    RunCustomBuild,
}

#[derive(Debug, Deserialize)]
struct Dependency {
    index: usize,
    extern_crate_name: String,
    #[allow(unused)]
    public: bool,
    #[allow(unused)]
    noprelude: bool,
}
