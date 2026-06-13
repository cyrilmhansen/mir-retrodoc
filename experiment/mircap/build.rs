fn main() {
    println!("cargo:rerun-if-changed=schema/mircap.capnp");

    capnpc::CompilerCommand::new()
        .file("schema/mircap.capnp")
        .run()
        .unwrap_or_else(|err| {
            panic!(
                "Failed to compile Cap'n Proto schema. \
                 Make sure the `capnp` executable is installed. \
                 On Ubuntu: sudo apt-get install capnproto. \
                 Original error: {err:?}"
            );
        });
}
