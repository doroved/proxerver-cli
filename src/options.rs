use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[clap(author, version, about, long_about = None)]
pub struct Opt {
    #[clap(
        long,
        value_name = "PATH",
        help = "Path to the configuration file. Default: '/config.toml'"
    )]
    pub config: Option<String>,
}
