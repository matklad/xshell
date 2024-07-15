use std::future::Future;
use std::io;
use std::pin::Pin;
use std::process::Output;
use std::process::Stdio;

use tokio::io::AsyncWriteExt;

use crate::Cmd;
use crate::Error;
use crate::Result;

impl<'a> Cmd<'a> {
    // region:running
    /// Runs the command **asynchronously**.
    ///
    /// By default the command itself is echoed to stderr, its standard streams
    /// are inherited, and non-zero return code is considered an error. These
    /// behaviors can be overridden by using various builder methods of the [`Cmd`].
    pub fn run_async(&self) -> Pin<Box<dyn Future<Output = Result<()>> + '_>> {
        Box::pin(async move { self.output_impl_async(false, false).await.map(|_| ()) })
    }

    /// Run the command **asynchronously** and return its stdout as a string. Any trailing newline or carriage return will be trimmed.
    pub fn read_async(&self) -> Pin<Box<dyn Future<Output = Result<String>> + '_>> {
        Box::pin(async move { self.read_stream_async(false).await })
    }

    /// Run the command **asynchronously** and return its stderr as a string. Any trailing newline or carriage return will be trimmed.
    pub fn read_stderr_async(&self) -> Pin<Box<dyn Future<Output = Result<String>> + '_>> {
        Box::pin(async move { self.read_stream_async(true).await })
    }

    /// Run the command **asynchronously** and return its output.
    pub fn output_async(&self) -> Pin<Box<dyn Future<Output = Result<Output>> + '_>> {
        Box::pin(async move { self.output_impl_async(true, true).await })
    }

    async fn read_stream_async(&self, read_stderr: bool) -> Result<String> {
        let read_stdout = !read_stderr;
        let output = self.output_impl_async(read_stdout, read_stderr).await?;
        self.check_status(output.status)?;

        let stream = if read_stderr { output.stderr } else { output.stdout };
        let mut stream = String::from_utf8(stream).map_err(|err| Error::new_cmd_utf8(self, err))?;

        if stream.ends_with('\n') {
            stream.pop();
        }
        if stream.ends_with('\r') {
            stream.pop();
        }

        Ok(stream)
    }

    async fn output_impl_async(
        &self,
        read_stdout: bool,
        read_stderr: bool,
    ) -> Result<Output, Error> {
        let mut command = tokio::process::Command::from(self.to_command());

        if !self.data.ignore_stdout {
            command.stdout(if read_stdout { Stdio::piped() } else { Stdio::inherit() });
        }
        if !self.data.ignore_stderr {
            command.stderr(if read_stderr { Stdio::piped() } else { Stdio::inherit() });
        }

        command.stdin(match &self.data.stdin_contents {
            Some(_) => Stdio::piped(),
            None => Stdio::null(),
        });

        let mut child = command.spawn().map_err(|err| {
            if matches!(err.kind(), io::ErrorKind::NotFound) {
                let cwd = self.shell.cwd.borrow();
                if let Err(err) = cwd.metadata() {
                    return Error::new_current_dir(err, Some(cwd.clone()));
                }
            }
            Error::new_cmd_io(self, err)
        })?;

        if let Some(stdin_contents) = self.data.stdin_contents.clone() {
            let mut stdin = child.stdin.take().unwrap();
            tokio::spawn(async move {
                stdin.write_all(&stdin_contents).await?;
                stdin.flush().await
            });
        }

        let output = child.wait_with_output().await.map_err(|err| Error::new_cmd_io(self, err))?;
        self.check_status(output.status)?;
        Ok(output)
    }
}
