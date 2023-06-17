fn main() -> Result<(), Box<dyn std::error::Error>> {
    capnpc::CompilerCommand::new()
        .src_prefix("resources/schemas/")
        .file("resources/schemas/tsm_unitman.capnp")
        .run()?;

    Ok(())
}