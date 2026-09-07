use std::path::{Path, PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::env::set_var("PROTOC", protoc_bin_vendored::protoc_bin_path().unwrap());

    let contracts = Path::new(".contracts/proto");
    if !contracts.exists() {
        return Err(
            "\n.contracts/proto is missing. Fetch the wire contracts first:\n\n    bin/sync-contracts\n"
                .into(),
        );
    }

    println!("cargo:rerun-if-changed=.contracts/proto");

    let out_dir = PathBuf::from(std::env::var("OUT_DIR")?);
    tonic_build::configure()
        .file_descriptor_set_path(out_dir.join("pricing_descriptor.bin"))
        .compile(
            &[
                contracts.join("pricing/v1/pricing.proto").to_str().unwrap(),
            ],
            &[contracts.to_str().unwrap()],
        )?;

    return Ok(());
}
