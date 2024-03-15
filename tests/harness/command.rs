//! Utilities for running commands and providing user-friendly error messages.
//!
//! TODO: Make this its own crate!!!
//!
//! The big open question is error messages.

use std::borrow::Cow;
use std::error::Error;
use std::fmt::Debug;
use std::fmt::Display;
use std::process::{Command, ExitStatus, Output};

use color_eyre::eyre::eyre;
use color_eyre::eyre::{self, Context};
use color_eyre::{Help, SectionExt};

/// Like [`Output`], but UTF-8 decoded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Utf8Output {
    pub status: ExitStatus,
    pub stdout: String,
    pub stderr: String,
}

impl TryFrom<Output> for Utf8Output {
    type Error = eyre::Report;

    fn try_from(
        Output {
            status,
            stdout,
            stderr,
        }: Output,
    ) -> Result<Self, Self::Error> {
        let stdout = String::from_utf8(stdout).map_err(|err| {
            eyre!(
                "Stdout contained invalid UTF-8: {}",
                String::from_utf8_lossy(err.as_bytes())
            )
        })?;
        let stderr = String::from_utf8(stderr).map_err(|err| {
            eyre!(
                "Stderr contained invalid UTF-8: {}",
                String::from_utf8_lossy(err.as_bytes())
            )
        })?;

        Ok(Utf8Output {
            status,
            stdout,
            stderr,
        })
    }
}

impl TryFrom<&Output> for Utf8Output {
    type Error = eyre::Report;

    fn try_from(
        Output {
            status,
            stdout,
            stderr,
        }: &Output,
    ) -> Result<Self, Self::Error> {
        let stdout = String::from_utf8(stdout.to_vec()).map_err(|err| {
            eyre!(
                "Stdout contained invalid UTF-8: {}",
                String::from_utf8_lossy(err.as_bytes())
            )
        })?;
        let stderr = String::from_utf8(stderr.to_vec()).map_err(|err| {
            eyre!(
                "Stderr contained invalid UTF-8: {}",
                String::from_utf8_lossy(err.as_bytes())
            )
        })?;
        let status = *status;

        Ok(Utf8Output {
            status,
            stdout,
            stderr,
        })
    }
}

/// Extension trait for [`Command`], adding helpers to gather output while checking the exit
/// status.
pub trait CommandExt {
    /// Display the command for user-facing logs.
    fn display(&self) -> String;

    /// Like [`Command::output`], but checks the exit status and provides nice error messages.
    #[track_caller]
    fn output_checked(&mut self) -> eyre::Result<Output>;

    /// Like [`output_checked`], but also decodes Stdout and Stderr as UTF-8.
    #[track_caller]
    fn output_checked_utf8(&mut self) -> eyre::Result<Utf8Output>;

    /// Like [`output_checked`] but a closure determines if the command failed instead of
    /// [`ExitStatus::success`].
    #[track_caller]
    fn output_checked_with<O>(
        &mut self,
        succeeded: impl Fn(&O) -> Result<(), ()>,
    ) -> eyre::Result<O>
    where
        O: TryFrom<Output>,
        O: Debug + OutputLike + Send + Sync + 'static,
        <O as TryFrom<Output>>::Error: Into<eyre::Report>;

    /// Like `output_checked_with`, but the closure's return value is used as the function's return
    /// value.
    ///
    /// Useful to apply constraints to the output.
    #[track_caller]
    fn output_transformed_with<O, R, E>(
        &mut self,
        succeeded: impl Fn(&O) -> Result<R, Option<E>>,
    ) -> eyre::Result<R>
    where
        O: TryFrom<Output>,
        O: Debug + OutputLike + Send + Sync + 'static,
        <O as TryFrom<Output>>::Error: Into<eyre::Report>,
        E: Display;

    /// Like [`output_checked_with`], but also decodes Stdout and Stderr as UTF-8.
    #[track_caller]
    fn output_checked_with_utf8(
        &mut self,
        succeeded: impl Fn(&Utf8Output) -> Result<(), ()>,
    ) -> eyre::Result<Utf8Output>;

