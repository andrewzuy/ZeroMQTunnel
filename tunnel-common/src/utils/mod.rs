pub fn uuid_v4() -> Result<uuid::Uuid, anyhow::Error> { Ok(uuid::Uuid::new_v4()) }
