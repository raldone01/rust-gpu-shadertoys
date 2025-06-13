use std::{
  error::Error,
  fs::{self, File},
  io::{self, Write},
  path::Path,
};

use example_shaders::shaders::SHADER_DEFINITIONS;
use flate2::Compression;
use spirv_builder::{CompileResult, MetadataPrintout, SpirvBuilder, SpirvBuilderError};

fn build_shader(path_to_crate: &str, out_dir: &str) -> Result<CompileResult, SpirvBuilderError> {
  let builder = SpirvBuilder::new(path_to_crate, "spirv-unknown-vulkan1.2")
    // Maybe?
    //.preserve_bindings(true)
    .print_metadata(MetadataPrintout::DependencyOnly)
    // Actually enable the shader code
    .shader_crate_features(["shader_code"].into_iter().map(String::from))
    // We want separate files for each shader
    .multimodule(true);

  builder.build()
}

fn export_shader_definitions(out_dir: &str) -> Result<(), Box<dyn Error>> {
  for shader_definition in SHADER_DEFINITIONS {
    let file_name = format!("{}/{}.json", out_dir, shader_definition.name);
    let file_content = serde_json::to_string_pretty(shader_definition)?;
    std::fs::write(file_name, file_content)?;
  }
  Ok(())
}

/// https://stackoverflow.com/a/65192210/4479969
fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
  fs::create_dir_all(&dst)?;
  for entry in fs::read_dir(src)? {
    let entry = entry?;
    let ty = entry.file_type()?;
    if ty.is_dir() {
      copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
    } else {
      fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
    }
  }
  Ok(())
}

#[must_use]
fn module_result_to_paths(module_result: &spirv_builder::ModuleResult) -> Vec<String> {
  match module_result {
    spirv_builder::ModuleResult::SingleModule(path) => vec![path.to_string_lossy().to_string()],
    spirv_builder::ModuleResult::MultiModule(map) => map
      .values()
      .map(|p| p.to_string_lossy().to_string())
      .collect(),
  }
}

fn main() -> Result<(), Box<dyn Error>> {
  let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR environment variable not set");
  let path_to_crate = "example_shaders";
  export_shader_definitions(&out_dir)?;
  // QUESTION: We ignore the result here not to crash rust-analyzer I assume?
  let result = build_shader(path_to_crate, &out_dir)?;
  let module_paths = module_result_to_paths(&result.module);
  // copy the shader files to the OUT_DIR
  for module_path in &module_paths {
    let src_path = Path::new(module_path);
    let dst_path = Path::new(&out_dir).join(
      src_path
        .file_name()
        .expect("Module path should have a file name"),
    );
    std::fs::copy(src_path, dst_path)?;
  }

  let inspection_dir = Path::new(
    &std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR environment variable not set"),
  )
  .join("target")
  .join("shader_inspection");
  // try to delete the inspection directory if it exists
  if inspection_dir.exists() {
    std::fs::remove_dir_all(&inspection_dir)?;
  }
  if !inspection_dir.exists() {
    std::fs::create_dir_all(&inspection_dir)?;
  }

  // Copy the out_dir contents to the inspection directory
  copy_dir_all(&out_dir, &inspection_dir)?;
  // Write debug of CompileResult to the inspection directory
  let debug_file_path = inspection_dir.join("compile_result_debug.txt");
  let mut debug_file = std::fs::File::create(debug_file_path)?;
  writeln!(debug_file, "Compile Result: {:#?}", result)?;
  writeln!(debug_file, "Module Paths: {:?}", &module_paths)?;

  Ok(())
}
