use turso::Connection;

use crate::error::Result;
use crate::palace::layers;

pub async fn run(conn: &Connection, wing: Option<&str>) -> Result<()> {
    let text = layers::wake_up(conn, wing).await?;
    println!("{text}");
    Ok(())
}
