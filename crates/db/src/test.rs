#[cfg(test)]
mod tests
{
    use std::time::Duration;

    use crate::*;

    #[tokio::test]
    async fn test_set_get()
    {
        let db = Db::new();

        // Set a value
        db.set("key1".to_string(), Bytes::from("value1"), None);

        // Get the value and check if it's correct
        assert_eq!(db.get("key1"), Some(Bytes::from("value1")));
    }

    #[tokio::test]
    async fn test_set_expire_get()
    {
        let db = Db::new();

        // Set a value with expiration
        db.set("key2".to_string(), Bytes::from("value2"), Some(Duration::from_secs(1)));

        // Wait for the expiration
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Get the value and check if it's expired
        assert_eq!(db.get("key2"), None);
    }

    #[tokio::test]
    async fn test_subscribe_publish()
    {
        let db = Db::new();

        // Subscribe to a channel
        let mut receiver = db.subscribe("channel1".to_string());

        // Publish a message to the channel
        db.publish("channel1", Bytes::from("message1"));

        // Receive the published message
        assert_eq!(receiver.recv().await.unwrap(), Bytes::from("message1"));
    }
}
