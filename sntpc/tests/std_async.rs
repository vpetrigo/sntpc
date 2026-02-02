#[cfg(all(test, feature = "std", feature = "sync"))]
mod sntpc_async_tests {
    use miniloop::executor::Executor;
    use sntpc::{Error, NtpContext, NtpUdpSocket, Result, StdTimestampGen, get_time};
    use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};

    const EXECUTOR_NUMBER_OF_TASKS: usize = 1;

    struct UdpSocketWrapper {
        socket: UdpSocket,
    }

    impl UdpSocketWrapper {
        #[must_use]
        fn new(socket: UdpSocket) -> Self {
            Self { socket }
        }
    }

    impl From<UdpSocket> for UdpSocketWrapper {
        fn from(socket: UdpSocket) -> Self {
            UdpSocketWrapper::new(socket)
        }
    }

    impl NtpUdpSocket for UdpSocketWrapper {
        async fn send_to(&self, buf: &[u8], addr: SocketAddr) -> Result<usize> {
            match self.socket.send_to(buf, addr) {
                Ok(usize) => Ok(usize),
                Err(_) => Err(Error::Network),
            }
        }

        async fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
            match self.socket.recv_from(buf) {
                Ok((size, addr)) => Ok((size, addr)),
                Err(_) => Err(Error::Network),
            }
        }
    }

    #[test]
    fn test_ntp_async_request_sntpv4_supported() {
        let context = NtpContext::new(StdTimestampGen::default());
        let pools = [
            "pool.ntp.org:123",
            "time.google.com:123",
            "time.apple.com:123",
            "time.cloudflare.com:123",
            "time.facebook.com:123",
        ];

        for pool in &pools {
            let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
            socket
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .expect("Unable to set up socket timeout");
            let socket = UdpSocketWrapper::from(socket);

            for address in pool.to_socket_addrs().unwrap().filter(SocketAddr::is_ipv4) {
                let result = Executor::<EXECUTOR_NUMBER_OF_TASKS>::new().block_on(get_time(address, &socket, context));

                assert!(result.is_ok(), "{pool} is bad - {:?}", result.unwrap_err());
                assert_ne!(result.unwrap().seconds, 0);
            }
        }
    }

    #[test]
    fn test_ntp_async_request_sntpv3_not_supported() {
        let context = NtpContext::new(StdTimestampGen::default());

        let pools = ["time.nist.gov:123", "time.windows.com:123"];

        for pool in &pools {
            let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
            socket
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .expect("Unable to set up socket timeout");
            let socket = UdpSocketWrapper::from(socket);

            for address in pool.to_socket_addrs().unwrap().filter(SocketAddr::is_ipv4) {
                let result = Executor::<EXECUTOR_NUMBER_OF_TASKS>::new().block_on(get_time(address, &socket, context));
                assert!(result.is_err(), "{pool} is ok");
                assert_eq!(result.unwrap_err(), Error::IncorrectResponseVersion);
            }
        }
    }
}
