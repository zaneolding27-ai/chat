use std::{env, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=shaders/triangle.vert");
    println!("cargo:rerun-if-changed=shaders/triangle.frag");

    let compiler = shaderc::Compiler::new().expect("failed to create shader compiler");
    let options = shaderc::CompileOptions::new().expect("failed to create shader options");
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is not set"));

    for (source, kind, output) in [
        (
            include_str!("shaders/triangle.vert"),
            shaderc::ShaderKind::Vertex,
            "triangle.vert.spv",
        ),
        (
            include_str!("shaders/triangle.frag"),
            shaderc::ShaderKind::Fragment,
            "triangle.frag.spv",
        ),
    ] {
        let artifact = compiler
            .compile_into_spirv(source, kind, output, "main", Some(&options))
            .unwrap_or_else(|error| panic!("failed to compile {output}: {error}"));
        fs::write(out_dir.join(output), artifact.as_binary_u8())
            .unwrap_or_else(|error| panic!("failed to write {output}: {error}"));
    }
}
