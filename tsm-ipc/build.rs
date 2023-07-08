fn main() {
    protobuf_codegen::Codegen::new()
        .protoc_path(&protoc_bin_vendored::protoc_bin_path().unwrap())
        .includes(&["schema"])
        .input("schema/tsm_common_rpc.proto")
        .input("schema/tsm_unitman_rpc.proto")
        .cargo_out_dir("protos")
        .run_from_script();
}