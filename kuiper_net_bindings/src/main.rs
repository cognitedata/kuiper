use interoptopus::util::NamespaceMappings;
use interoptopus::Interop;
use interoptopus_backend_csharp::overloads::DotNet;
use interoptopus_backend_csharp::{Config, Generator};

fn main() {
    let config = Config {
        dll_name: "kuiper_net".to_string(),
        namespace_mappings: NamespaceMappings::new("Cognite.Kuiper"),
        ..Config::default()
    };

    Generator::new(config, kuiper_net::ffi_inventory())
        .add_overload_writer(DotNet::new())
        .write_file("../KuiperNet/Kuiper.cs")
        .unwrap();
}
