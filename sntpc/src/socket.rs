macro_rules! cfg_socket_impl {
    ($l:literal, { $($item:item)* }) => {
        $(
            #[cfg(feature = $l)]
            $item
        )*
    };
}

cfg_socket_impl!("std-socket", {
    mod std;
});
cfg_socket_impl!("embassy-socket", {
    mod embassy;
});
cfg_socket_impl!("tokio-socket", {
    mod tokio;
});
