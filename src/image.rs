use std::collections::HashMap;

use base64::Engine;

#[derive(Debug, Clone, PartialEq)]
pub enum ImageFormat {
    Png,
    Jpeg,
    Gif,
    Webp,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ImageSource {
    DataUri(Vec<u8>, ImageFormat),
    Remote(String),
    Cid(String),
    LocalPath(String),
    Invalid,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImageData {
    pub bytes: Vec<u8>,
    pub format: ImageFormat,
}

pub fn parse_source(src: &str) -> ImageSource {
    let src = src.trim();
    if src.starts_with("data:") {
        return resolve_data_uri(src).unwrap_or(ImageSource::Invalid);
    }
    if src.starts_with("cid:") {
        return ImageSource::Cid(src.trim_start_matches("cid:").to_string());
    }
    if src.starts_with("http://") || src.starts_with("https://") {
        return ImageSource::Remote(src.to_string());
    }
    if !src.is_empty() {
        return ImageSource::LocalPath(src.to_string());
    }
    ImageSource::Invalid
}

pub fn resolve_image(src: &str, mime_parts: &HashMap<String, Vec<u8>>) -> Option<ImageData> {
    match parse_source(src) {
        ImageSource::DataUri(bytes, format) => Some(ImageData { bytes, format }),
        ImageSource::Cid(id) => mime_parts.get(&id).map(|bytes| ImageData {
            bytes: bytes.clone(),
            format: detect_image_format(bytes),
        }),
        ImageSource::LocalPath(path) => std::fs::read(path).ok().map(|bytes| ImageData {
            format: detect_image_format(&bytes),
            bytes,
        }),
        ImageSource::Remote(_) | ImageSource::Invalid => None,
    }
}

pub fn source_dimensions(source: &ImageSource) -> Option<(u32, u32)> {
    match source {
        ImageSource::DataUri(bytes, _) => image::load_from_memory(bytes)
            .ok()
            .map(|img| (img.width(), img.height())),
        ImageSource::LocalPath(path) => image::image_dimensions(path).ok(),
        _ => None,
    }
}

fn resolve_data_uri(src: &str) -> Option<ImageSource> {
    let payload = src.strip_prefix("data:")?;
    let (meta, data) = payload.split_once(',')?;
    if !meta.contains(";base64") {
        return None;
    }
    let mime = meta.split(';').next().unwrap_or_default();
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data)
        .ok()?;
    Some(ImageSource::DataUri(bytes, format_from_mime(mime)))
}

fn format_from_mime(mime: &str) -> ImageFormat {
    match mime {
        "image/png" => ImageFormat::Png,
        "image/jpeg" | "image/jpg" => ImageFormat::Jpeg,
        "image/gif" => ImageFormat::Gif,
        "image/webp" => ImageFormat::Webp,
        _ => ImageFormat::Unknown,
    }
}

fn detect_image_format(bytes: &[u8]) -> ImageFormat {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        ImageFormat::Png
    } else if bytes.starts_with(&[0xFF, 0xD8]) {
        ImageFormat::Jpeg
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        ImageFormat::Gif
    } else if bytes.starts_with(b"RIFF") && bytes.len() >= 12 && &bytes[8..12] == b"WEBP" {
        ImageFormat::Webp
    } else {
        ImageFormat::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::{ImageSource, parse_source, resolve_image};
    use std::collections::HashMap;

    #[test]
    fn parses_data_uri_source() {
        let src = "data:image/png;base64,aGVsbG8=";
        let parsed = parse_source(src);
        match parsed {
            ImageSource::DataUri(bytes, _) => assert_eq!(bytes, b"hello"),
            _ => panic!("expected data uri"),
        }
    }

    #[test]
    fn resolves_cid_from_map() {
        let mut map = HashMap::new();
        map.insert("logo".to_string(), vec![0x89, b'P', b'N', b'G']);
        let data = resolve_image("cid:logo", &map).expect("cid image");
        assert_eq!(data.bytes.len(), 4);
    }
}
