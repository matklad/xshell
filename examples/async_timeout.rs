use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    #[cfg(feature = "async")]
    timeout_example().await;

    #[cfg(feature = "async")]
    no_timeout_example().await;

    Ok(())
}

#[cfg(feature = "async")]
async fn timeout_example() {
    use std::time::Duration;

    use anyhow::{anyhow, Context};
    use tokio::time::timeout;
    use xshell::{cmd, Shell};

    let sh = Shell::new().unwrap();
    let cmd = cmd!(sh, "sleep 5");
    let res = match timeout(Duration::from_secs(3), cmd.read_async()).await {
        Ok(result) => result.context("Run failed"),
        Err(e) => Err(anyhow!("Timeout: {e}")),
    };

    println!("Should timeout: {res:?}");
}

#[cfg(feature = "async")]
async fn no_timeout_example() {
    use std::time::Duration;

    use anyhow::{anyhow, Context};
    use tokio::time::timeout;
    use xshell::{cmd, Shell};

    let sh = Shell::new().unwrap();
    let cmd = cmd!(sh, "echo Hello");
    let res = match timeout(Duration::from_secs(3), cmd.read_async()).await {
        Ok(result) => result.context("Run failed"),
        Err(e) => Err(anyhow!("Timeout: {e}")),
    };

    println!("Should echo: {res:?}");
}
