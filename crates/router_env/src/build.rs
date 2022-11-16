use vergen::{vergen, Config, ShaKind};

fn main() {
    let mut config = Config::default();

    let build = config.build_mut();
    *build.enabled_mut() = false;
    *build.skip_if_error_mut() = true;

    let cargo = config.cargo_mut();
    *cargo.enabled_mut() = true;
    *cargo.features_mut() = false;
    *cargo.profile_mut() = true;
    *cargo.target_triple_mut() = true;

    let git = config.git_mut();
    *git.enabled_mut() = true;
    *git.commit_author_mut() = false;
    *git.commit_count_mut() = false;
    *git.commit_message_mut() = false;
    *git.commit_timestamp_mut() = true;
    *git.semver_mut() = false;
    *git.skip_if_error_mut() = true;
    *git.sha_kind_mut() = ShaKind::Both;
    *git.skip_if_error_mut() = true;

    let rustc = config.rustc_mut();
    *rustc.enabled_mut() = true;
    *rustc.channel_mut() = false;
    *rustc.commit_date_mut() = false;
    *rustc.host_triple_mut() = false;
    *rustc.llvm_version_mut() = false;
    *rustc.semver_mut() = true;
    *rustc.sha_mut() = true; // required for sever been available
    *rustc.skip_if_error_mut() = true;

    let sysinfo = config.sysinfo_mut();
    *sysinfo.enabled_mut() = false;
    *sysinfo.os_version_mut() = false;
    *sysinfo.user_mut() = false;
    *sysinfo.memory_mut() = false;
    *sysinfo.cpu_vendor_mut() = false;
    *sysinfo.cpu_core_count_mut() = false;
    *sysinfo.cpu_name_mut() = false;
    *sysinfo.cpu_brand_mut() = false;
    *sysinfo.cpu_frequency_mut() = false;
    *sysinfo.skip_if_error_mut() = true;

    vergen(config).expect("Problem determining current platform characteristics");
}
