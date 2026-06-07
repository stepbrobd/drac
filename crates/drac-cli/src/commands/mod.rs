use clap::Subcommand;

pub type CmdResult = Result<(), Box<dyn std::error::Error>>;

macro_rules! commands {
    ($($(#[$meta:meta])* $variant:ident => $ty:ty),+ $(,)?) => {
        #[derive(Subcommand, Debug)]
        pub enum Command {
            $($(#[$meta])* $variant($ty)),+
        }

        impl Command {
            pub fn run(self) -> CmdResult {
                match self {
                    $(Command::$variant(cmd) => cmd.run()),+
                }
            }
        }
    };
}

pub mod build_info;
pub mod version;

commands! {
    BuildInfo => build_info::BuildInfoCmd,
    Version => version::VersionCmd,
}
