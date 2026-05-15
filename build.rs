use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let assets_dir = manifest_dir.join("assets");

    let mut files = Vec::new();
    collect_asset_files(&assets_dir, &assets_dir, &mut files)
        .expect("failed to collect asset files");
    files.sort_by(|(left, _), (right, _)| left.cmp(right));

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let generated_path = out_dir.join("embedded_assets.rs");
    fs::write(generated_path, generate_assets_module(files))
        .expect("failed to write embedded assets");
}

fn collect_asset_files(
    root: &Path,
    directory: &Path,
    files: &mut Vec<(String, PathBuf)>,
) -> io::Result<()> {
    println!("cargo:rerun-if-changed={}", directory.display());

    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;

        if file_type.is_dir() {
            collect_asset_files(root, &path, files)?;
        } else if file_type.is_file() {
            println!("cargo:rerun-if-changed={}", path.display());
            files.push((asset_path(root, &path), path));
        }
    }

    Ok(())
}

fn asset_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .expect("asset path should be under assets root")
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

fn generate_assets_module(files: Vec<(String, PathBuf)>) -> String {
    let mut output = String::from(
        "pub struct EmbeddedAsset {\n    pub path: &'static str,\n    pub content: &'static [u8],\n}\n\npub const ASSETS: &[EmbeddedAsset] = &[\n",
    );

    for (asset_path, file_path) in files {
        let file_path = file_path
            .canonicalize()
            .expect("failed to canonicalize asset path");
        output.push_str(&format!(
            "    EmbeddedAsset {{ path: {asset_path:?}, content: include_bytes!({file_path:?}) }},\n",
            file_path = file_path.to_string_lossy(),
        ));
    }

    output.push_str("];\n");
    output
}
