use colored::*;
use std::process::{Command, Stdio};
use common::get_kari_dir;

/// Handle certificate management commands
pub fn handle_certificate_command() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() < 3 {
        print_certificate_help();
        return Ok(());
    }
    
    match args[2].as_str() {
        "generate" => generate_certificates(),
        "status" => check_certificate_status(),
        "help" => {
            print_certificate_help();
            Ok(())
        },
        _ => {
            println!("{}", "Unknown certificate command".red());
            print_certificate_help();
            Err("Invalid command".to_string())
        }
    }
}

/// Display help for certificate commands
fn print_certificate_help() {
    println!("{}", "CERTIFICATE MANAGEMENT COMMANDS:".bright_yellow().bold());
    println!();
    println!("  {} {}", "kari certificate generate".green().bold(), "Generate self-signed TLS certificates".bright_white());
    println!("  {} {}", "kari certificate status".green().bold(), "Check TLS certificate status".bright_white());
    println!("  {} {}", "kari certificate help".green().bold(), "Display this help message".bright_white());
    println!();
    println!("{}", "TLS CONFIGURATION:".bright_yellow().bold());
    println!("  TLS certificates are used to secure network connections between nodes");
    println!("  Certificates are stored in the ~/.kari/certs directory");
    println!("  For production use, consider obtaining certificates from a trusted CA");
}

/// Generate self-signed certificates
fn generate_certificates() -> Result<(), String> {
    let kari_dir = get_kari_dir();
    let certs_dir = kari_dir.join("certs");
    
    // Create certificates directory if it doesn't exist
    if !certs_dir.exists() {
        std::fs::create_dir_all(&certs_dir)
            .map_err(|e| format!("Failed to create certificates directory: {}", e))?;
    }
    
    let cert_path = certs_dir.join("node.crt");
    let key_path = certs_dir.join("node.key");
    
    // Check if certificates already exist
    if cert_path.exists() && key_path.exists() {
        println!("{}", "Certificates already exist:".yellow());
        println!("  Certificate: {}", cert_path.display());
        println!("  Key: {}", key_path.display());
        println!();
        println!("To regenerate certificates, delete the existing files first.");
        return Ok(());
    }
    
    println!("{}", "Generating self-signed certificates...".green());
    
    // Check if OpenSSL is available
    match Command::new("openssl").arg("version").stdout(Stdio::null()).status() {
        Ok(_) => {},
        Err(_) => {
            println!("{}", "OpenSSL not found!".red());
            println!("Please install OpenSSL and try again.");
            println!("  - On Windows: Install OpenSSL from https://slproweb.com/products/Win32OpenSSL.html");
            println!("  - On macOS: Run 'brew install openssl'");
            println!("  - On Linux: Run 'sudo apt-get install openssl' or equivalent");
            return Err("OpenSSL not found".to_string());
        }
    }
    
    // Generate a private key and self-signed certificate
    let _subject = "/C=US/ST=California/L=San Francisco/O=Kanari Network/OU=Blockchain/CN=localhost";
    
    // Get the local IP for SAN
    let local_ip = match panorama::node::get_local_ip() {
        Some(ip) => ip,
        None => "127.0.0.1".to_string()
    };
    
    // Create openssl.cnf with SAN extension
    let openssl_cnf = certs_dir.join("openssl.cnf");
    let config_content = format!(r#"
[req]
distinguished_name = req_distinguished_name
req_extensions = v3_req
prompt = no

[req_distinguished_name]
C = US
ST = California
L = San Francisco
O = Kanari Network
OU = Blockchain
CN = localhost

[v3_req]
keyUsage = keyEncipherment, dataEncipherment
extendedKeyUsage = serverAuth
subjectAltName = @alt_names

[alt_names]
DNS.1 = localhost
IP.1 = 127.0.0.1
IP.2 = {}
"#, local_ip);

    std::fs::write(&openssl_cnf, config_content)
        .map_err(|e| format!("Failed to create OpenSSL config: {}", e))?;
    
    // Generate private key and certificate
    let status = Command::new("openssl")
        .args(&[
            "req", "-x509", 
            "-newkey", "rsa:4096", 
            "-keyout", key_path.to_str().unwrap(),
            "-out", cert_path.to_str().unwrap(),
            "-days", "3650",  // 10 years validity
            "-nodes",  // No password protection
            "-config", openssl_cnf.to_str().unwrap()
        ])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|e| format!("Failed to execute OpenSSL: {}", e))?;
    
    if !status.success() {
        return Err("Certificate generation failed".to_string());
    }
    
    // Remove the config file
    let _ = std::fs::remove_file(openssl_cnf);
    
    println!("{}", "Certificates generated successfully:".green());
    println!("  Certificate: {}", cert_path.display());
    println!("  Key: {}", key_path.display());
    println!();
    println!("To enable TLS, edit your ~/.kari/config.yaml file and set use_tls: true");
    println!("For direct HTTPS support, configure a reverse proxy like Nginx or Caddy");
    
    Ok(())
}

