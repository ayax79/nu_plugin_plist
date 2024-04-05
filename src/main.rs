use nu_plugin::{serve_plugin, MsgPackSerializer};

use nu_plist::NuPlistPlugin;

mod nu_plist;

fn main() {
    serve_plugin(&NuPlistPlugin, MsgPackSerializer);
}
