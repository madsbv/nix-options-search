/// Test utilities to handle in-repo test data, and tests to verify that this data is kept in sync with code.
/// The test data in question is acquired by the `update_sources.py` script. This module should, in addition to defining utility functions to read the test data and make it available to tests in other modules, also run tests of that data itself to catch possible errors or failures of the data acquisition script.
use crate::{
    config::{
        consts::{self, BUILTIN_SOURCES},
        SourceConfig,
    },
    source::{Source, SourceData},
};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::LazyLock,
};

pub(crate) struct SourceWithHTML {
    pub(crate) source: Source,
    pub(crate) data: SourceData,
    pub(crate) data_html: String,
    pub(crate) version_html: String,
    expectations: SourceExpectations,
}

pub(crate) static BUILTIN_SOURCES_WITH_HTML: LazyLock<[SourceWithHTML; 7]> = LazyLock::new(|| {
    let base_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("testing/data");
    BUILTIN_SOURCES_EXPECTATIONS
        .clone()
        .map(|s| read_source_html_from_testdata(s, &base_dir))
});

#[derive(Clone, PartialEq, Debug)]
struct SourceExpectations {
    source_config: &'static SourceConfig,
    min_data_html_length: usize,
    min_version_html_length: usize,
    min_parsed_terms: usize,
}

// List of all builtin sources with expected minimum amounts of associated data. The bounds are set somewhat lower than current actual amounts; if these bounds are violated it's expected, but not guaranteed, to be a bug in nox, and these bounds may need updating over time.
static BUILTIN_SOURCES_EXPECTATIONS: LazyLock<[SourceExpectations; 7]> = LazyLock::new(|| {
    [
        SourceExpectations {
            source_config: &consts::NIX_DARWIN,
            min_data_html_length: 1_000_000,
            min_version_html_length: 1_000_000,
            min_parsed_terms: 1_000,
        },
        SourceExpectations {
            source_config: &consts::NIX_OS,
            min_data_html_length: 23_000_000,
            min_version_html_length: 1_000_000,
            min_parsed_terms: 23_000,
        },
        SourceExpectations {
            source_config: &consts::NIXOS_UNSTABLE,
            min_data_html_length: 23_000_000,
            min_version_html_length: 1_000_000,
            min_parsed_terms: 23_000,
        },
        SourceExpectations {
            source_config: &consts::HOMEMANAGER,
            min_data_html_length: 4_500_000,
            min_version_html_length: 100_000,
            min_parsed_terms: 4_500,
        },
        SourceExpectations {
            source_config: &consts::HOMEMANAGER_NIXOS,
            min_data_html_length: 10_000,
            min_version_html_length: 100_000,
            min_parsed_terms: 10,
        },
        SourceExpectations {
            source_config: &consts::HOMEMANAGER_NIX_DARWIN,
            min_data_html_length: 10_000,
            min_version_html_length: 100_000,
            min_parsed_terms: 10,
        },
        SourceExpectations {
            source_config: &consts::NIX_BUILTINS,
            min_data_html_length: 100_000,
            min_version_html_length: 100_000,
            min_parsed_terms: 100,
        },
    ]
});

