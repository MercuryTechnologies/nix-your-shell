use expect_test::expect;

mod harness;
use harness::CommandExt;

#[test]
fn test_help() {
    let help = harness::command()
        .arg("--help")
        .output_checked_utf8()
        .unwrap();

    expect![[r#"
        A `nix` and `nix-shell` wrapper for shells other than `bash`.

        Use by adding `nix-your-shell | source` to your shell configuration.

        Usage: nix-your-shell [OPTIONS] <SHELL> [COMMAND]

        Commands:
          env        Print the shell environment code to use `nix-your-shell`
          nix-shell  Execute a `nix-shell` command, running the shell if no command is explicitly given
          nix        Execute a `nix` command, running the shell if no command is explicitly given

        Arguments:
          <SHELL>
                  The shell to use for wrapped commands and the shell environment. This can be an executable
                  name like `fish` or the path to an executable like `/opt/homebrew/bin/fish`

        Options:
              --log <LOG>
                  Log filter directives, of the form `target[span{field=value}]=level`, where all components
                  except the level are optional.
          
                  Try `debug` or `trace`.
          
                  [env: NIX_YOUR_SHELL_LOG=]
                  [default: info]

              --absolute
                  Print absolute paths to `nix-your-shell` in shell environment code.
          
                  Note that this will not transform the shell argument to an absolute path.
          
                  Absolute paths are automatically printed if `nix-your-shell` isn't on the `$PATH`.

              --nom
                  Use `nom` (`nix-output-monitor`) instead of `nix` for running commands

          -h, --help
                  Print help (see a summary with '-h')

          -V, --version
                  Print version
    "#]].assert_eq(&help.stdout);
    expect![""].assert_eq(&help.stderr);
}
