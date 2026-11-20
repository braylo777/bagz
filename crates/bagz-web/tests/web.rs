use sha2::{Digest, Sha256};

#[test]
fn embedded_bytes_match_recorded_hashes() {
    for a in bagz_web::assets() {
        assert_eq!(
            hex(Sha256::digest(a.body.as_bytes()).as_slice()),
            a.sha256,
            "{}",
            a.path
        );
    }
}

#[test]
fn index_links_embedded_script_and_styles() {
    let index = bagz_web::resolve("/");
    assert_eq!(index.path, "/index.html");
    for a in bagz_web::assets().filter(|a| a.path.starts_with("/assets/")) {
        assert!(
            index.body.contains(a.path),
            "index.html does not reference {}",
            a.path
        );
    }
    assert_eq!(bagz_web::resolve("/deck?x=1").path, "/index.html"); // SPA fallback
    assert_eq!(
        bagz_web::resolve("/favicon.svg").content_type,
        "image/svg+xml"
    );
}

#[test]
fn every_catalog_job_ships_in_the_app() {
    // The Rust catalog (src/data/jobs.json) and the bundled app agree on all 12 jobs.
    assert_eq!(bagz_core::Catalog::bundled().unwrap().jobs().len(), 12);
    assert!(
        bagz_web::catalog_gaps().is_empty(),
        "missing: {:?}",
        bagz_web::catalog_gaps()
    );
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}