    /// Like [`Command::status`], but gives a nice error message if the status is unsuccessful
    /// rather than returning the [`ExitStatus`].
    #[track_caller]
    fn status_checked(&mut self) -> eyre::Result<()>;

    /// Log the command that will be run. Prints a user-facing message before executing a `sudo`
    /// command.
    ///
    /// If `bootstrap-mercury` is run with the `--confirm-sudo` option, this method also confirms
    /// with the user before executing a `sudo` command.
    fn log_and_confirm(&self) -> eyre::Result<()>;
}

impl CommandExt for Command {
    fn log_and_confirm(&self) -> eyre::Result<()> {
        let command = self.display();
        tracing::debug!(?command, "Executing command");
        Ok(())
    }

    fn output_transformed_with<O, R, E>(
        &mut self,
        succeeded: impl Fn(&O) -> Result<R, Option<E>>,
    ) -> eyre::Result<R>
    where
        O: TryFrom<Output>,
        O: Debug + OutputLike + Send + Sync + 'static,
        <O as TryFrom<Output>>::Error: Into<eyre::Report>,
        E: Display,
    {
        let (output, exec_error) = get_output_as(self)?;
        succeeded(&output).or_else(|user_err| {
            CommandError::from_exec_error(exec_error, output).into_report_wrapped(user_err)
        })
    }

    fn output_checked_with<O>(
        &mut self,
        succeeded: impl Fn(&O) -> Result<(), ()>,
    ) -> eyre::Result<O>
    where
        O: TryFrom<Output>,
        O: Debug + OutputLike + Send + Sync + 'static,
        <O as TryFrom<Output>>::Error: Into<eyre::Report>,
    {
        let (output, exec_error) = get_output_as(self)?;
        match succeeded(&output) {
            Ok(()) => Ok(output),
            Err(()) => CommandError::from_exec_error(exec_error, output).into_report(),
        }
    }

    fn output_checked_with_utf8(
        &mut self,
        succeeded: impl Fn(&Utf8Output) -> Result<(), ()>,
    ) -> eyre::Result<Utf8Output> {
        self.output_checked_with(succeeded)
    }

    fn output_checked(&mut self) -> eyre::Result<Output> {
        self.output_checked_with(|output: &Output| {
            if output.status.success() {
                Ok(())
            } else {
                Err(())
            }
        })
    }

    fn output_checked_utf8(&mut self) -> eyre::Result<Utf8Output> {
        self.output_checked_with_utf8(|output| {
            if output.status.success() {
                Ok(())
            } else {
                Err(())
            }
        })
    }

    fn status_checked(&mut self) -> eyre::Result<()> {
        self.log_and_confirm()?;
        let exec_error = CommandExecError::from(&*self);
        #[allow(clippy::disallowed_methods)]
        let status = self
            .status()
            .wrap_err_with(|| exec_error.clone())
            .with_suggestion(|| {
                format!(
                    "Is `{}` installed and present in your `$PATH`?",
                    exec_error.program
                )
            })?;
        if status.success() {
            Ok(())
        } else {
            CommandError::from_exec_error(
                exec_error,
                Output {
                    stdout: vec![],
                    stderr: vec![],
                    status,
                },
            )
            .into_report()
        }
    }

    fn display(&self) -> String {
        let (program, args) = get_program_and_args(self);
        format!("{program} {args}")
    }
}

/// A command output type.
pub trait OutputLike {
    /// The command's exit status.
    fn status(&self) -> std::process::ExitStatus;
    /// The command's stdout, decoded to UTF-8 on a best-effort basis.
    fn stdout(&self) -> Cow<'_, str>;
    /// The command's stderr, decoded to UTF-8 on a best-effort basis.
    fn stderr(&self) -> Cow<'_, str>;
}

impl OutputLike for Output {
    fn status(&self) -> std::process::ExitStatus {
        self.status
    }

    fn stdout(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(&self.stdout)
    }

    fn stderr(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(&self.stderr)
    }
}

impl OutputLike for Utf8Output {
    fn status(&self) -> std::process::ExitStatus {
        self.status
    }

    fn stdout(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.stdout)
    }

    fn stderr(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.stderr)
    }
}

