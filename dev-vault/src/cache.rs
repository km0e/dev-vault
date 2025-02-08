use async_trait::async_trait;

#[async_trait]
pub trait Cache {
    async fn get(&self, hid: &str, path: &str) -> crate::Result<Option<(u64, u64)>>;
    async fn set(&self, hid: &str, path: &str, version: u64, modified: u64) -> crate::Result<()>;
}
