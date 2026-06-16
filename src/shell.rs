use std::fmt::Display;

use camino::Utf8Path;
use camino::Utf8PathBuf;
use miette::miette;

/// A user's shell.
#[derive(Clone, Copy, Debug)]
pub enum ShellKind {
    /// The `zsh` shell.
    /// <https://zsh.sourceforge.io/>
    Zsh,

    /// The `fish` shell.
    /// <https://fishshell.com/>
    Fish,

    /// The `bash` shell.
    /// <https://www.gnu.org/software/bash/>
    Bash,

    /// The `nu` shell
    /// <https://www.nushell.sh/>
    Nushell,

    /// The `xonsh` shell.
    /// <https://xon.sh>
    Xonsh,
}

impl Display for ShellKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShellKind::Zsh => write!(f, "zsh"),
            ShellKind::Fish => write!(f, "fish"),
            ShellKind::Bash => write!(f, "bash"),
            ShellKind::Nushell => write!(f, "nu"),
            ShellKind::Xonsh => write!(f, "xonsh"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Shell {
    pub kind: ShellKind,
    pub path: Utf8PathBuf,
}

impl Shell {
    pub fn from_path(path: impl AsRef<Utf8Path>) -> miette::Result<Self> {
        let path = path.as_ref();
        let file_name = match path.file_name() {
            Some(name) => name,
            None => {
                return Err(miette!("Path has no filename: {path:?}"));
            }
        };

        let kind = match file_name {
            name if name.starts_with("zsh") => ShellKind::Zsh,
            name if name.starts_with("fish") => ShellKind::Fish,
            name if name.starts_with("bash") => ShellKind::Bash,
            name if name.starts_with("nu") => ShellKind::Nushell,
            name if name.starts_with("xonsh") => ShellKind::Xonsh,
            _ => return Err(miette!("Unknown shell: {file_name:?}")),
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
