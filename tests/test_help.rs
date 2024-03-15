use command_error::CommandExt;
use expect_test::expect;
use expect_test::expect_file;
use test_harness::ShellKind;
use test_harness::Test;

#[test]
fn test_help() {
    test_harness::ensure_nix_your_shell_bin(env!("CARGO_BIN_EXE_nix-your-shell"));
    test_harness::ensure_shell_kind(ShellKind::Bash);
    let env = Test::new("data/simple-shell").unwrap();
    let help = env
        .command_raw()
        .arg("--help")
        .output_checked_utf8()
        .unwrap();

    expect_file!["data/help.txt"].assert_eq(&help.stdout);
    expect![""].assert_eq(&help.stderr);
}
