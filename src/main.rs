use nu_plugin::{serve_plugin, MsgPackSerializer};

use nu_plist::NuPlist;

mod nu_plist;

fn main() {
    serve_plugin(&mut NuPlist {}, MsgPackSerializer {});
}
