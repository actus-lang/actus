use std::path::{Path, PathBuf};

pub(super) fn file_uri_to_path(uri: &str) -> Option<PathBuf> {
    let remainder = uri.strip_prefix("file://")?;
    let (authority, path) = if remainder.starts_with('/') {
        (None, remainder)
    } else {
        let (authority, path) = remainder.split_once('/').unwrap_or((remainder, ""));
        (Some(authority), path)
    };
    let decoded = percent_decode(path)?;
    let value = if let Some(authority) = authority.filter(|value| !value.is_empty()) {
        let decoded_path = decoded.trim_start_matches('/');
        if authority.eq_ignore_ascii_case("localhost") {
            format!("/{decoded_path}")
        } else if authority.ends_with(':') {
            format!("/{authority}/{decoded_path}")
        } else {
            format!("//{authority}/{decoded_path}")
        }
    } else {
        decoded
    };
    #[cfg(windows)]
    let value = {
        let mut value = value.replace('/', "\\");
        if value.starts_with('\\') && value.as_bytes().get(2) == Some(&b':') {
            value.remove(0);
        }
        value
    };
    Some(PathBuf::from(value))
}

pub(super) fn path_to_file_uri(path: &Path) -> String {
    let normalized = path.to_string_lossy().replace('\\', "/");
    let uri_path = if normalized.starts_with("//") {
        normalized.trim_start_matches('/').to_owned()
    } else if normalized.as_bytes().get(1) == Some(&b':') {
        format!("/{}", normalized.trim_start_matches('/'))
    } else if normalized.starts_with('/') {
        normalized
    } else {
        format!("/{}", normalized)
    };
    format!("file://{}", encode_path(&uri_path))
}

fn percent_decode(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let high = hex_digit(*bytes.get(index + 1)?)?;
            let low = hex_digit(*bytes.get(index + 2)?)?;
            decoded.push((high << 4) | low);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(decoded).ok()
}

fn encode_path(value: &str) -> String {
    value.bytes().fold(String::new(), |mut encoded, byte| {
        if byte.is_ascii_alphanumeric() || b"-._~/".contains(&byte) || byte == b':' {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
        encoded
    })
}

fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{file_uri_to_path, path_to_file_uri};
    use std::path::Path;

    #[test]
    fn file_uri_round_trip_preserves_encoded_unicode_and_spaces() {
        let path = Path::new("/tmp/actus space-ქართული/main.act");
        let uri = path_to_file_uri(path);
        assert_eq!(
            uri,
            "file:///tmp/actus%20space-%E1%83%A5%E1%83%90%E1%83%A0%E1%83%97%E1%83%A3%E1%83%9A%E1%83%98/main.act"
        );
        assert_eq!(file_uri_to_path(&uri).as_deref(), Some(path));
    }

    #[cfg(not(windows))]
    #[test]
    fn file_uri_preserves_windows_drive_and_unc_shapes_on_unix() {
        assert_eq!(
            file_uri_to_path("file:///C:/Users/A%20B/main.act").as_deref(),
            Some(Path::new("/C:/Users/A B/main.act"))
        );
        assert_eq!(
            file_uri_to_path("file://server/share/A%20B.act").as_deref(),
            Some(Path::new("//server/share/A B.act"))
        );
    }

    #[cfg(windows)]
    #[test]
    fn file_uri_normalizes_windows_drive_and_unc_paths() {
        assert_eq!(
            file_uri_to_path("file:///C:/Users/A%20B/main.act").as_deref(),
            Some(Path::new(r"C:\Users\A B\main.act"))
        );
        assert_eq!(
            file_uri_to_path("file://server/share/A%20B.act").as_deref(),
            Some(Path::new(r"\\server\share\A B.act"))
        );
        assert_eq!(
            path_to_file_uri(Path::new(r"C:\Users\A B\main.act")),
            "file:///C:/Users/A%20B/main.act"
        );
    }
}
