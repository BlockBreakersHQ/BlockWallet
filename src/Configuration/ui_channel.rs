use glib::ControlFlow;

pub fn unbounded<T>() -> (async_channel::Sender<T>, async_channel::Receiver<T>) {
    async_channel::unbounded()
}

pub fn attach<T: 'static>(
    receiver: async_channel::Receiver<T>,
    mut callback: impl FnMut(T) -> ControlFlow + 'static,
) {
    glib::spawn_future_local(async move {
        while let Ok(item) = receiver.recv().await {
            if callback(item) == ControlFlow::Break {
                break;
            }
        }
    });
}
