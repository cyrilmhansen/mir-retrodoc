use crate::error::CliError;
use mircap::ModuleImage;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileFormat {
    Text,
    Binary,
}

pub fn detect_format(path: &str, format_opt: Option<&str>) -> Result<FileFormat, CliError> {
    if let Some(f) = format_opt {
        match f {
            "text" => Ok(FileFormat::Text),
            "binary" => Ok(FileFormat::Binary),
            _ => Err(CliError::Generic(format!(
                "Invalid format option: {}. Must be 'text' or 'binary'",
                f
            ))),
        }
    } else {
        if path.ends_with(".mircap.txt") {
            Ok(FileFormat::Text)
        } else if path.ends_with(".mircap") {
            Ok(FileFormat::Binary)
        } else {
            Err(CliError::Generic(format!(
                "Could not determine format for file: {}. Please use `.mircap.txt` (text), `.mircap` (binary), or specify `--format text|binary` explicitly.",
                path
            )))
        }
    }
}

pub fn load_image(path: &str, format_opt: Option<&str>) -> Result<ModuleImage, CliError> {
    let fmt = detect_format(path, format_opt)?;
    match fmt {
        FileFormat::Text => {
            let content = std::fs::read_to_string(path)?;
            let image = ModuleImage::from_text(&content)?;
            Ok(image)
        }
        FileFormat::Binary => {
            let bytes = std::fs::read(path)?;
            let image = ModuleImage::from_capnp_bytes(&bytes)?;
            Ok(image)
        }
    }
}
