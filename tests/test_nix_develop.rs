use test_harness::test;
use test_harness::Test;

#[test]
fn test_nix_develop() {
    let env = Test::new("data/simple-shell").unwrap();

    let mut session = env
        .spawn(env.command().args(["nix", "--", "develop"]))
        .unwrap();
    session.send_line("hello").unwrap();
    session.exp_string("Hello, world!").unwrap();
    session.send_line("exit").unwrap();
    session.exp_eof().unwrap();
}
