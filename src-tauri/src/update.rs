use log::info;
use nyquest::r#async::Request;
use semver::Version;

#[derive(serde::Deserialize, PartialEq, Clone, Debug)]
struct Release {
    html_url: String,
    tag_name: String,
}

/// "new" version from releases array
fn get_target_release(current: Version, releases: Vec<Release>) -> Option<Release> {
    let omit_prereleases = current.pre.is_empty();
    releases
        .iter()
        .filter_map(|rel| {
            let Ok(v) = Version::parse(&rel.tag_name) else {
                return None;
            };
            if omit_prereleases && !v.pre.is_empty() {
                return None;
            }
            if v <= current {
                return None;
            }
            Some((v, rel))
        })
        .max_by(|(v1, _), (v2, _)| v1.cmp(v2))
        .map(|(_, r)| r.to_owned())
}

#[test]
fn get_target_release_test() {
    macro_rules! release {
        ( $x:literal ) => {
            Release {
                html_url: $x.to_string(),
                tag_name: $x.to_string(),
            }
        };
    }
    macro_rules! vervec {
        ( $( $x:literal ),* ) => {
            vec![ $( release!($x), )* ]
        };
    }
    macro_rules! vp {
        ( $x:literal ) => {
            Version::parse($x).unwrap()
        };
    }
    assert_eq!(
        get_target_release(vp!("0.0.0"), vervec![]),
        None,
        "empty array -> None"
    );
    assert_eq!(
        get_target_release(vp!("0.1.0"), vervec!["0.0.0", "0.1.0"]),
        None,
        "current is latest release"
    );
    assert_eq!(
        get_target_release(vp!("0.1.1"), vervec!["0.0.0", "0.1.0"]),
        None,
        "current is in future version"
    );
    assert_eq!(
        get_target_release(vp!("0.1.0"), vervec!["0.1.1"]),
        Some(release!("0.1.1")),
        "has new latest release"
    );
    assert_eq!(
        get_target_release(vp!("0.1.0-rc.1"), vervec!["0.1.0-alpha.1", "0.1.0"]),
        Some(release!("0.1.0")),
        "see other channel"
    );
    assert_eq!(
        get_target_release(vp!("0.1.0"), vervec!["0.1.0", "0.2.0-rc.1"]),
        None,
        "ignore prerelease"
    );
    assert_eq!(
        get_target_release(vp!("0.1.0"), vervec!["0.1.0", "0.2.0-rc.1", "0.2.0"]),
        Some(release!("0.2.0")),
        "skip prerelease but see latest"
    );
}

pub async fn check_for_updates(
    current_version: Version,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let repo = "called-d/osc-wardrobe";
    let ua = format!("called-d_osc-wardrobe/{}", current_version);
    let url = format!("https://api.github.com/repos/{}/releases", repo);
    info!("get release info: {:?}", url);
    let client = nyquest::client::ClientBuilder::default()
        .with_header("Accept", "application/vnd.github+json")
        .user_agent(ua)
        .build_async()
        .await?;
    let resp = client.request(Request::get(url)).await?;

    match resp.with_successful_status() {
        Ok(resp) => Ok(
            get_target_release(current_version, resp.json::<Vec<Release>>().await?)
                .map(|r| r.html_url),
        ),
        Err(e) => Err(e.into()),
    }
}