/// Check certificate status
fn check_certificate_status() -> Result<(), String> {
    let kari_dir = get_kari_dir();
    let certs_dir = kari_dir.join("certs");
    let cert_path = certs_dir.join("node.crt");
    let key_path = certs_dir.join("node.key");
    
    println!("{}", "Certificate Status:".bright_yellow());
    println!();
    
    println!("Directory: {}", certs_dir.display());
    
    // Check if certificate exists
    if cert_path.exists() {
        println!("Certificate: {} {}", "✓".green(), cert_path.display());
        
        // Check certificate details
        let cert_info = Command::new("openssl")
            .args(&["x509", "-in", cert_path.to_str().unwrap(), "-text", "-noout"])
            .output()
            .map_err(|e| format!("Failed to execute OpenSSL: {}", e))?;
        
        if cert_info.status.success() {
            let output = String::from_utf8_lossy(&cert_info.stdout);
            
            // Extract validity period
            if let Some(start_idx) = output.find("Not Before:") {
                if let Some(end_idx) = output[start_idx..].find("\n") {
                    println!("  {}", &output[start_idx..start_idx+end_idx]);
                }
            }
            
            if let Some(start_idx) = output.find("Not After :") {
                if let Some(end_idx) = output[start_idx..].find("\n") {
                    println!("  {}", &output[start_idx..start_idx+end_idx]);
                }
            }
            
            // Extract subject
            if let Some(start_idx) = output.find("Subject:") {
                if let Some(end_idx) = output[start_idx..].find("\n") {
                    println!("  {}", &output[start_idx..start_idx+end_idx]);
                }
            }
        }
    } else {
        println!("Certificate: {} {}", "✗".red(), "Not found");
    }
    
    // Check if key exists
    if key_path.exists() {
        println!("Private Key: {} {}", "✓".green(), key_path.display());
    } else {
        println!("Private Key: {} {}", "✗".red(), "Not found");
    }
    
    // Check if TLS is enabled in config
    let config_path = kari_dir.join("config.yaml");
    let tls_enabled = if config_path.exists() {
        match std::fs::read_to_string(&config_path) {
            Ok(content) => content.contains("use_tls: true"),
            Err(_) => false,
        }
    } else {
        false
    };
    
    println!("TLS Enabled: {}", if tls_enabled { "Yes".green() } else { "No".yellow() });
    
    if cert_path.exists() && key_path.exists() && !tls_enabled {
        println!();
        println!("{}", "Certificates exist but TLS is not enabled.".yellow());
        println!("To enable TLS, edit ~/.kari/config.yaml and set use_tls: true");
    } else if (!cert_path.exists() || !key_path.exists()) && tls_enabled {
        println!();
        println!("{}", "TLS is enabled but certificates are missing!".red());
        println!("Generate certificates with 'kari certificate generate'");
    }
    
    Ok(())
}
