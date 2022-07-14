use core::pin::Pin;
use core::task::Poll;

use futures::{future, ready, Future};

use crate::incoming_package::{IncomingPackage, IncomingStateMachine};
use crate::outgoing_package::{bytes_from_package, OutgoingPackage};
use crate::traits::AsyncReadWrite;
use crate::{Error, Result};

/// Represents a connection with a pulse oximeter.
///
/// Use the [PulseOximeter::send_package()] and [PulseOximeter::receive_package] methods to
/// communicate with the foobar device.
// pub struct PulseOximeter<T: AsyncRead + AsyncWrite + Unpin> {
pub struct PulseOximeter<T: AsyncReadWrite + Unpin> {
    port: T,
    incoming: IncomingStateMachine,
    outgoing: OutgoingStatus,
}

enum OutgoingStatus {
    None,
    Some {
        buffer: [u8; 9],
        already_sent: usize,
    },
}

impl<T: AsyncReadWrite + Unpin> PulseOximeter<T> {
    /// Create a pulse oximeter interface, using the given `port` for communication.
    pub fn new(port: T) -> Self {
        Self {
            port,
            incoming: IncomingStateMachine::None,
            outgoing: OutgoingStatus::None,
        }
    }

    /// Send a package to the device.
    ///
    /// Note that if a future returned by a previous call to this function was not polled until
    /// completion, the rest of the package of the previous call will be sent before the new
    /// package will be sent.
    #[allow(clippy::bool_assert_comparison)]
    pub fn send_package<P>(&mut self, package: P) -> impl Future<Output = Result<(), T::Error>> + '_
    where
        P: OutgoingPackage,
    {
        let buffer = bytes_from_package(package);

        // Determine whether the last send operation was completed
        let mut unfinished_send = matches!(self.outgoing, OutgoingStatus::Some { .. });

        future::poll_fn(move |cx| loop {
            match self.outgoing {
                // No ongoing send operation, so start next one
                OutgoingStatus::None => {
                    self.outgoing = OutgoingStatus::Some {
                        buffer,
                        already_sent: 0,
                    }
                }
                // Ongoing send operation
                OutgoingStatus::Some {
                    buffer,
                    ref mut already_sent,
                } => {
                    let slice = &buffer[*already_sent..9];
                    let bytes_written = ready!(Pin::new(&mut self.port).poll_write(cx, slice))?;
                    if bytes_written > slice.len() {
                        return Err(Error::DeviceWriteTooMuch {
                            requested: slice.len(),
                            reported: bytes_written,
                        })
                        .into();
                    }
                    *already_sent += bytes_written;
                    if *already_sent == 9 {
                        // Current send operation finished
                        self.outgoing = OutgoingStatus::None;
                        if unfinished_send {
                            // Old send operation is now finished, start sending actual data of
                            // this function call
                            unfinished_send = false;
                        } else {
                            // Send operation completed, return
                            return Poll::Ready(Ok(()));
                        }
                    }
                }
            }
        })
    }

    /// Receive the next package from the device.
    pub fn receive_package(
        &mut self,
    ) -> impl Future<Output = Result<IncomingPackage, T::Error>> + '_ {
        future::poll_fn(move |cx| {
            self.incoming.resume(|buf| Pin::new(&mut self.port).poll_read(cx, buf))
        })
    }
}
