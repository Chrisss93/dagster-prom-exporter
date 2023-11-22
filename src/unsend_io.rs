//! Following examples/single_threaded.rs from the hyper crate (version 1)
//!
// use hyper::body::{Body, Bytes, Frame};
// use pin_project_lite::pin_project;
// use tokio::net::TcpStream;

// use std::{marker::PhantomData, pin::Pin, task::{Context, Poll}};
// pub struct UnsendIO {
//     _marker: PhantomData<*const ()>,
//     stream: TokioIo<TcpStream>,
// }

// impl From<TcpStream>for UnsendIO {
//     fn from(io: TcpStream) -> Self {
//         Self {
//             _marker: PhantomData,
//             stream: TokioIo { inner: io }
//         }
//     }
// }

// impl hyper::rt::Write for UnsendIO {
//     fn poll_write(
//         mut self: Pin<&mut Self>,
//         cx: &mut Context<'_>,
//         buf: &[u8],
//     ) -> Poll<Result<usize, std::io::Error>> {
//         Pin::new(&mut self.stream).poll_write(cx, buf)
//     }

//     fn poll_flush(
//         mut self: Pin<&mut Self>,
//         cx: &mut Context<'_>,
//     ) -> Poll<Result<(), std::io::Error>> {
//         Pin::new(&mut self.stream).poll_flush(cx)
//     }

//     fn poll_shutdown(
//         mut self: Pin<&mut Self>,
//         cx: &mut Context<'_>,
//     ) -> Poll<Result<(), std::io::Error>> {
//         Pin::new(&mut self.stream).poll_shutdown(cx)
//     }
// }

// impl hyper::rt::Read for UnsendIO {
//     fn poll_read(
//         mut self: Pin<&mut Self>,
//         cx: &mut Context<'_>,
//         buf: hyper::rt::ReadBufCursor<'_>,
//     ) -> Poll<std::io::Result<()>> {
//         Pin::new(&mut self.stream).poll_read(cx, buf)
//     }
// }

// pub struct UnsendBody {
//     // Our Body type is !Send and !Sync:
//     _marker: PhantomData<*const ()>,
//     data: Option<Bytes>,
// }

// impl<B> From<B> for UnsendBody where B: Into<Bytes> {
//     fn from(x: B) -> Self {
//         Self {
//             _marker: PhantomData,
//             data: Some(x.into()),
//         }
//     }
// }

// impl Body for UnsendBody {
//     type Data = Bytes;
//     type Error = hyper::Error;

//     fn poll_frame(
//         self: Pin<&mut Self>,
//         _: &mut Context<'_>,
//     ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
//         Poll::Ready(self.get_mut().data.take().map(|d| Ok(Frame::data(d))))
//     }
// }


// pin_project! {
//     #[derive(Debug)]
//     struct TokioIo<T> {
//         #[pin]
//         inner: T,
//     }
// }

// impl<T> hyper::rt::Read for TokioIo<T>
// where
//     T: tokio::io::AsyncRead,
// {
//     fn poll_read(
//         self: Pin<&mut Self>,
//         cx: &mut Context<'_>,
//         mut buf: hyper::rt::ReadBufCursor<'_>,
//     ) -> Poll<Result<(), std::io::Error>> {
//         let n = unsafe {
//             let mut tbuf = tokio::io::ReadBuf::uninit(buf.as_mut());
//             match tokio::io::AsyncRead::poll_read(self.project().inner, cx, &mut tbuf) {
//                 Poll::Ready(Ok(())) => tbuf.filled().len(),
//                 other => return other,
//             }
//         };

//         unsafe {
//             buf.advance(n);
//         }
//         Poll::Ready(Ok(()))
//     }
// }

// impl<T> hyper::rt::Write for TokioIo<T>
// where
//     T: tokio::io::AsyncWrite,
// {
//     fn poll_write(
//         self: Pin<&mut Self>,
//         cx: &mut Context<'_>,
//         buf: &[u8],
//     ) -> Poll<Result<usize, std::io::Error>> {
//         tokio::io::AsyncWrite::poll_write(self.project().inner, cx, buf)
//     }

//     fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
//         tokio::io::AsyncWrite::poll_flush(self.project().inner, cx)
//     }

//     fn poll_shutdown(
//         self: Pin<&mut Self>,
//         cx: &mut Context<'_>,
//     ) -> Poll<Result<(), std::io::Error>> {
//         tokio::io::AsyncWrite::poll_shutdown(self.project().inner, cx)
//     }

//     fn is_write_vectored(&self) -> bool {
//         tokio::io::AsyncWrite::is_write_vectored(&self.inner)
//     }

//     fn poll_write_vectored(
//         self: Pin<&mut Self>,
//         cx: &mut Context<'_>,
//         bufs: &[std::io::IoSlice<'_>],
//     ) -> Poll<Result<usize, std::io::Error>> {
//         tokio::io::AsyncWrite::poll_write_vectored(self.project().inner, cx, bufs)
//     }
// }

// impl<T> tokio::io::AsyncRead for TokioIo<T>
// where
//     T: hyper::rt::Read,
// {
//     fn poll_read(
//         self: Pin<&mut Self>,
//         cx: &mut Context<'_>,
//         tbuf: &mut tokio::io::ReadBuf<'_>,
//     ) -> Poll<Result<(), std::io::Error>> {
//         //let init = tbuf.initialized().len();
//         let filled = tbuf.filled().len();
//         let sub_filled = unsafe {
//             let mut buf = hyper::rt::ReadBuf::uninit(tbuf.unfilled_mut());

//             match hyper::rt::Read::poll_read(self.project().inner, cx, buf.unfilled()) {
//                 Poll::Ready(Ok(())) => buf.filled().len(),
//                 other => return other,
//             }
//         };

//         let n_filled = filled + sub_filled;
//         // At least sub_filled bytes had to have been initialized.
//         let n_init = sub_filled;
//         unsafe {
//             tbuf.assume_init(n_init);
//             tbuf.set_filled(n_filled);
//         }

//         Poll::Ready(Ok(()))
//     }
// }

// impl<T> tokio::io::AsyncWrite for TokioIo<T>
// where
//     T: hyper::rt::Write,
// {
//     fn poll_write(
//         self: Pin<&mut Self>,
//         cx: &mut Context<'_>,
//         buf: &[u8],
//     ) -> Poll<Result<usize, std::io::Error>> {
//         hyper::rt::Write::poll_write(self.project().inner, cx, buf)
//     }

//     fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
//         hyper::rt::Write::poll_flush(self.project().inner, cx)
//     }

//     fn poll_shutdown(
//         self: Pin<&mut Self>,
//         cx: &mut Context<'_>,
//     ) -> Poll<Result<(), std::io::Error>> {
//         hyper::rt::Write::poll_shutdown(self.project().inner, cx)
//     }

//     fn is_write_vectored(&self) -> bool {
//         hyper::rt::Write::is_write_vectored(&self.inner)
//     }

//     fn poll_write_vectored(
//         self: Pin<&mut Self>,
//         cx: &mut Context<'_>,
//         bufs: &[std::io::IoSlice<'_>],
//     ) -> Poll<Result<usize, std::io::Error>> {
//         hyper::rt::Write::poll_write_vectored(self.project().inner, cx, bufs)
//     }
// }
