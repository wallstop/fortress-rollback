#[cfg(all(test, target_arch = "wasm32", target_os = "unknown"))]
mod tests {
    use fortress_rollback::{ChaosConfig, ChaosSocket, Message, NonBlockingSocket};
    use wasm_bindgen_test::wasm_bindgen_test;

    struct EmptySocket;

    impl NonBlockingSocket<u8> for EmptySocket {
        fn send_to(&mut self, _msg: &Message, _addr: &u8) {}

        fn receive_all_messages(&mut self) -> Vec<(u8, Message)> {
            Vec::new()
        }
    }

    #[wasm_bindgen_test]
    fn default_clock_receive_does_not_panic_in_browser() {
        let config = ChaosConfig::builder().seed(42).build();
        let mut socket = ChaosSocket::new(EmptySocket, config);

        // Checking for ready packets reads the default clock even when the queue is empty.
        let messages = socket.receive_all_messages();

        assert!(messages.is_empty(), "empty inner socket returned messages");
    }
}
