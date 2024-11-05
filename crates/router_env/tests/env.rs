#![allow(clippy::print_stdout)]

#[cfg(feature = "vergen")]
use router_env as env;

#[cfg(feature = "vergen")]
#[tokio::test]
async fn basic() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("CARGO_PKG_VERSION : {:?}", env!("CARGO_PKG_VERSION"));
    println!("CARGO_PROFILE : {:?}", env!("CARGO_PROFILE"));

    println!(
        "GIT_COMMIT_TIMESTAMP : {:?}",
        env!("VERGEN_GIT_COMMIT_TIMESTAMP")
    );
    println!("GIT_SHA : {:?}", env!("VERGEN_GIT_SHA"));
    println!("RUSTC_SEMVER : {:?}", env!("VERGEN_RUSTC_SEMVER"));
    println!(
        "CARGO_TARGET_TRIPLE : {:?}",
        env!("VERGEN_CARGO_TARGET_TRIPLE")
    );

    Ok(())
}

#[cfg(feature = "vergen")]
#[tokio::test]
async fn env_macro() {
    println!("version : {:?}", env::version!());
    println!("build : {:?}", env::build!());
    println!("commit : {:?}", env::commit!());
    // println!("platform : {:?}", env::platform!());

    assert!(!env::version!().is_empty());
    assert!(!env::build!().is_empty());
    assert!(!env::commit!().is_empty());
    // assert!(env::platform!().len() > 0);
}
