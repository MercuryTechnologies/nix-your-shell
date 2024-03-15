//! `#[test]` attribute macro for `nix-your-shell` integration tests.

use std::fmt::Display;

use proc_macro::TokenStream;

use quote::quote;
use quote::ToTokens;
use syn::parse;
use syn::parse::Parse;
use syn::parse::ParseStream;
use syn::Attribute;
use syn::Block;
use syn::Ident;
use syn::ItemFn;

/// The shells to generate tests for.
///
/// This mirrors `nix_your_shell::ShellKind` but is defined locally because
/// proc-macro crates cannot depend on the main crate.
///
/// NOTE: We don't have a `Nushell` variant here because Nushell is allergic to being tested. In
/// fact, Nushell's own integration tests don't even run the shell in interactive mode! Clown
/// software.
///
/// See: https://github.com/nushell/nushell/issues/9497
#[derive(Clone, Copy)]
enum ShellKind {
    Bash,
    Fish,
    Zsh,
    Xonsh,
}

impl Display for ShellKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShellKind::Bash => write!(f, "bash"),
            ShellKind::Fish => write!(f, "fish"),
            ShellKind::Zsh => write!(f, "zsh"),
            ShellKind::Xonsh => write!(f, "xonsh"),
        }
    }
}

impl ToTokens for ShellKind {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let variant = match self {
            ShellKind::Bash => quote! { ::nix_your_shell::ShellKind::Bash },
            ShellKind::Fish => quote! { ::nix_your_shell::ShellKind::Fish },
            ShellKind::Zsh => quote! { ::nix_your_shell::ShellKind::Zsh },
            ShellKind::Xonsh => quote! { ::nix_your_shell::ShellKind::Xonsh },
        };
        tokens.extend(variant);
    }
}

const SHELLS: &[ShellKind] = &[
    ShellKind::Bash,
    ShellKind::Fish,
    ShellKind::Zsh,
    ShellKind::Xonsh,
];

/// Runs a test for each shell.
///
/// One test is generated for each shell in the `SHELLS` constant.
#[proc_macro_attribute]
pub fn test(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse annotated function
    let function: ItemFn = parse(item).expect("Could not parse item as function");

    // Generate functions for each shell we want to test.
    let mut ret = TokenStream::new();
    for shell in SHELLS {
        ret.extend::<TokenStream>(
            make_test_fn(function.clone(), *shell)
                .to_token_stream()
                .into(),
        );
    }
    ret
}

struct Attributes(Vec<Attribute>);

impl Parse for Attributes {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self(input.call(Attribute::parse_outer)?))
    }
}

fn make_test_fn(mut function: ItemFn, shell: ShellKind) -> ItemFn {
    let shell_name = shell.to_string();
    let test_name_base = function.sig.ident.to_string();
    let test_name = format!("{test_name_base}_{shell_name}");
    function.sig.ident = Ident::new(&test_name, function.sig.ident.span());

    // Add attributes to enable tracing.
    function.attrs.extend(
        parse::<Attributes>(
            quote! {
                #[::tracing_test::traced_test]
                #[::std::prelude::v1::test]
            }
            .into(),
        )
        .expect("Could not parse quoted attributes")
        .0,
    );

    let old_stmts = std::mem::take(&mut function.block.stmts);

    // Wrap the test code in setup code.
    let new_body = parse::<Block>(
        quote! {
            {
                ::test_harness::ensure_nix_your_shell_bin(::std::env!("CARGO_BIN_EXE_nix-your-shell"));
                ::test_harness::ensure_shell_kind(#shell);
                #(#old_stmts);*
            }
        }
        .into(),
    )
    .expect("Could not parse function body");

    // Replace function body
    *function.block = new_body;

    function
}
