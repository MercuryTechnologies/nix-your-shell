/// Arguments to a `nix` invocation.
#[derive(Debug)]
pub struct NixArgs {
    /// Arguments to the `nix` invocation, including the subcommand.
    pub args: Vec<String>,
    /// Subcommand to run, like `build` or `shell`.
    pub subcommand: Option<String>,
}

fn try_consume_option_values(
    args: &[String],
    ret: &mut Vec<String>,
    i: &mut usize,
    values_to_consume: usize,
) -> bool {
    if *i + values_to_consume >= args.len() {
        // Truncated argv: fewer values remain than the option expects. Keep whatever
        // partial values were actually present so we don't silently drop input.
        ret.extend(args[*i + 1..].iter().cloned());
        return false;
    }

    for offset in 1..=values_to_consume {
        ret.push(args[*i + offset].clone());
    }

    *i += values_to_consume;
    true
}

fn handle_end_of_options(args: &[String], ret: &mut Vec<String>, i: usize) -> bool {
    if args[i] == "--" {
        // End of options sentinel: copy the remaining positional arguments verbatim.
        ret.extend(args[i + 1..].iter().cloned());
        return true;
    }

    false
}

/// Transform arguments to a `nix` invocation to run the specified `command` with the specified
/// `command_args`.
///
/// Only modifies `nix develop` and `nix shell` commands.
pub fn transform_nix(args: Vec<String>, command: &str, command_args: Vec<String>) -> NixArgs {
    let mut ret = Vec::with_capacity(args.len() + 2);

    let mut subcommand = None;

    let mut i = 0;
    while i < args.len() {
        ret.push(args[i].clone());

        if handle_end_of_options(&args, &mut ret, i) {
            break;
        }

        match args[i].as_str() {
            "--help" | "--version"
                | "-c" | "--command"
                => {
                // We already have a command to run.
                return NixArgs {
                        args,
                        subcommand
                    };
            }

            // Two arguments
            "--option"
                | "--redirect"
                | "--override-flake"
                | "--arg"
                | "--argstr"
                | "--override-input"
                => {
                if !try_consume_option_values(&args, &mut ret, &mut i, 2) {
                    // Truncated option value(s); keep input unchanged and stop parsing.
                    break;
                }
            }

            // One argument
            "--log-format"
            | "--access-tokens"
            | "--allowed-impure-host-deps"
            | "--allowed-uris"
            | "--allowed-users"
            | "--bash-prompt"
            | "--bash-prompt-prefix"
            | "--bash-prompt-suffix"
            | "--build-hook"
            | "--build-poll-interval"
            | "--build-users-group"
            | "--builders"
            | "--commit-lockfile-summary"
            | "--connect-timeout"
            | "--cores"
            | "--diff-hook"
            | "--download-attempts"
            | "--download-speed"
            | "--experimental-features"
            | "--extra-access-tokens"
            | "--extra-allowed-impure-host-deps"
            | "--extra-allowed-uris"
            | "--extra-allowed-users"
            | "--extra-experimental-features"
            | "--extra-extra-platforms"
            | "--extra-hashed-mirrors"
            | "--extra-nix-path"
            | "--extra-platforms"
            | "--extra-plugin-files"
            | "--extra-sandbox-paths"
            | "--extra-secret-key-files"
            | "--extra-substituters"
            | "--extra-system-features"
            | "--extra-trusted-public-keys"
            | "--extra-trusted-substituters"
            | "--extra-trusted-users"
            | "--flake-registry"
            | "--gc-reserved-space"
            | "--hashed-mirrors"
            | "--http-connections"
            | "--log-lines"
            | "--max-build-log-size"
            | "--max-free"
            | "--max-jobs"
            | "--max-silent-time"
            | "--min-free"
            | "--min-free-check-interval"
            | "--nar-buffer-size"
            | "--narinfo-cache-negative-ttl"
            | "--narinfo-cache-positive-ttl"
            | "--netrc-file"
            | "--nix-path"
            | "--plugin-files"
            | "--post-build-hook"
            | "--pre-build-hook"
            | "--repeat"
            | "--sandbox-paths"
            | "--secret-key-files"
            | "--stalled-download-timeout"
            | "--store"
            | "--substituters"
            | "--system"
            | "--system-features"
            | "--tarball-ttl"
            | "--timeout"
            | "--trusted-public-keys"
            | "--trusted-substituters"
            | "--trusted-users"
            | "--user-agent-suffix"
            // `nix develop` options
            | "-k" | "--keep"
            | "--phase"
            |"--profile"
            | "--unset"
            | "--eval-store"
            | "-I" | "--include"
            | "--inputs-from"
            | "--update-input"
            | "--expr"
            | "-f" | "--file"
            => {
                if !try_consume_option_values(&args, &mut ret, &mut i, 1) {
                    // Truncated option value; keep input unchanged and stop parsing.
                    break;
                }
            }

            // Zero arguments
            "--offline"
            | "--refresh"
            | "--debug"
            | "-L"
            | "--print-build-logs"
            | "--quiet"
            | "-v"
            | "--verbose"
            | "--accept-flake-config"
            | "--no-accept-flake-config"
            | "--allow-dirty"
            | "--no-allow-dirty"
            | "--allow-import-from-derivation"
            | "--no-allow-import-from-derivation"
            | "--allow-symlinked-store"
            | "--no-allow-symlinked-store"
            | "--allow-unsafe-native-code-during-evaluation"
            | "--no-allow-unsafe-native-code-during-evaluation"
            | "--auto-optimise-store"
            | "--no-auto-optimise-store"
            | "--builders-use-substitutes"
            | "--no-builders-use-substitutes"
            | "--compress-build-log"
            | "--no-compress-build-log"
            | "--darwin-log-sandbox-violations"
            | "--no-darwin-log-sandbox-violations"
            | "--enforce-determinism"
            | "--no-enforce-determinism"
            | "--eval-cache"
            | "--no-eval-cache"
            | "--fallback"
            | "--no-fallback"
            | "--fsync-metadata"
            | "--no-fsync-metadata"
            | "--http2"
            | "--no-http2"
            | "--ignore-try"
            | "--no-ignore-try"
            | "--impersonate-linux-26"
            | "--no-impersonate-linux-26"
            | "--keep-build-log"
            | "--no-keep-build-log"
            | "--keep-derivations"
            | "--no-keep-derivations"
            | "--keep-env-derivations"
            | "--no-keep-env-derivations"
            | "--keep-failed"
            | "--no-keep-failed"
            | "--keep-going"
            | "--no-keep-going"
            | "--keep-outputs"
            | "--no-keep-outputs"
            | "--preallocate-contents"
            | "--no-preallocate-contents"
            | "--print-missing"
            | "--no-print-missing"
            | "--pure-eval"
            | "--no-pure-eval"
            | "--require-sigs"
            | "--no-require-sigs"
            | "--restrict-eval"
            | "--no-restrict-eval"
            | "--run-diff-hook"
            | "--no-run-diff-hook"
            | "--sandbox"
            | "--no-sandbox"
            | "--sandbox-fallback"
            | "--no-sandbox-fallback"
            | "--show-trace"
            | "--no-show-trace"
            | "--substitute"
            | "--no-substitute"
            | "--sync-before-registering"
            | "--no-sync-before-registering"
            | "--trace-function-calls"
            | "--no-trace-function-calls"
            | "--trace-verbose"
            | "--no-trace-verbose"
            | "--use-case-hack"
            | "--no-use-case-hack"
            | "--use-registries"
            | "--no-use-registries"
            | "--use-sqlite-wal"
            | "--no-use-sqlite-wal"
            | "--warn-dirty"
            | "--no-warn-dirty"
            | "--relaxed-sandbox"
            // `nix develop` options
            | "--build"
            | "--check"
            | "--configure"
            | "--debugger"
            | "-i" | "--ignore-environment"
            | "--install"
            | "--installcheck"
            | "--unpack"
            | "--impure"
            | "--commit-lock-file"
            | "--no-registries"
            | "--no-update-lock-file"
            | "--no-write-lock-file"
            | "--recreate-lock-file"
            | "--derivation"
            => {}

            "build" | "develop" | "flake" | "help" | "profile" | "repl" | "run" | "search"
            | "shell" | "bundle" | "copy" | "edit" | "eval" | "fmt" | "log" | "path-info"
            | "registry" | "why-depends" | "daemon" | "describe-stores" | "hash" | "key"
            | "nar" | "print-dev-env" | "realisation" | "show-config" | "show-derivation"
            | "store" | "doctor" | "upgrade-nix" => {
                // Top-level subcommand.

                // Replace `subcommand` unless it already has a value.
                subcommand.get_or_insert_with(|| args[i].clone());
            }

            _ => {
                // Unknown argument, ignore.
            }
        }

        i += 1;
    }

    // We want to add our `--command` flag right at the end, because `--command` makes *all the
    // rest of the positional arguments* get parsed as arguments to the command.
    //
    // Note that this behavior is unlike `nix-shell`, where the `--command` flag takes one argument
    // that may include spaces...
    match subcommand.as_deref() {
        Some("develop") | Some("shell") => {
            ret.push("--command".into());
            ret.push(command.into());
            ret.extend(command_args);
        }

        _ => {}
    }

    NixArgs {
        args: ret,
        subcommand,
    }
}

