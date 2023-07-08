fn main() {
    protobuf_codegen::Codegen::new()
        .includes(&["schema"])
        .input("schema/tsm_common_rpc.proto")
        .input("schema/tsm_unitman_rpc.proto")
        .cargo_out_dir("protos")
        .run_from_script();
}