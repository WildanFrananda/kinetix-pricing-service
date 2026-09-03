use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::env::set_var("PROTOC", protoc_bin_vendored::protoc_bin_path().unwrap());

    let out_dir = PathBuf::from(std::env::var("OUT_DIR")?);
    tonic_build::configure()
        .file_descriptor_set_path(out_dir.join("pricing_descriptor.bin"))
        .compile(&["proto/pricing/v1/pricing_service.proto"], &["proto"])?;

    tonic_build::compile_protos("proto/shipping/v1/shipping_service.proto")?;

    return Ok(());
}
