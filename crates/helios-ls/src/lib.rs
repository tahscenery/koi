mod connection;
mod protocol;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T, E = Error> = std::result::Result<T, E>;

pub fn start() {
    if let Err(error) = __start() {
        eprintln!("{}", error);
        std::process::exit(1);
    }
}

pub fn __start() -> Result<()> {
    let (_connection, threads) = connection::stdio();

    threads.join()?;
    Ok(())
}
