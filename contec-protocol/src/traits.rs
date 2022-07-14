use core::pin::Pin;
use core::task::{Context, Poll};

/// Trait that must be implemented by the serial port that enables async communication
pub trait AsyncReadWrite {
    /// Device error
    #[cfg(feature = "std")]
    type Error: snafu::AsErrorSource;
    #[cfg(not(feature = "std"))]
    type Error;

    /// Attempt to read from the object into `buf`.
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize, Self::Error>>;

    /// Attempt to write bytes from `buf` into the object.
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, Self::Error>>;
}

#[cfg(feature = "std")]
impl<T> AsyncReadWrite for T
where
    T: futures::io::AsyncRead + futures::io::AsyncWrite + Unpin,
{
    type Error = futures::io::Error;

    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize, Self::Error>> {
        futures::io::AsyncRead::poll_read(self, cx, buf)
    }

    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, Self::Error>> {
        futures::io::AsyncWrite::poll_write(self, cx, buf)
    }
}
