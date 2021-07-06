use std::io;
#[cfg(windows)]
use winres::WindowsResource;

fn main() -> io::Result<()> {
    println!("cargo:rerun-if-changed=src");

    #[cfg(windows)]
    {
        WindowsResource::new()
            .set_icon("assets/dank.ico")
            .compile()?;
    }

    Ok(())
}
