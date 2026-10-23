//! A source's published versions, newest first, with the files each has.

use crate::sources::catalog::Source;
use serde::{Deserialize, Serialize};

/// Releases looked at in a GitHub repo (its newest).
const GITHUB_PAGE: usize = 30;
/// A GitHub release asset's digest starts so.
const SHA256_PREFIX: &str = "sha256:";
/// The source archive of a `git` release, as an asset.
pub const SOURCE_ARCHIVE: &str = "source.tar.gz";

/// A file of a release.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Asset {
    pub name: String,
    pub url: String,
    /// Bytes; 0 when it isn't said.
    pub size: u64,
    pub sha256: Option<String>,
}

/// One published version.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Release {
    pub version: String,
    pub assets: Vec<Asset>,
    /// Its release notes' page.
    pub notes_url: Option<String>,
    /// Its title, and when it came out (RFC 3339), where the source says.
    pub title: Option<String>,
    pub published: Option<String>,
}

impl Release {
    fn bare(version: &str) -> Self {
        Self {
            version: version.to_string(),
            assets: Vec::new(),
            notes_url: None,
            title: None,
            published: None,
        }
    }

    /// The asset named `name`.
    pub fn asset(&self, name: &str) -> Option<&Asset> {
        self.assets.iter().find(|a| a.name == name)
    }

    /// The asset whose name ends with `suffix`.
    pub fn asset_ending(&self, suffix: &str) -> Option<&Asset> {
        self.assets.iter().find(|a| a.name.ends_with(suffix))
    }
}

/// Whether `release` of `source` has what this platform installs: a GitHub
/// source's asset for it (other sources always do).
pub fn carries(source: &Source, release: &Release) -> bool {
    match source {
        Source::Github { .. } => source
            .asset_suffix()
            .is_some_and(|suffix| release.asset_ending(suffix).is_some()),
        _ => true,
    }
}

/// `source`'s versions, newest first.
pub fn of(source: &Source) -> Result<Vec<Release>, String> {
    match source {
        Source::Github { repo, .. } => github(repo, false),
        Source::Git { repo } => github(repo, true),
        Source::Crate { name } => crates_io(name),
        Source::Rustup { toolchain } | Source::RustTargets { toolchain, .. } => {
            Ok(vec![Release::bare(toolchain)])
        }
        Source::Adoptium { major } => adoptium(*major),
        Source::Zig => zig(),
    }
}

#[derive(Deserialize)]
struct GhRelease {
    tag_name: String,
    name: Option<String>,
    published_at: Option<String>,
    html_url: String,
    /// None for a draft (seen with a token).
    tarball_url: Option<String>,
    draft: bool,
    assets: Vec<GhAsset>,
}

#[derive(Deserialize)]
struct GhAsset {
    name: String,
    browser_download_url: String,
    size: u64,
    digest: Option<String>,
}

/// A repo's published releases (`source`: with the tag's source archive).
fn github(repo: &str, source: bool) -> Result<Vec<Release>, String> {
    let url = format!("https://api.github.com/repos/{repo}/releases?per_page={GITHUB_PAGE}");
    let list: Vec<GhRelease> = crate::sources::http::get_json(&url)?;
    // Pre-releases too: a beta is how a version reaches its testers (its
    // title says what it is).
    Ok(list
        .into_iter()
        .filter(|r| !r.draft)
        .map(|r| {
            let mut assets: Vec<Asset> = r
                .assets
                .into_iter()
                .map(|a| Asset {
                    sha256: a
                        .digest
                        .and_then(|d| d.strip_prefix(SHA256_PREFIX).map(str::to_string)),
                    name: a.name,
                    url: a.browser_download_url,
                    size: a.size,
                })
                .collect();
            if source && let Some(url) = r.tarball_url {
                assets.push(Asset {
                    name: SOURCE_ARCHIVE.into(),
                    url,
                    size: 0,
                    sha256: None,
                });
            }
            Release {
                version: r.tag_name.trim_start_matches('v').to_string(),
                assets,
                notes_url: Some(r.html_url),
                title: r.name.filter(|n| !n.is_empty()),
                published: r.published_at,
            }
        })
        .collect())
}