/// Transform arguments to a `nix-shell` invocation to run the specified `command` with the
/// specified `command_args`.
pub fn transform_nix_shell(
    args: Vec<String>,
    command: &str,
    command_args: &[String],
) -> Vec<String> {
    let mut ret = Vec::with_capacity(args.len() + 2);
    ret.push("--command".into());
    ret.push(shell_words::join(
        std::iter::once(command).chain(command_args.iter().map(|arg| arg.as_str())),
    ));

    let mut i = 0;
    while i < args.len() {
        ret.push(args[i].clone());

        if handle_end_of_options(&args, &mut ret, i) {
            break;
        }

        match args[i].as_str() {
            // Two arguments
            "--arg" | "--argstr"
                // `nix-store`
                | "--option"
                // From `nix-build` source...
                | "--override-flake"
                => {
                if !try_consume_option_values(&args, &mut ret, &mut i, 2) {
                    // Truncated option value(s); keep input unchanged and stop parsing.
                    break;
                }
            }

            // One argument
            "--attr" | "-A" | "--exclude" | "--keep"
                | "-i" // Interpreter, shebang only
                // `nix-store`
                | "--add-root"
                // From `nix-build` source...
                | "--cores"
                | "--max-silent-time"
                | "--timeout"
                | "--store-uri"
                | "-I" | "--include"
                | "--eval-store"
                | "-o" | "--out-link"
                => {
                if !try_consume_option_values(&args, &mut ret, &mut i, 1) {
                    // Truncated option value; keep input unchanged and stop parsing.
                    break;
                }
            }

            // Zero arguments
            "--pure" | "--impure"
                // `--packages` changes the meaning of positional arguments, so we effectively
                // ignore it.
                | "-p" | "--packages"
                // Also changes meaning of positional arguments.
                | "-E" | "--expr"
                // `nix-store`
                | "--dry-run" | "--ignore-unknown" | "--check"
                // From `nix-build` source...
                | "-Q" | "--no-build-output"
                | "-K" | "--keep-failed"
                | "-k" | "--keep-going"
                | "--fallback"
                | "--readonly-mode"
                | "--no-gc-warning"
                | "--add-drv-link" | "--indirect"
                | "--no-out-link" | "--no-link"
                | "--drv-link"
                | "--repair"
                | "--run-env"
                => {
                // Nothing to skip.
            }

            "--command" | "--run"
                | "--help"
                | "--version"
                => {
                // We already have a command to run; don't add our own `--command {command}`
                // arguments.
                return args;
            }

            _ => {
                // Unknown argument, ignore.
            }
        }

        i += 1;
    }

    ret
}

