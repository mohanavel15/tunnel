use clap::{Parser, Subcommand, CommandFactory};

#[derive(Parser)]
#[command()]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Authenication
    Auth {
        token: String,
    },
    /// start TCP tunnel
    Tcp {
        port: u16,
    },
    /// start UDP tunnel
    Udp {
        port: u16,
    },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Auth { token }) => {
            println!("token {}", token);
        }
        Some(Commands::Tcp { port }) => {
            println!("forwarding tcp connections to {}", port);
        }
        Some(Commands::Udp { port }) => {
            println!("forwarding udp connections to {}", port);
        }
        None => Cli::command().print_help().unwrap_or_else(|_| println!("Unable print help. report issue"))
    }
}