fn read_source_html_from_testdata(se: SourceExpectations, base_dir: &Path) -> SourceWithHTML {
    let source_dir = base_dir.join(&se.source_config.name);

    // 1. Ensure the directory exists
    assert!(
                source_dir.exists(),
                "Missing test data directory for '{}'. Run the Python sync script `testing/update_sources.py`.",
                se.source_config.name
            );

    // 2. Verify 'url' file matches code
    let url_path = source_dir.join("url");
    let saved_url = fs::read_to_string(&url_path)
        .unwrap_or_else(|_| panic!("Missing 'url' file for {}", se.source_config.name));
    assert_eq!(
        saved_url.trim(),
        se.source_config.url,
        "URL mismatch for '{}' in file {:?}",
        se.source_config.name,
        url_path
    );

    // 3. Verify 'version_url' file matches code
    let version_url_path = source_dir.join("version_url");
    let saved_version_url = fs::read_to_string(&version_url_path)
        .unwrap_or_else(|_| panic!("Missing 'version_url' file for {}", se.source_config.name));

    let expected_version = se.source_config.version_url.as_deref().unwrap_or("None");
    assert_eq!(
        saved_version_url.trim(),
        expected_version,
        "Version URL mismatch for '{}'",
        se.source_config.name
    );

    // 4. Ensure data_html actually exists and is not empty
    let html_path = source_dir.join("data_html");
    let data_html = fs::read_to_string(&html_path)
        .unwrap_or_else(|_| panic!("Missing 'data_html' for {}", se.source_config.name));
    assert!(
        !data_html.is_empty(),
        "HTML data file for '{}' is empty",
        se.source_config.name
    );

    // 5. Version HTML presence check
    let version_html_path = source_dir.join("version_html");
    let version_html = if se.source_config.version_url.is_some() {
        let version_html = fs::read_to_string(&version_html_path).unwrap_or_else(|_| {
            panic!(
                "Source '{}' has a version_url but no version_html file was found.",
                se.source_config.name
            )
        });
        assert!(
            !version_html.is_empty(),
            "version_html for '{}' is empty",
            se.source_config.name
        );
        version_html
    } else {
        assert!(
            !version_html_path.exists(),
            "Source '{}' has no version_url, but a version_html file exists. Please delete it.",
            se.source_config.name
        );
        data_html.clone()
    };

    let source = Source::from(se.source_config);
    let data = source
        .parse_data(&data_html, &version_html)
        .expect("Can parse test data");

    SourceWithHTML {
        source,
        data,
        data_html,
        version_html,
        expectations: se,
    }
}

#[test]
fn verify_all_builtin_sources_tested() {
    assert_eq!(BUILTIN_SOURCES.len(), BUILTIN_SOURCES_EXPECTATIONS.len());
    assert_eq!(BUILTIN_SOURCES.len(), BUILTIN_SOURCES_WITH_HTML.len());
    for ((s, se), swh) in BUILTIN_SOURCES
        .into_iter()
        .zip(BUILTIN_SOURCES_EXPECTATIONS.iter())
        .zip(BUILTIN_SOURCES_WITH_HTML.iter())
    {
        assert_eq!(s, se.source_config);
        assert_eq!(se, &swh.expectations);
        assert_eq!(swh.source, swh.data.source);
    }
}

#[test]
fn verify_builtin_sources_expectations() {
    for swh in BUILTIN_SOURCES_WITH_HTML.iter() {
        eprintln!("Source: {}", swh.source);
        eprintln!("data_html.len(): {:?}", swh.data_html.len());
        eprintln!("version_html.len(): {}", swh.version_html.len());
        eprintln!("data.opts.len(): {}", swh.data.opts.len());
        eprintln!();
    }
    for swh in BUILTIN_SOURCES_WITH_HTML.iter() {
        eprintln!("Verifying expectations for {}", swh.source);
        assert_eq!(swh.source, Source::from(swh.expectations.source_config));
        assert!(swh.data_html.len() >= swh.expectations.min_data_html_length, "Assertion failed: swh.data_html.len() ({}) >= swh.expectations.min_data_html_length ({})", swh.data_html.len(),  swh.expectations.min_data_html_length);
        assert!(swh.version_html.len() >= swh.expectations.min_version_html_length, "Assertion failed: swh.version_html.len() ({}) >= swh.expectations.min_version_html_length ({})", swh.version_html.len(),  swh.expectations.min_version_html_length);
        assert!(
            swh.data.opts.len() >= swh.expectations.min_parsed_terms,
            "Assertion failed: swh.data.opts.len() ({}) >= swh.expectations.min_parsed_terms ({})",
            swh.data.opts.len(),
            swh.expectations.min_parsed_terms
        );
        eprintln!("Expectations for {} verified", swh.source);
    }
}