#[cfg(test)]
mod tests {
    use super::transform_nix;
    use super::transform_nix_shell;

    fn strs(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    // --- transform_nix: truncated option values ---

    /// A one-argument option with no value must not panic; we keep what we have and stop.
    #[test]
    fn nix_one_arg_option_missing_value_does_not_panic() {
        let out = transform_nix(strs(&["develop", "--keep"]), "fish", vec![]);
        assert_eq!(out.subcommand.as_deref(), Some("develop"));
        assert_eq!(out.args, strs(&["develop", "--keep", "--command", "fish"]));
    }

    /// A two-argument option with both values missing must not panic.
    #[test]
    fn nix_two_arg_option_missing_both_values_does_not_panic() {
        let out = transform_nix(strs(&["develop", "--option"]), "fish", vec![]);
        assert_eq!(
            out.args,
            strs(&["develop", "--option", "--command", "fish"])
        );
    }

    /// A two-argument option with one value present keeps that partial value rather than
    /// silently dropping it, per the "keep already copied args" intent.
    ///
    /// Regression guard: before the truncation fix, the present value (`foo`) was dropped
    /// from the output.
    #[test]
    fn nix_two_arg_option_partial_value_is_preserved() {
        let out = transform_nix(strs(&["develop", "--option", "foo"]), "fish", vec![]);
        assert_eq!(
            out.args,
            strs(&["develop", "--option", "foo", "--command", "fish"])
        );
    }

    // --- transform_nix: end-of-options (`--`) handling ---

    /// `--` mid-args: the remainder is copied verbatim and parsing stops.
    #[test]
    fn nix_double_dash_copies_remainder_verbatim() {
        let out = transform_nix(strs(&["develop", "--", "extra", "args"]), "fish", vec![]);
        assert_eq!(
            out.args,
            strs(&["develop", "--", "extra", "args", "--command", "fish"])
        );
    }

    /// `--` as the final token must not read past the end of argv.
    #[test]
    fn nix_double_dash_as_final_token_does_not_panic() {
        let out = transform_nix(strs(&["develop", "--"]), "fish", vec![]);
        assert_eq!(out.args, strs(&["develop", "--", "--command", "fish"]));
    }

    /// `--` appearing as the value of an option is consumed as that value, not treated
    /// as the end-of-options sentinel.
    #[test]
    fn nix_double_dash_as_option_value_is_consumed() {
        let out = transform_nix(strs(&["develop", "--keep", "--", "pkg"]), "fish", vec![]);
        assert_eq!(
            out.args,
            strs(&["develop", "--keep", "--", "pkg", "--command", "fish"])
        );
    }

    // --- transform_nix_shell: truncated option values ---

    /// A two-argument option with one value present keeps that partial value in the
    /// `nix-shell` path too. Regression guard for the same partial-value-drop bug.
    #[test]
    fn nix_shell_two_arg_option_partial_value_is_preserved() {
        let out = transform_nix_shell(strs(&["--arg", "x"]), "fish", &[]);
        assert_eq!(out, strs(&["--command", "fish", "--arg", "x"]));
    }

    /// A truncated one-argument option in the `nix-shell` path must not panic.
    #[test]
    fn nix_shell_one_arg_option_missing_value_does_not_panic() {
        let out = transform_nix_shell(strs(&["--attr"]), "fish", &[]);
        assert_eq!(out, strs(&["--command", "fish", "--attr"]));
    }

    /// `--` in the `nix-shell` path copies the remainder verbatim.
    #[test]
    fn nix_shell_double_dash_copies_remainder_verbatim() {
        let out = transform_nix_shell(strs(&["--pure", "--", "rest"]), "fish", &[]);
        assert_eq!(out, strs(&["--command", "fish", "--pure", "--", "rest"]));
    }
}
