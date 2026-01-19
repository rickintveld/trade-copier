use anyhow::Result;
use tokio::net::TcpListener;
use log::{info, warn};

const MAX_BIND_RETRIES: u32 = 3;

/// Attempts to bind to the specified address, killing any process using the port if necessary
pub async fn bind_with_retry(address: &str) -> Result<TcpListener> {
    // Extract port from address for error messages
    let port = address.split(':').last().unwrap_or("unknown");
    
    for attempt in 1..=MAX_BIND_RETRIES {
        match TcpListener::bind(address).await {
            Ok(listener) => {
                info!("[PORT_UTILS] Successfully bound to {}", address);
                return Ok(listener);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
                warn!(
                    "[PORT_UTILS] Address {} is already in use (attempt {}/{}). Attempting to free the port...",
                    address, attempt, MAX_BIND_RETRIES
                );
                
                // Try to extract port number and kill process using it
                if let Ok(port_num) = port.parse::<u16>() {
                    if let Err(kill_err) = kill_process_on_port(port_num).await {
                        warn!("[PORT_UTILS] Failed to kill process on port {}: {}", port_num, kill_err);
                    }
                } else {
                    warn!("[PORT_UTILS] Could not parse port from address: {}", address);
                }
                
                // Wait a bit for the port to be released
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                
                if attempt == MAX_BIND_RETRIES {
                    return Err(anyhow::anyhow!(
                        "Failed to bind to {} after {} attempts. Port is still in use (os error {})",
                        address,
                        MAX_BIND_RETRIES,
                        e.raw_os_error().unwrap_or(0)
                    ));
                }
            }
            Err(e) => {
                return Err(e.into());
            }
        }
    }
    
    unreachable!()
}

/// Kills any process listening on the specified port
pub async fn kill_process_on_port(port: u16) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        
        // Use lsof to find the process using the port
        let output = Command::new("lsof")
            .args(["-ti", &format!(":{}", port)])
            .output()?;
        
        if output.status.success() {
            let pids = String::from_utf8_lossy(&output.stdout);
            for pid in pids.lines().filter(|line| !line.is_empty()) {
                info!("[PORT_UTILS] Killing process {} using port {}", pid, port);
                let _ = Command::new("kill")
                    .args(["-9", pid])
                    .status();
            }
            return Ok(());
        }
    }
    
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        
        // Use netstat to find the process and taskkill to terminate it
        let output = Command::new("cmd")
            .args(["/C", &format!("netstat -ano | findstr :{}", port)])
            .output()?;
        
        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            // Extract PID from netstat output (last column)
            if let Some(line) = output_str.lines().next() {
                if let Some(pid) = line.split_whitespace().last() {
                    info!("[PORT_UTILS] Killing process {} using port {}", pid, port);
                    let _ = Command::new("taskkill")
                        .args(["/F", "/PID", pid])
                        .status();
                    return Ok(());
                }
            }
        }
    }
    
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        use std::process::Command;
        
        // Use fuser on Linux
        let output = Command::new("fuser")
            .args([&format!("{}/tcp", port)])
            .output()?;
        
        if output.status.success() {
            let pids = String::from_utf8_lossy(&output.stdout);
            for pid in pids.split_whitespace().filter(|p| !p.is_empty()) {
                info!("[PORT_UTILS] Killing process {} using port {}", pid, port);
                let _ = Command::new("kill")
                    .args(["-9", pid])
                    .status();
            }
            return Ok(());
        }
    }
    
    Err(anyhow::anyhow!("Could not find or kill process on port {}", port))
}
