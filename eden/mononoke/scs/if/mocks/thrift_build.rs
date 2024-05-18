// @generated by autocargo

use std::env;
use std::fs;
use std::path::Path;
use thrift_compiler::Config;
use thrift_compiler::GenContext;
const CRATEMAP: &str = "\
derived_data_type derived_data_type_if //eden/mononoke/derived_data/if:derived_data_type_if-rust
fb303_core fb303_core //fb303/thrift:fb303_core-rust
megarepo_configs megarepo_configs //configerator/structs/scm/mononoke/megarepo:megarepo_configs-rust
source_control crate //eden/mononoke/scs/if:source_control-rust
thrift thrift //thrift/annotation:thrift-rust
";
#[rustfmt::skip]
fn main() {
    println!("cargo:rerun-if-changed=thrift_build.rs");
    let out_dir = env::var_os("OUT_DIR").expect("OUT_DIR env not provided");
    let cratemap_path = Path::new(&out_dir).join("cratemap");
    fs::write(cratemap_path, CRATEMAP).expect("Failed to write cratemap");
    let mut conf = Config::from_env(GenContext::Mocks)
        .expect("Failed to instantiate thrift_compiler::Config");
    let cargo_manifest_dir = env::var_os("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR not provided");
    let mut base_path = Path::new(&cargo_manifest_dir)
        .join("../../../../..")
        .canonicalize()
        .expect("Failed to canonicalize base_path");
    if cfg!(windows) {
        base_path = base_path.to_string_lossy().trim_start_matches(r"\\?\").into();
    }
    conf.base_path(base_path);
    conf.types_crate("source_control__types");
    conf.clients_crate("source_control__clients");
    conf.options("deprecated_default_enum_min_i32,serde");
    let srcs: &[&str] = &["../source_control.thrift"];
    conf.run(srcs).expect("Failed while running thrift compilation");
}
