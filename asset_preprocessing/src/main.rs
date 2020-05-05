pub mod gltf;
use log::error;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "example", about = "An example of StructOpt usage.")]
struct Opt {
    /// Activate debug mode
    // short and long flags (-d, --debug) will be deduced from the field's name
    #[structopt(short, long)]
    debug: bool,

    /// Input file
    #[structopt(parse(from_os_str))]
    input: PathBuf,

    /// Output file, stdout if not present
    #[structopt(parse(from_os_str))]
    asset_path: PathBuf,
}

fn main() {
    let opt = Opt::from_args();
    if opt.debug {
        std::env::set_var("RUST_LOG", "asset_preprocessing=DEBUG");
    }
    pretty_env_logger::init();

    if !opt.asset_path.is_dir() {
        error!("asset_path should point to a directory.");
        return;
    }

    if let Err(e) = gltf::import_gltf(opt.input, opt.asset_path) {
        error!("{:?}", e);
    }
}
