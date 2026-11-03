use axmldecoder::{Node, XmlDocument};
use sha2::{Digest, Sha256};
use std::io::{Cursor, Read};
use thiserror::Error;
use zip::ZipArchive;

#[derive(Debug, Clone, Default)]
pub struct ApkMetadata {
    pub package_name: Option<String>,
    pub version_name: Option<String>,
    pub version_code: Option<u32>,
    pub min_sdk: Option<u32>,
    pub target_sdk: Option<u32>,
    pub abis: Vec<String>,
    pub signing_fingerprint: Option<String>,
}

#[derive(Debug, Error)]
pub enum ApkError {
    #[error("invalid ZIP archive: {0}")]
    Zip(#[from] zip::result::ZipError),
}

const KNOWN_ABIS: &[&str] = &["arm64-v8a", "armeabi-v7a", "x86_64", "x86"];

pub fn extract_metadata(data: &[u8]) -> Result<ApkMetadata, ApkError> {
    let cursor = Cursor::new(data);
    let mut archive = ZipArchive::new(cursor)?;

    let mut metadata = ApkMetadata::default();

    // Try to extract manifest metadata; failures are non-fatal
    match read_manifest(&mut archive) {
        Ok(manifest_bytes) => match axmldecoder::parse(&manifest_bytes) {
            Ok(doc) => {
                extract_manifest_fields(&doc, &mut metadata);
            }
            Err(e) => {
                tracing::warn!(error = ?e, "failed to decode AndroidManifest.xml binary XML, continuing with partial metadata");
            }
        },
        Err(msg) => {
            tracing::warn!(reason = %msg, "could not read AndroidManifest.xml from APK, continuing with partial metadata");
        }
    }

    // ABI detection always works (just checking zip entries)
    metadata.abis = detect_abis(&archive);

    // Signing fingerprint extraction; failure is non-fatal
    metadata.signing_fingerprint = extract_signing_fingerprint(&mut archive);

    Ok(metadata)
}

pub fn compute_sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

fn read_manifest(archive: &mut ZipArchive<Cursor<&[u8]>>) -> Result<Vec<u8>, String> {
    let mut manifest = archive
        .by_name("AndroidManifest.xml")
        .map_err(|_| "AndroidManifest.xml not found in APK")?;
    let mut buf = Vec::new();
    manifest
        .read_to_end(&mut buf)
        .map_err(|e| format!("failed to read manifest: {e}"))?;
    Ok(buf)
}

fn extract_manifest_fields(doc: &XmlDocument, metadata: &mut ApkMetadata) {
    let root = match doc.get_root() {
        Some(Node::Element(e)) => e,
        _ => {
            tracing::warn!("AndroidManifest.xml has no root element");
            return;
        }
    };

    metadata.package_name = find_attr(root, "package");
    metadata.version_code = find_attr(root, "versionCode").and_then(|v| parse_manifest_int(&v));
    // Resource references like @0x... are not usable as version names
    metadata.version_name = find_attr(root, "versionName").filter(|v| !v.starts_with('@'));

    // Find <uses-sdk> child for minSdkVersion / targetSdkVersion
    let (min_sdk, target_sdk) = extract_sdk_versions(root);
    metadata.min_sdk = min_sdk;
    metadata.target_sdk = target_sdk;
}

/// Search for an attribute by short name (e.g. "package") or prefixed (e.g. "android:versionCode").
fn find_attr(element: &axmldecoder::Element, short_name: &str) -> Option<String> {
    let attrs = element.get_attributes();
    // Try exact match first
    if let Some(v) = attrs.get(short_name)
        && !v.is_empty()
    {
        return Some(v.clone());
    }
    // Try with android: prefix
    let prefixed = format!("android:{short_name}");
    if let Some(v) = attrs.get(&prefixed)
        && !v.is_empty()
    {
        return Some(v.clone());
    }
    // Fallback: search for any key ending with the short name (handles namespace URIs)
    for (k, v) in attrs {
        if (k.ends_with(&format!(":{short_name}")) || k == short_name) && !v.is_empty() {
            return Some(v.clone());
        }
    }
    None
}

fn extract_sdk_versions(root: &axmldecoder::Element) -> (Option<u32>, Option<u32>) {
    for child in root.get_children() {
        if let Node::Element(el) = child
            && el.get_tag() == "uses-sdk"
        {
            let min = find_attr(el, "minSdkVersion").and_then(|v| parse_manifest_int(&v));
            let target = find_attr(el, "targetSdkVersion").and_then(|v| parse_manifest_int(&v));
            return (min, target);
        }
    }
    (None, None)
}

/// Parse an integer from a manifest attribute value.
/// Handles decimal, hex (0x...), and rejects resource references (@0x...).
fn parse_manifest_int(s: &str) -> Option<u32> {
    // Resource references are not parseable integers
    if s.starts_with('@') {
        return None;
    }
    if let Some(hex) = s.strip_prefix("0x") {
        u32::from_str_radix(hex, 16).ok()
    } else {
        s.parse::<u32>().ok()
    }
}

/// Extract the signing certificate fingerprint from META-INF/*.RSA or *.DSA.
///
/// Attempts to extract the actual DER-encoded X.509 certificate from the PKCS#7
/// structure. If that fails, falls back to hashing the entire signing block file,
/// which is still deterministic per signing key.
fn extract_signing_fingerprint(archive: &mut ZipArchive<Cursor<&[u8]>>) -> Option<String> {
    // Find the first META-INF/*.RSA or *.DSA entry
    let signing_entry_name = archive
        .file_names()
        .filter_map(Result::ok)
        .find(|name| {
            let upper = name.to_uppercase();
            (name.starts_with("META-INF/") || name.starts_with("META-INF\\"))
                && (upper.ends_with(".RSA") || upper.ends_with(".DSA"))
        })?
        .into_owned();

    let mut entry = archive.by_name(&signing_entry_name).ok()?;
    let mut buf = Vec::new();
    if let Err(e) = entry.read_to_end(&mut buf) {
        tracing::warn!(
            entry = %signing_entry_name,
            error = %e,
            "failed to read signing block entry"
        );
        return None;
    }

    if buf.is_empty() {
        return None;
    }

    // Try to extract the first X.509 certificate from the PKCS#7 DER structure.
    // If that fails, fall back to hashing the entire signing block file.
    let cert_der = extract_first_cert_from_pkcs7(&buf);

    if cert_der.is_none() {
        tracing::debug!(
            entry = %signing_entry_name,
            "could not extract X.509 cert from PKCS#7, falling back to full-file hash"
        );
    }

    let hash_input = cert_der.as_deref().unwrap_or(&buf);
    let mut hasher = Sha256::new();
    hasher.update(hash_input);
    let digest = hasher.finalize();
    Some(hex::encode(digest))
}

/// Attempt to pull the first X.509 certificate out of a DER-encoded PKCS#7
/// SignedData blob using minimal ASN.1 parsing (no external crate needed).
fn extract_first_cert_from_pkcs7(data: &[u8]) -> Option<Vec<u8>> {
    let mut pos = 0;

    // Outer SEQUENCE
    let (_, _end) = read_tlv(data, pos)?;

    // Skip the SEQUENCE tag+length to get inside
    pos = skip_tag_and_length(data, pos)?;

    // OID for signedData
    let (_oid_content, oid_end) = read_tlv(data, pos)?;
    pos = oid_end;

    // [0] EXPLICIT wrapper
    if pos >= data.len() || (data[pos] & 0xE0) != 0xA0 {
        return None;
    }
    pos = skip_tag_and_length(data, pos)?;

    // Inner SEQUENCE (SignedData)
    pos = skip_tag_and_length(data, pos)?;

    // INTEGER (version)
    let (_, int_end) = read_tlv(data, pos)?;
    pos = int_end;

    // SET (digestAlgorithms)
    let (_, set_end) = read_tlv(data, pos)?;
    pos = set_end;

    // SEQUENCE (contentInfo)
    let (_, ci_end) = read_tlv(data, pos)?;
    pos = ci_end;

    // Now we expect [0] IMPLICIT (certificates) - tag 0xA0
    if pos >= data.len() || data[pos] != 0xA0 {
        return None;
    }
    pos = skip_tag_and_length(data, pos)?;

    // First element inside should be a SEQUENCE (the X.509 certificate)
    if pos >= data.len() || data[pos] != 0x30 {
        return None;
    }

    let (_cert_content, cert_end) = read_tlv(data, pos)?;
    // The full DER certificate is from pos..cert_end
    Some(data[pos..cert_end].to_vec())
}

/// Read a DER TLV at `pos`. Returns (content_start, element_end).
fn read_tlv(data: &[u8], pos: usize) -> Option<(usize, usize)> {
    if pos >= data.len() {
        return None;
    }
    let _tag = data[pos];
    let (length, content_start) = read_der_length(data, pos + 1)?;
    let end = content_start + length;
    if end > data.len() {
        return None;
    }
    Some((content_start, end))
}

/// Skip past the tag and length bytes at `pos`, returning the content start offset.
fn skip_tag_and_length(data: &[u8], pos: usize) -> Option<usize> {
    let (content_start, _) = read_tlv(data, pos)?;
    Some(content_start)
}

/// Read a DER length starting at `pos`. Returns (length_value, offset_after_length).
fn read_der_length(data: &[u8], pos: usize) -> Option<(usize, usize)> {
    if pos >= data.len() {
        return None;
    }
    let first = data[pos];
    if first < 0x80 {
        Some((first as usize, pos + 1))
    } else if first == 0x80 {
        // Indefinite length - not supported
        None
    } else {
        let num_bytes = (first & 0x7F) as usize;
        if num_bytes > 4 || pos + 1 + num_bytes > data.len() {
            return None;
        }
        let mut length: usize = 0;
        for i in 0..num_bytes {
            length = (length << 8) | data[pos + 1 + i] as usize;
        }
        Some((length, pos + 1 + num_bytes))
    }
}

fn detect_abis(archive: &ZipArchive<Cursor<&[u8]>>) -> Vec<String> {
    let names: Vec<_> = archive.file_names().filter_map(Result::ok).collect();
    KNOWN_ABIS
        .iter()
        .filter(|abi| {
            let prefix = format!("lib/{abi}/");
            names.iter().any(|name| name.starts_with(&prefix))
        })
        .map(|abi| (*abi).to_string())
        .collect()
}
