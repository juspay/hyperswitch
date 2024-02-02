#[cfg(feature = "vergen")]
use router_env as env;

#[cfg(feature = "vergen")]
#[tokio::test]
/// This method prints out the values of several environment variables related to the cargo build and git commit, including CARGO_PKG_VERSION, CARGO_PROFILE, VERGEN_GIT_COMMIT_TIMESTAMP, VERGEN_GIT_SHA, VERGEN_RUSTC_SEMVER, and VERGEN_CARGO_TARGET_TRIPLE. It returns a Result indicating success or an error if one occurs.
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
/// Asynchronously retrieves and prints the version, build, and commit information from the environment. 
///
async fn env_macro() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("version : {:?}", env::version!());
    println!("build : {:?}", env::build!());
    println!("commit : {:?}", env::commit!());
    // println!("platform : {:?}", env::platform!());

    assert!(!env::version!().is_empty());
    assert!(!env::build!().is_empty());
    assert!(!env::commit!().is_empty());
    // assert!(env::platform!().len() > 0);

    Ok(())
}
