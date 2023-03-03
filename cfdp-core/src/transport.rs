use std::{
    collections::HashMap,
    io::{Error as IoError, ErrorKind},
    net::{SocketAddr, ToSocketAddrs, UdpSocket},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

use crossbeam_channel::{Receiver, Sender};
use log::error;

use crate::pdu::{PDUEncode, VariableID, PDU};

/// Transports are designed to run in a thread in the background
/// inside a [Daemon](crate::daemon::Daemon) process
pub trait PDUTransport {
    /// Verify underyling communication method is ready.
    fn is_ready(&self) -> bool;

    /// Send input PDU to the remote
    /// The implementation must have a method to lookup an Entity's address from the ID
    fn request(&mut self, destination: VariableID, pdu: PDU) -> Result<(), IoError>;

    /// Provides logic for listening for incoming PDUs and sending any outbound PDUs

    /// A transport implementation will send any received messages through the
    /// [Sender] channel to the [Daemon](crate::daemon::Daemon).
    /// The [Receiver] channel is used to recv PDUs from the Daemon and send them to their respective remote Entity.
    /// The [Daemon](crate::daemon::Daemon) is responsible for receiving messages and distribute them to each
    /// transaction [Send](crate::transaction::SendTransaction) or [Recv](crate::transaction::RecvTransaction)
    /// The signal is used to indicate a shutdown operation was requested.
    fn pdu_handler(
        &mut self,
        signal: Arc<AtomicBool>,
        sender: Sender<PDU>,
        recv: Receiver<(VariableID, PDU)>,
    ) -> Result<(), IoError>;
}

/// A wrapper struct around a [UdpSocket] and a Mapping from
/// EntityIDs to [SocketAddr] instances.
pub struct UdpTransport {
    socket: UdpSocket,
    entity_map: HashMap<VariableID, SocketAddr>,
}
impl UdpTransport {
    pub fn new<T: ToSocketAddrs>(
        addr: T,
        entity_map: HashMap<VariableID, SocketAddr>,
    ) -> Result<Self, IoError> {
        let socket = UdpSocket::bind(addr)?;
        socket.set_read_timeout(Some(Duration::from_secs(1)))?;
        socket.set_write_timeout(Some(Duration::from_secs(1)))?;
        socket.set_nonblocking(true)?;
        Ok(Self { socket, entity_map })
    }
}
impl TryFrom<(UdpSocket, HashMap<VariableID, SocketAddr>)> for UdpTransport {
    type Error = IoError;
    fn try_from(inputs: (UdpSocket, HashMap<VariableID, SocketAddr>)) -> Result<Self, Self::Error> {
        let me = Self {
            socket: inputs.0,
            entity_map: inputs.1,
        };
        me.socket.set_read_timeout(Some(Duration::from_secs(1)))?;
        me.socket.set_write_timeout(Some(Duration::from_secs(1)))?;
        me.socket.set_nonblocking(true)?;
        Ok(me)
    }
}
impl PDUTransport for UdpTransport {
    fn is_ready(&self) -> bool {
        self.socket.local_addr().is_ok()
    }

    fn request(&mut self, destination: VariableID, pdu: PDU) -> Result<(), IoError> {
        self.entity_map
            .get(&destination)
            .ok_or_else(|| IoError::from(ErrorKind::AddrNotAvailable))
            .and_then(|addr| {
                self.socket
                    .send_to(pdu.encode().as_slice(), addr)
                    .map(|_n| ())
            })
    }

    fn pdu_handler(
        &mut self,
        signal: Arc<AtomicBool>,
        sender: Sender<PDU>,
        recv: Receiver<(VariableID, PDU)>,
    ) -> Result<(), IoError> {
        // this buffer will be 511 KiB, should be sufficiently small;
        let mut buffer = vec![0_u8; u16::MAX as usize];
        while !signal.load(Ordering::Relaxed) {
            match self.socket.recv_from(&mut buffer) {
                Ok(_n) => match PDU::decode(&mut buffer.as_slice()) {
                    Ok(pdu) => {
                        match sender.send(pdu) {
                            Ok(()) => {}
                            Err(error) => {
                                error!("Transport found disconnect sending channel: {}", error);
                                return Err(IoError::from(ErrorKind::ConnectionAborted));
                            }
                        };
                    }
                    Err(error) => {
                        error!("Error decoding PDU: {}", error);
                        // might need to stop depending on the error.
                        // some are recoverable though
                    }
                },
                Err(ref e)
                    if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut =>
                {
                    // continue to trying to send
                }
                Err(e) => {
                    error!("encountered IO error: {e}");
                    return Err(e);
                }
            };
            match recv.try_recv() {
                Ok((entity, pdu)) => self.request(entity, pdu)?,
                Err(crossbeam_channel::TryRecvError::Empty) => {
                    // nothing to do here
                }
                Err(err @ crossbeam_channel::TryRecvError::Disconnected) => {
                    error!("Transport found disconnected channel: {}", err);
                    return Err(IoError::from(ErrorKind::ConnectionAborted));
                }
            };
            thread::sleep(Duration::from_micros(500))
        }
        Ok(())
    }
}
