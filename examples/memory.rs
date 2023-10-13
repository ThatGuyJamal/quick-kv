use quick_kv::prelude::*;

fn main() -> anyhow::Result<()>
{
    let mut client = QuickMemoryClient::<String>::new(ClientConfig::default());

    client.set("hello", "world".to_string())?;

    let value = client.get("hello")?;

    println!("Value: {:?}", value.unwrap());

    client.delete("hello").unwrap();

    client.set("hello", "world2".to_string())?;

    let value = client.get("hello")?;

    println!("Value: {:?}", value.unwrap());

    client.update("hello", "world3".to_string(), None)?;

    let value = client.get("hello")?;

    println!("Value: {:?}", value.unwrap());

    Ok(())
}
