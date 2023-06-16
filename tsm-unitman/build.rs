fn main() {
    capnpc::CompilerCommand::new()
        // .output_path("src/")
        .src_prefix("resources/schemas/")
        .file("resources/schemas/tsm_unitman.capnp")
        .run().unwrap();
}