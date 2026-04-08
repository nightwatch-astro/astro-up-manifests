use crate::audit_types::FileType;

/// Minimum bytes needed for magic byte detection.
pub const MIN_DETECTION_BYTES: usize = 8;

/// Detect the file type from the first bytes of a downloaded file.
///
/// - `4D 5A` -> PE executable (further classified by installer signatures)
/// - `50 4B 03 04` -> ZIP archive
/// - `D0 CF 11 E0 A1 B1 1A E1` -> MSI (OLE Compound Document)
#[must_use]
pub fn detect_file_type(bytes: &[u8]) -> FileType {
    if bytes.len() < 2 {
        return FileType::Unknown;
    }
    if bytes.len() >= 8 && bytes[..8] == [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1] {
        return FileType::Msi;
    }
    if bytes.len() >= 4 && bytes[..4] == [0x50, 0x4B, 0x03, 0x04] {
        return FileType::Zip;
    }
    if bytes[..2] == [0x4D, 0x5A] {
        return classify_pe(bytes);
    }
    FileType::Unknown
}

/// Check if a ZIP archive contains a nested installer (exe or msi).
#[must_use]
pub fn zip_contains_installer(bytes: &[u8]) -> bool {
    let sig = [0x50, 0x4B, 0x01, 0x02];
    let mut pos = 0;
    while pos + 46 < bytes.len() {
        if let Some(offset) = find_bytes(&bytes[pos..], &sig) {
            let abs = pos + offset;
            if abs + 46 > bytes.len() {
                break;
            }
            let name_len = u16::from_le_bytes([bytes[abs + 28], bytes[abs + 29]]) as usize;
            let name_start = abs + 46;
            let name_end = name_start + name_len;
            if name_end <= bytes.len() {
                if let Ok(name_str) = std::str::from_utf8(&bytes[name_start..name_end]) {
                    let path = std::path::Path::new(name_str);
                    if let Some(ext) = path.extension() {
                        if ext.eq_ignore_ascii_case("exe") || ext.eq_ignore_ascii_case("msi") {
                            return true;
                        }
                    }
                }
            }
            pos = abs + 46 + name_len;
        } else {
            break;
        }
    }
    false
}

fn classify_pe(bytes: &[u8]) -> FileType {
    if contains_bytes(bytes, b"NullsoftInst") || contains_bytes(bytes, &[0xEF, 0xBE, 0xAD, 0xDE]) {
        return FileType::Nsis;
    }
    if contains_bytes(bytes, b"Inno Setup") {
        return FileType::InnoSetup;
    }
    FileType::PeExe
}

fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    find_bytes(haystack, needle).is_some()
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || needle.len() > haystack.len() {
        return None;
    }
    haystack.windows(needle.len()).position(|w| w == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pe_exe() {
        let mut b = vec![0x4D, 0x5A];
        b.extend_from_slice(&[0; 510]);
        assert_eq!(detect_file_type(&b), FileType::PeExe);
    }

    #[test]
    fn zip() {
        assert_eq!(detect_file_type(&[0x50, 0x4B, 0x03, 0x04, 0, 0, 0, 0]), FileType::Zip);
    }

    #[test]
    fn msi() {
        assert_eq!(detect_file_type(&[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]), FileType::Msi);
    }

    #[test]
    fn nsis_nullsoft() {
        let mut b = vec![0x4D, 0x5A];
        b.extend_from_slice(&[0; 100]);
        b.extend_from_slice(b"NullsoftInst");
        b.extend_from_slice(&[0; 100]);
        assert_eq!(detect_file_type(&b), FileType::Nsis);
    }

    #[test]
    fn nsis_magic() {
        let mut b = vec![0x4D, 0x5A];
        b.extend_from_slice(&[0; 100]);
        b.extend_from_slice(&[0xEF, 0xBE, 0xAD, 0xDE]);
        b.extend_from_slice(&[0; 100]);
        assert_eq!(detect_file_type(&b), FileType::Nsis);
    }

    #[test]
    fn inno_setup() {
        let mut b = vec![0x4D, 0x5A];
        b.extend_from_slice(&[0; 100]);
        b.extend_from_slice(b"Inno Setup Setup Data (6.3.3)");
        b.extend_from_slice(&[0; 100]);
        assert_eq!(detect_file_type(&b), FileType::InnoSetup);
    }

    #[test]
    fn unknown() {
        assert_eq!(detect_file_type(&[]), FileType::Unknown);
        assert_eq!(detect_file_type(&[0x7F, 0x45, 0x4C, 0x46]), FileType::Unknown);
    }

    #[test]
    fn zip_with_exe_inside() {
        let mut b = vec![0x50, 0x4B, 0x03, 0x04];
        b.extend_from_slice(&[0; 100]);
        b.extend_from_slice(&[0x50, 0x4B, 0x01, 0x02]);
        b.extend_from_slice(&[0; 24]);
        let name = b"setup.exe";
        #[expect(clippy::cast_possible_truncation, reason = "test fixture: name is <10 bytes")]
        b.extend_from_slice(&(name.len() as u16).to_le_bytes());
        b.extend_from_slice(&[0; 16]);
        b.extend_from_slice(name);
        b.extend_from_slice(&[0; 50]);
        assert!(zip_contains_installer(&b));
    }

    #[test]
    fn zip_without_installer() {
        let mut b = vec![0x50, 0x4B, 0x03, 0x04];
        b.extend_from_slice(&[0; 100]);
        b.extend_from_slice(&[0x50, 0x4B, 0x01, 0x02]);
        b.extend_from_slice(&[0; 24]);
        let name = b"readme.txt";
        #[expect(clippy::cast_possible_truncation, reason = "test fixture: name is <10 bytes")]
        b.extend_from_slice(&(name.len() as u16).to_le_bytes());
        b.extend_from_slice(&[0; 16]);
        b.extend_from_slice(name);
        b.extend_from_slice(&[0; 50]);
        assert!(!zip_contains_installer(&b));
    }
}
