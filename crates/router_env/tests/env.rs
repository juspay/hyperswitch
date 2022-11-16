use router_env as env;

#[tokio::test]
async fn basic() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("CARGO_PKG_VERSION : {:?}", env!("CARGO_PKG_VERSION"));
    println!("CARGO_PROFILE : {:?}", env!("VERGEN_CARGO_PROFILE"));

    println!(
        "GIT_COMMIT_TIMESTAMP : {:?}",
        env!("VERGEN_GIT_COMMIT_TIMESTAMP")
    );
    println!("GIT_SHA : {:?}", env!("VERGEN_GIT_SHA"));
    println!("GIT_SHA_SHORT : {:?}", env!("VERGEN_GIT_SHA_SHORT"));
    println!("RUSTC_SEMVER : {:?}", env!("VERGEN_RUSTC_SEMVER"));
    println!(
        "CARGO_TARGET_TRIPLE : {:?}",
        env!("VERGEN_CARGO_TARGET_TRIPLE")
    );

    // println!(
    //     "SYSINFO_OS_VERSION : {:?}",
    //     env!("VERGEN_SYSINFO_OS_VERSION")
    // );
    // println!("SYSINFO_CPU_BRAND : {:?}", env!("VERGEN_SYSINFO_CPU_BRAND"));

    // println!("CARGO_BIN_NAME : {}", env!("CARGO_BIN_NAME"));

    Ok(())
}

#[tokio::test]
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
