use async_trait::async_trait;

#[async_trait]
pub trait Cache {
    async fn check_update(&self, uid: &str, path: &str, modified: u64) -> crate::Result<bool>;
    async fn set(&self, uid: &str, path: &str, modified: u64) -> crate::Result<()>;
}
