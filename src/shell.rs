use std::fmt::Display;

use camino::Utf8Path;
use camino::Utf8PathBuf;
use color_eyre::eyre;
use color_eyre::eyre::eyre;
use color_eyre::eyre::Context;

/// A user's shell.
#[derive(Clone, Debug)]
pub enum ShellKind {
    /// The `zsh` shell.
    /// <https://zsh.sourceforge.io/>
    Zsh,

    /// The `fish` shell.
    /// <https://fishshell.com/>
    Fish,

    /// A different shell.
    Other(String),
}

impl Display for ShellKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShellKind::Zsh => write!(f, "zsh"),
            ShellKind::Fish => write!(f, "fish"),
            ShellKind::Other(shell) => write!(f, "{shell}"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Shell {
    pub kind: ShellKind,
    pub path: Utf8PathBuf,
}

impl Shell {
    /// Get the `$SHELL` environment variable as a path.
    pub fn get_path() -> eyre::Result<Utf8PathBuf> {
        let shell_path = match std::env::var_os("SHELL") {
            Some(var) => var,
            None => {
                return Err(eyre!("`$SHELL` environment variable is unset"))
                    .wrap_err("Cannot determine shell");
            }
        };
        let shell_path = shell_path
            .to_str()
            .ok_or_else(|| {
                eyre!("`$SHELL` environment variable isn't valid UTF-8: {shell_path:?}")
            })?
            .to_owned();
        Ok(Utf8PathBuf::from(shell_path))
    }

    /// Get the `ShellPath` represented by the `$SHELL` environment variable.
    pub fn from_env() -> eyre::Result<Self> {
        Self::from_path(Self::get_path()?)
            .wrap_err("Failed to determine user shell from `$SHELL` environment variable")
    }

    pub fn from_path(path: impl AsRef<Utf8Path>) -> eyre::Result<Self> {
        let path = path.as_ref();
        let file_name = match path.file_name() {
            Some(name) => name,
            None => {
                return Err(eyre!("Path has no filename: {path:?}"));
            }
        };

        let kind = if file_name.starts_with("zsh") {
            ShellKind::Zsh
        } else if file_name.starts_with("fish") {
            ShellKind::Fish
        } else {
            ShellKind::Other(file_name.to_string())
        };

        Ok(Self {
            kind,
            path: path.to_owned(),
        })
    }
}

impl Display for Shell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.kind)
    }
}
