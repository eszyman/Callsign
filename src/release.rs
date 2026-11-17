//! This copy's version, compared with the latest public GitHub release.

use std::cmp::Ordering;
use std::time::Duration;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

const LATEST_URL: &str = "https://api.github.com/repos/eszyman/Callsign/releases/latest";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublishedRelease {
    pub tag: String,
    pub page: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VersionOrder {
    Same,
    Behind,
    Ahead,
}

#[derive(Clone, Debug)]
pub enum ReleaseLook {
    Checking,
    Ready {
        published: PublishedRelease,
        order: VersionOrder,
    },
    Failed(String),
}

pub fn fetch_latest_release() -> Result<PublishedRelease, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(8))
        .user_agent(format!("Callsign/{VERSION}"))
        .build()
        .map_err(|err| err.to_string())?;
    let response = client
        .get(LATEST_URL)
        .header("Accept", "application/vnd.github+json")
        .send()
        .map_err(|err| format!("Couldn't reach GitHub. {err}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("GitHub answered {status}."));
    }
    let body: LatestRelease = response
        .json()
        .map_err(|err| format!("GitHub's answer wasn't a release. {err}"))?;
    if body.tag_name.trim().is_empty() {
        return Err("GitHub's latest release has no version.".into());
    }
    Ok(PublishedRelease {
        tag: body.tag_name,
        page: body.html_url,
    })
}

pub fn compare_versions(current: &str, latest_tag: &str) -> Result<VersionOrder, String> {
    let current_parts = parse_version(current)
        .ok_or_else(|| format!("This copy's version '{current}' is not a number."))?;
    let latest_parts = parse_version(latest_tag)
        .ok_or_else(|| format!("GitHub's version '{latest_tag}' is not a number."))?;
    Ok(match current_parts.cmp(&latest_parts) {
        Ordering::Equal => VersionOrder::Same,
        Ordering::Less => VersionOrder::Behind,
        Ordering::Greater => VersionOrder::Ahead,
    })
}

fn parse_version(text: &str) -> Option<(u64, u64, u64)> {
    let text = text.trim().trim_start_matches(['v', 'V']);
    let text = text.split(['-', '+']).next()?.trim();
    if text.is_empty() {
        return None;
    }
    let mut parts = text.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next().unwrap_or("0").parse().ok()?;
    let patch = parts.next().unwrap_or("0").parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

#[derive(serde::Deserialize)]
struct LatestRelease {
    tag_name: String,
    html_url: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_line_up_with_or_without_a_v() {
        assert_eq!(
            compare_versions("0.3.0", "v0.3.0").unwrap(),
            VersionOrder::Same
        );
        assert_eq!(
            compare_versions("0.2.0", "v0.3.0").unwrap(),
            VersionOrder::Behind
        );
        assert_eq!(
            compare_versions("0.3.0", "v0.2.0").unwrap(),
            VersionOrder::Ahead
        );
        assert_eq!(
            compare_versions("1.0.0", "v0.9.9").unwrap(),
            VersionOrder::Ahead
        );
    }
}
