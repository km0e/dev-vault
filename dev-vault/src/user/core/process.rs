#[async_trait::async_trait]
// TODO:maybe better to use a trait for the sync method
pub trait PtyProcessImpl {
    async fn window_change(
        &self,
        width: u32,
        height: u32,
        pix_width: u32,
        pix_height: u32,
    ) -> crate::Result<()>;
    async fn wait(&mut self) -> crate::Result<i32>;
    async fn output(&mut self) -> crate::Result<Vec<u8>>;
    async fn sync(
        &mut self,
        reader: Box<dyn tokio::io::AsyncRead + Unpin + Send>,
        writer: Box<dyn tokio::io::AsyncWrite + Unpin + Send>,
    ) -> crate::Result<i32>;
}

pub type BoxedPtyProcess = Box<dyn PtyProcessImpl + Unpin + Send + Sync>;
