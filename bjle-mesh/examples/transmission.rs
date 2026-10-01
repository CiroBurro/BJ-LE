use bjle_mesh::Peer;

#[tokio::main]
async fn main() -> bluer::Result<()> {
    let mut bucket = Peer::new().await?;

    bucket.start_listening().await?;
    bucket.get_data().await?;
    tokio::time::sleep(std::time::Duration::from_secs(180)).await;
    Ok(())
}
