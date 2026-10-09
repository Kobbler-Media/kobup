use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OutputMetadata {
    #[serde(default)]
    baseline_profiles: Vec<ProfileBand>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProfileBand {
    min_api: i64,
    baseline_profiles: Vec<String>,
}

/// The `.dm` files AGP writes next to a release APK, one per API band, as listed in the
/// `output-metadata.json` beside it.
pub fn profiles_beside(apk: &Path) -> Vec<(i64, PathBuf)> {
    let Some(dir) = apk.parent() else {
        return Vec::new();
    };
    let Ok(text) = std::fs::read_to_string(dir.join("output-metadata.json")) else {
        return Vec::new();
    };
    let Ok(metadata) = serde_json::from_str::<OutputMetadata>(&text) else {
        return Vec::new();
    };
    metadata
        .baseline_profiles
        .into_iter()
        .filter_map(|band| Some((band.min_api, dir.join(band.baseline_profiles.first()?))))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::profiles_beside;
    use std::path::PathBuf;

    fn output_dir(name: &str, metadata: Option<&str>) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("kobup-cli-dm-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("app-release.apk"), b"apk").unwrap();
        if let Some(metadata) = metadata {
            std::fs::write(dir.join("output-metadata.json"), metadata).unwrap();
        }
        dir
    }

    #[test]
    fn reads_the_profiles_agp_lists_beside_the_apk() {
        let dir = output_dir(
            "agp",
            Some(
                r#"{"elements":[{"outputFile":"app-release.apk"}],"baselineProfiles":[
                    {"minApi":28,"maxApi":30,"baselineProfiles":["baselineProfiles/1/app-release.dm"]},
                    {"minApi":31,"maxApi":2147483647,"baselineProfiles":["baselineProfiles/0/app-release.dm"]}]}"#,
            ),
        );
        assert_eq!(
            profiles_beside(&dir.join("app-release.apk")),
            vec![
                (28, dir.join("baselineProfiles/1/app-release.dm")),
                (31, dir.join("baselineProfiles/0/app-release.dm")),
            ]
        );
    }

    #[test]
    fn an_apk_without_profiles_uploads_alone() {
        for (name, metadata) in [
            ("none", None),
            ("no-profiles", Some(r#"{"elements":[]}"#)),
            ("garbage", Some("not json")),
        ] {
            let dir = output_dir(name, metadata);
            assert!(
                profiles_beside(&dir.join("app-release.apk")).is_empty(),
                "{name}"
            );
        }
    }
}