#[derive(Deserialize)]
struct CratesIo {
    versions: Vec<CrateVersion>,
}

#[derive(Deserialize)]
struct CrateVersion {
    num: String,
    yanked: bool,
}

fn crates_io(name: &str) -> Result<Vec<Release>, String> {
    let all: CratesIo =
        crate::sources::http::get_json(&format!("https://crates.io/api/v1/crates/{name}"))?;
    Ok(all
        .versions
        .into_iter()
        .filter(|v| !v.yanked)
        .map(|v| Release::bare(&v.num))
        .collect())
}

#[derive(Deserialize)]
struct AdoptiumAsset {
    binary: AdoptiumBinary,
    version: AdoptiumVersion,
}

#[derive(Deserialize)]
struct AdoptiumBinary {
    package: AdoptiumPackage,
}

#[derive(Deserialize)]
struct AdoptiumPackage {
    name: String,
    link: String,
    size: u64,
    checksum: Option<String>,
}

#[derive(Deserialize)]
struct AdoptiumVersion {
    semver: String,
}

/// The newest Temurin JDK of `major` for this machine.
fn adoptium(major: u32) -> Result<Vec<Release>, String> {
    let (os, arch) =
        crate::machine::platform::adoptium().ok_or("Adoptium has no JDK for this platform")?;
    let url = format!(
        "https://api.adoptium.net/v3/assets/latest/{major}/hotspot?os={os}&architecture={arch}&image_type=jdk"
    );
    let list: Vec<AdoptiumAsset> = crate::sources::http::get_json(&url)?;
    Ok(list
        .into_iter()
        .map(|a| {
            let p = a.binary.package;
            Release {
                version: a.version.semver,
                assets: vec![Asset {
                    name: p.name,
                    url: p.link,
                    size: p.size,
                    sha256: p.checksum,
                }],
                notes_url: None,
                title: None,
                published: None,
            }
        })
        .collect())
}

#[derive(Deserialize)]
struct ZigBuild {
    tarball: String,
    shasum: String,
    size: String,
}

/// The stable zig releases for this machine, newest first.
fn zig() -> Result<Vec<Release>, String> {
    let index: serde_json::Map<String, serde_json::Value> =
        crate::sources::http::get_json("https://ziglang.org/download/index.json")?;
    let host = crate::machine::platform::zig();
    let mut out: Vec<Release> = index
        .into_iter()
        .filter(|(version, _)| semver::Version::parse(version).is_ok_and(|v| v.pre.is_empty()))
        .filter_map(|(version, builds)| {
            let b: ZigBuild = serde_json::from_value(builds.get(&host)?.clone()).ok()?;
            let name = b.tarball.rsplit('/').next()?.to_string();
            let asset = Asset {
                name,
                url: b.tarball,
                size: b.size.parse().unwrap_or(0),
                sha256: Some(b.shasum),
            };
            Some(Release {
                version,
                assets: vec![asset],
                notes_url: None,
                title: None,
                published: None,
            })
        })
        .collect();
    out.sort_by(|a, b| crate::plans::compat::compare(&b.version, &a.version));
    Ok(out)
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_draft_without_a_source_archive_reads() {
        let draft = r#"[{"tag_name": "v2.2.0", "name": null, "published_at": null, "html_url": "u",
            "tarball_url": null, "draft": true, "prerelease": false, "assets": []}]"#;
        let read: Vec<super::GhRelease> = serde_json::from_str(draft).unwrap();
        assert!(read[0].draft && read[0].tarball_url.is_none());
    }

    #[test]
    fn a_release_without_this_platforms_file_isnt_one_to_install() {
        let suffix = "symbols.zip".to_owned();
        let assets = [(yog_ids::platform::host_tag(), suffix.clone())]
            .into_iter()
            .collect();
        let source = crate::sources::catalog::Source::Github {
            repo: "o/r".into(),
            assets,
            system: None,
        };
        let mut r = super::Release::bare("1.0.0");
        assert!(!super::carries(&source, &r));
        r.assets.push(super::Asset {
            name: format!("yog-runtime-{suffix}"),
            url: String::new(),
            size: 0,
            sha256: None,
        });
        assert!(super::carries(&source, &r));
    }
}
