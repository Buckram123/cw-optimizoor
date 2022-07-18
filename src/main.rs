use std::{fs, path::PathBuf};

use anyhow::Result;
use cargo::{core::Workspace, ops};
use clap::Parser;

use path_absolutize::Absolutize;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use cw_optimizoor::{compilation::*, ext::*, hashing::*, optimization::*};

/// cw-optimizoor
#[derive(Debug, Parser)]
#[clap(name = "cargo")]
#[clap(bin_name = "cargo")]
#[clap(about = "CosmWasm optimizer", long_about = None)]
enum Cargo {
    CwOptimizoor(CwOptimizoor),
}

#[derive(clap::Args, Debug)]
#[clap(author, version, about, long_about = None)]
struct CwOptimizoor {
    /// Path to the Cargo.toml
    #[clap(value_parser)]
    manifest_path: Option<std::path::PathBuf>,
}

fn main() -> Result<()> {
    let Cargo::CwOptimizoor(args) = Cargo::parse();

    let manifest_path = args
        .manifest_path
        .expect("missing manifest path")
        .absolutize()?
        .to_path_buf();

    let cfg = config()?;
    let ws = Workspace::new(manifest_path.as_ref(), &cfg).expect("couldn't create workspace");
    let compile_opts = compile_opts(&cfg, Some(ops::Packages::Default))?;
    let output_dir = create_artifacts_dir(&ws)?;

    println!("🧐️  Compiling .../{}", manifest_path.rtake(2).display());
    let intermediate_wasm_paths = compile(&compile_opts, &ws)?;
    println!("🤓  Intermediate checksums:");
    write_checksums(
        &intermediate_wasm_paths,
        &output_dir.join("checksums_intermediate.txt"),
    )?;

    println!("🥸  Ahh I'm optimiziing:");
    let final_wasm_paths = intermediate_wasm_paths
        .par_iter()
        .map(|wasm_path| {
            let output_path = optimized_output_path(wasm_path, &output_dir)?;
            optimize(wasm_path, &output_path)?;

            println!("    .../{}", wasm_path.rtake(5).display());
            anyhow::Ok(output_path)
        })
        .collect::<Result<Vec<PathBuf>>>()?;
    println!("🤓  Final checksums:");
    write_checksums(&final_wasm_paths, &output_dir.join("checksums.txt"))?;

    println!(
        "🫡  Done. Saved optimized artifacts to:\n   {}/artifacts",
        ws.root().display()
    );

    Ok(())
}

/// Creates the artifacts dir if it doesn't exist.
fn create_artifacts_dir(ws: &Workspace) -> Result<PathBuf> {
    let output_dir = ws.root().absolutize()?.to_path_buf().join("artifacts");
    fs::create_dir_all(&output_dir)?;

    Ok(output_dir)
}
