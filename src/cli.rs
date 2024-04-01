use clap::Parser;

/// Accepted file types
#[derive(Debug, PartialEq)]
pub enum ArgType {
    Apk,
    Axml,
    Arsc
}

/// Basic CLI for the binary
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[clap(flatten)]
    /// Path to the file to parse. The file can be either an APK, an Android
    /// binary-XML file, or a resource.arsc file.
    target: Target
}

/// Argument group to represent any file that can be parsed by AXMLParser
#[derive(Debug, clap::Args)]
#[group(required = true, multiple = false)]
struct Target {
   /// Path to an APK
   #[arg(short, long)]
   apk: Option<String>,

   /// Path to an Android binary XML file
   #[arg(short, long)]
   xml: Option<String>,

   /// Path to resources.arsc file
   #[arg(short, long)]
   res: Option<String>,
}

pub fn parse_args() -> Args {
    Args::parse()
}

pub fn get_arg_type(args: &Args) -> ArgType {
    if args.target.apk.is_some() {
        return ArgType::Apk;
    }

    if args.target.xml.is_some() {
        return ArgType::Axml;
    }

    if args.target.res.is_some() {
        return ArgType::Arsc;
    }

    panic!("Will never happen");
}

pub fn get_arg_path(args: &Args) -> String {
    if args.target.apk.is_some() {
        return args.target.apk.as_ref().unwrap().clone();
    }

    else if args.target.xml.is_some() {
        return args.target.xml.as_ref().unwrap().clone();
    }

    else if args.target.res.is_some() {
        return args.target.res.as_ref().unwrap().clone();
    }

    panic!("Will never happen");
}
