mod input;
mod result;
mod rush;

use crate::result::Result;
use crate::rush::Rush;

fn main() -> Result<()> {
    Rush::new().run()?;
    Ok(())
}
