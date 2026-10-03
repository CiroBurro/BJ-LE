use bjle_mesh::Peer;

#[tokio::main]
async fn main() -> bluer::Result<()> {
    let mut bucket = Peer::new().await?;

    bucket.start_listening().await?;
    match tokio::time::timeout(std::time::Duration::from_secs(180), bucket.get_data()).await {
        Ok(result) => result?,
        Err(_) => println!("Finished listening after 180 seconds"),
    }
    bucket.stop_listening().await?;
    Ok(())
}