/// An error from failing to execute a command. Produced by [`CommandExt`].
///
/// This is a command that fails to start, rather than a command that exits with a non-zero status
/// or similar, like [`CommandError`].
#[derive(Debug, Clone)]
pub struct CommandExecError {
    /// The executed program, shell quoted.
    program: String,
    /// The executed arguments. These are formatted into a single string and shell quoted, which is
    /// a little clumsy, but we only need these for the error message right now.
    args: String,
}

impl From<&Command> for CommandExecError {
    fn from(value: &Command) -> Self {
        let (program, args) = get_program_and_args(value);
        Self { program, args }
    }
}

impl Display for CommandExecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Failed to execute `{} {}`", self.program, self.args)
    }
}

impl Error for CommandExecError {}

/// An error from a failed command. Produced by [`CommandExt`].
#[derive(Debug)]
pub struct CommandError<O = Utf8Output> {
    /// The executed program, shell quoted.
    program: String,
    /// The executed arguments. These are formatted into a single string and shell quoted, which is
    /// a little clumsy, but we only need these for the error message right now.
    args: String,
    /// The program's output and exit code.
    pub output: O,
}

impl<O> Display for CommandError<O>
where
    O: OutputLike,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} failed: {}", self.program, self.output.status())
    }
}

impl<O> Error for CommandError<O> where O: Debug + OutputLike {}

impl<O> CommandError<O>
where
    O: Debug + OutputLike + Send + Sync + 'static,
{
    fn from_exec_error(exec_error: CommandExecError, output: O) -> Self {
        Self {
            program: exec_error.program,
            args: exec_error.args,
            output,
        }
    }

    fn into_report<T>(self) -> eyre::Result<T> {
        let command_failed = format!("Command failed: `{} {}`", self.program, self.args);

        let stdout = self.output.stdout();
        let stdout = stdout.trim();
        let stdout = if stdout.is_empty() {
            None
        } else {
            Some(stdout.to_owned().header("Stdout:"))
        };

        let stderr = self.output.stderr();
        let stderr = stderr.trim();
        let stderr = if stderr.is_empty() {
            None
        } else {
            Some(stderr.to_owned().header("Stderr:"))
        };

        let mut ret = Err(self).wrap_err(command_failed);

        if let Some(stdout) = stdout {
            ret = ret.with_section(|| stdout);
        }
        if let Some(stderr) = stderr {
            ret = ret.with_section(|| stderr);
        }

        tracing::debug!("Command failed: {:?}", ret.as_ref().map(|_| ()));
        ret
    }

    fn into_report_wrapped<T, E>(self, user_err: Option<E>) -> eyre::Result<T>
    where
        E: Display,
    {
        let report = self.into_report();
        match user_err {
            Some(user_err) => report.wrap_err(format!("{user_err}")),
            None => report,
        }
    }
}

fn get_program_and_args(cmd: &Command) -> (String, String) {
    // We're not doing anything weird with commands that are invalid UTF-8 so this is fine.
    let program = shell_words::quote(&cmd.get_program().to_string_lossy()).into_owned();
    let args = cmd
        .get_args()
        .map(|arg| shell_words::quote(&arg.to_string_lossy()).into_owned())
        .collect::<Vec<_>>()
        .join(" ");
    (program, args)
}

fn get_output_as<O>(cmd: &mut Command) -> eyre::Result<(O, CommandExecError)>
where
    O: TryFrom<Output>,
    O: Debug + OutputLike + Send + Sync + 'static,
    <O as TryFrom<Output>>::Error: Into<eyre::Report>,
{
    cmd.log_and_confirm()?;
    let exec_error = CommandExecError::from(&*cmd);
    #[allow(clippy::disallowed_methods)]
    let output = cmd
        .output()
        .wrap_err_with(|| exec_error.clone())
        .with_suggestion(|| {
            format!(
                "Is `{}` installed and present in your `$PATH`?",
                exec_error.program
            )
        })?
        .try_into()
        .map_err(|err: <O as TryFrom<Output>>::Error| err.into())?;
    Ok((output, exec_error))
}
