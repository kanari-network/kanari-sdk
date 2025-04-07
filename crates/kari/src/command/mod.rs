use std::fs;
use std::path::Path;
use std::process::Command;
use colored::Colorize;

/// Handle certificate-related commands
pub fn handle_certificate_command(args: &[String]) {
    if args.is_empty() {
        display_certificate_help();
        return;
    }

    match args[0].as_str() {
        "generate" => generate_certificate(),
        "info" => display_certificate_info(),
        _ => {
            println!("{}", "ERROR: Invalid certificate command".red().bold());
            display_certificate_help();
        }
    }
}

/// Display help for certificate commands
fn display_certificate_help() {
    println!("{}", "USAGE:".bright_yellow().bold());
    println!("  kari certificate <command>\n");
    println!("{}", "COMMANDS:".bright_yellow().bold());
    println!("  {}  {}", "generate".green().bold(), "Generate a new TLS certificate".bright_white());
    println!("  {}  {}", "info".green().bold(), "Display information about the current certificate".bright_white());
}

/// Generate a new TLS certificate
fn generate_certificate() {
    let cert_path = "./certs/node.crt";
    let key_path = "./certs/node.key";

    // Ensure the certs directory exists
    let certs_dir = Path::new("./certs");
    if !certs_dir.exists() {
        if let Err(e) = fs::create_dir(certs_dir) {
            println!("{}: {}", "Failed to create certs directory".red(), e);
            return;
        }
    }

    // Generate certificate using OpenSSL
    let openssl_command = format!(
        "openssl req -x509 -newkey rsa:2048 -keyout {} -out {} -days 365 -nodes -subj \"/CN=localhost\"",
        key_path, cert_path
    );

    match Command::new("sh")
        .arg("-c")
        .arg(openssl_command)
        .output()
    {
        Ok(output) => {
            if output.status.success() {
                println!("{}", "Certificate generated successfully!".green());
                println!("  Certificate: {}", cert_path);
                println!("  Private Key: {}", key_path);
            } else {
                println!("{}: {}", "Failed to generate certificate".red(), String::from_utf8_lossy(&output.stderr));
            }
        }
        Err(e) => {
            println!("{}: {}", "Error executing OpenSSL command".red(), e);
        }
    }
}

/// Display information about the current certificate
fn display_certificate_info() {
    let cert_path = "./certs/node.crt";

    if !Path::new(cert_path).exists() {
        println!("{}", "No certificate found!".red());
        println!("Please generate a certificate first using:");
        println!("{}", "kari certificate generate".green());
        return;
    }

    // Display certificate information using OpenSSL
    let openssl_command = format!("openssl x509 -in {} -text -noout", cert_path);

    match Command::new("sh")
        .arg("-c")
        .arg(openssl_command)
        .output()
    {
        Ok(output) => {
            if output.status.success() {
                println!("{}", "Certificate Information:".bright_yellow());
                println!("{}", String::from_utf8_lossy(&output.stdout));
            } else {
                println!("{}: {}", "Failed to read certificate".red(), String::from_utf8_lossy(&output.stderr));
            }
        }
        Err(e) => {
            println!("{}: {}", "Error executing OpenSSL command".red(), e);
        }
    }
}