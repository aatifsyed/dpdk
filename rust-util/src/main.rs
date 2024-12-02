use std::{collections::BTreeMap, convert::Infallible, io, iter, str::FromStr};

use anyhow::{bail, Context};
use camino::Utf8PathBuf;
use cargo_metadata::{semver::Version, CrateType, Edition, TargetKind};
use clap::Parser;
use monostate::MustBe;
use serde::{de::IgnoredAny, Deserialize, Deserializer};

fn main() {
    let build_plan = serde_ignored::deserialize::<_, _, BuildPlan>(
        &mut serde_json::Deserializer::from_reader(io::stdin()),
        |it| println!("unknown key: {it}"),
    )
    .unwrap();
    for invocation in build_plan.invocations {}
}

#[derive(Deserialize)]
struct BuildPlan {
    invocations: Vec<Invocation>,
    inputs: Vec<Utf8PathBuf>,
}

#[derive(Deserialize, Debug)]
struct Invocation {
    package_name: String,
    package_version: Version,
    target_kind: Vec<TargetKind>,
    kind: (),
    #[serde(flatten)]
    compile_mode: Compile,
    deps: Vec<usize>,
    outputs: Vec<Utf8PathBuf>,
    links: BTreeMap<Utf8PathBuf, Utf8PathBuf>,
    program: Utf8PathBuf,
    env: BTreeMap<String, String>,
    cwd: Utf8PathBuf,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "compile_mode", content = "args", rename_all = "kebab-case")]
enum Compile {
    Build(Build),
    RunCustomBuild(EmptySequence),
}

#[derive(Debug)]
struct EmptySequence;

impl<'de> Deserialize<'de> for EmptySequence {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = Vec::<IgnoredAny>::deserialize(d)?;
        match v.is_empty() {
            true => Ok(Self),
            false => Err(serde::de::Error::invalid_length(v.len(), &"0")),
        }
    }
}

#[derive(Parser, Debug)]
struct Build {
    #[arg(long)]
    crate_name: String,
    #[arg(long)]
    edition: _Edition,
    source: Utf8PathBuf,
    #[arg(long)]
    error_format: Ignored,
    #[arg(long)]
    json: Ignored,
    #[arg(long)]
    diagnostic_width: Ignored,
    #[arg(long)]
    crate_type: _CrateType,
    #[arg(long)]
    emit: Ignored,
    #[arg(long)]
    cfg: Vec<String>,
    #[arg(long)]
    check_cfg: Vec<String>,
    #[arg(short = 'C')]
    codegen: Vec<KeyValue>,
    #[arg(long)]
    out_dir: Utf8PathBuf,
    #[arg(short = 'L')]
    library_search_path: Vec<Utf8PathBuf>,
    #[arg(long)]
    cap_lints: Option<Ignored>,
    #[arg(long)]
    r#extern: Vec<KeyValue>,
}

impl<'de> Deserialize<'de> for Build {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Self::try_parse_from(
            iter::once(String::from("<rustc>")).chain(Vec::<String>::deserialize(d)?),
        )
        .map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone)]
struct KeyValue {
    key: String,
    value: String,
}

impl FromStr for KeyValue {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (key, value) = s.split_once('=').context("must have an `=`")?;
        Ok(Self {
            key: key.into(),
            value: value.into(),
        })
    }
}

#[derive(Debug, Clone)]
struct Ignored;

impl FromStr for Ignored {
    type Err = Infallible;
    fn from_str(_: &str) -> Result<Self, Self::Err> {
        Ok(Self)
    }
}

#[derive(Debug, Clone)]
struct _Edition(Edition);

impl FromStr for _Edition {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "2015" => Self(Edition::E2015),
            "2018" => Self(Edition::E2018),
            "2021" => Self(Edition::E2021),
            _ => bail!("unknown edition"),
        })
    }
}

#[derive(Debug, Clone)]
struct _CrateType(CrateType);

impl FromStr for _CrateType {
    type Err = Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.into()))
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
struct _Codegen {
    embed_bitcode: Option<String>,
    metadata: String,
    extra_filename: String,
    linker: String,
}
struct Codegen {
    embed_bitcode: Option<String>,
    metadata: String,
    extra_filename: String,
    linker: String,
    link_args: Vec<String>,
}
