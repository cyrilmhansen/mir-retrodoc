fn main() {
    // Tell Cargo to rerun this build script if the schema file changes.
    println!("cargo:rerun-if-changed=schema/mircap.capnp");

    capnpc::CompilerCommand::new()
        .file("schema/mircap.capnp")
        .run()
        .expect("schema compilation failed");
}
